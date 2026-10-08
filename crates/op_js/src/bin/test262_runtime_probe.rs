//! Bounded Test262 classic-script runtime subset probe.
//! The manifest, pinned upstream tree and explicit skip counts define the metric;
//! this is not a full Test262 harness, strict-mode test, or browser Web API test.
use op_js::{JsError, JsErrorKind, JsRuntime, parse_script};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BOOTSTRAP: &str = r#"
function Test262Error(message) {
    this.name = "Test262Error";
    this.message = message;
    if (message === undefined) this.message = "Test262 assertion failed";
}
function $ERROR(message) { throw new Test262Error(message); }
function assert(condition, message) {
    if (!condition) throw new Test262Error(message);
}
assert.sameValue = function(actual, expected, message) {
    if (actual === expected) {
        if (actual !== 0 || 1 / actual === 1 / expected) return;
    } else if (actual !== actual && expected !== expected) return;
    throw new Test262Error(message);
};
assert.notSameValue = function(actual, expected, message) {
    if (actual === expected) {
        if (actual !== 0 || 1 / actual === 1 / expected)
            throw new Test262Error(message);
    } else if (actual !== actual && expected !== expected) {
        throw new Test262Error(message);
    }
};
assert.throws = function(expected, thunk, message) {
    var threw = false;
    try { thunk(); }
    catch (error) {
        threw = true;
        if (error.name !== expected.name)
            throw new Test262Error(message);
    }
    if (!threw) throw new Test262Error(message);
};
"#;

#[derive(Default, Debug)]
struct Counts {
    listed: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    skipped_modules: usize,
    skipped_async: usize,
    skipped_strict: usize,
    skipped_parse_negative: usize,
    skipped_includes: usize,
    skipped_unsupported_flags: usize,
    positive: usize,
    negative_runtime: usize,
}
impl Counts {
    fn attempted(&self) -> usize {
        self.passed + self.failed
    }
    fn percent(&self) -> f64 {
        if self.attempted() == 0 {
            0.0
        } else {
            (self.passed as f64) * 100.0 / (self.attempted() as f64)
        }
    }
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Pass,
    Fail(String),
    Skip(&'static str),
}

/// Read the simple scalar/flow-list/indented-list YAML fields used by
/// Test262. Never silently drop an unsupported includes/flags list.
fn words(meta: &str, key: &str) -> Vec<String> {
    let prefix = format!("{key}:");
    let mut found = false;
    let mut field = String::new();
    for line in meta.lines() {
        if !found {
            if let Some(value) = line.strip_prefix(&prefix) {
                found = true;
                field.push_str(value);
                field.push(' ');
            }
            continue;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !line.starts_with([' ', '\t'])
            && !trimmed.starts_with(['[', ']', '-'])
        {
            break;
        }
        field.push_str(trimmed);
        field.push(' ');
    }
    if !found {
        return Vec::new();
    }
    field
        .split([',', ' ', '[', ']', '\t', '\n'])
        .map(|value| value.trim_matches(&['-', '\'', '"'][..]))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn negative(meta: &str) -> Option<(&str, &str)> {
    let mut active = false;
    let mut phase = "";
    let mut error_type = "";
    for line in meta.lines() {
        if line.trim() == "negative:" {
            active = true;
            continue;
        }
        if !active {
            continue;
        }
        if !line.starts_with("  ") && !line.starts_with('\t') {
            break;
        }
        let current = line.trim();
        if let Some(value) = current.strip_prefix("phase:") {
            phase = value.trim();
        }
        if let Some(value) = current.strip_prefix("type:") {
            error_type = value.trim();
        }
    }
    active.then_some((phase, error_type))
}

fn frontmatter(source: &str) -> Option<&str> {
    let start = source.find("/*---")? + 5;
    let end = source[start..].find("---*/")? + start;
    Some(&source[start..end])
}

fn failure_kind(error: &JsError) -> &str {
    match error.kind {
        JsErrorKind::Syntax => "SyntaxError",
        JsErrorKind::Type => "TypeError",
        JsErrorKind::Reference => "ReferenceError",
        JsErrorKind::ExecutionLimit => "ExecutionLimit",
        JsErrorKind::Exception => {
            if error.message.starts_with("TypeError:") {
                "TypeError"
            } else if error.message.starts_with("ReferenceError:") {
                "ReferenceError"
            } else if error.message.starts_with("SyntaxError:") {
                "SyntaxError"
            } else if error.message.starts_with("Test262Error:") {
                "Test262Error"
            } else {
                "Exception"
            }
        }
    }
}

fn evaluate(source: &str) -> Outcome {
    let Some(meta) = frontmatter(source) else {
        return Outcome::Skip("missing frontmatter");
    };
    let flags = words(meta, "flags");
    if flags.iter().any(|s| s == "module") {
        return Outcome::Skip("module");
    }
    if flags.iter().any(|s| s == "async") {
        return Outcome::Skip("async");
    }
    if flags.iter().any(|s| s == "onlyStrict") {
        return Outcome::Skip("onlyStrict");
    }
    if flags.iter().any(|s| s != "noStrict" && s != "raw") {
        return Outcome::Skip("unsupported flags");
    }
    if let Some((phase, _)) = negative(meta) {
        if phase == "parse" || phase == "early" {
            return Outcome::Skip("parse negative");
        }
        if phase != "runtime" {
            return Outcome::Skip("unsupported negative phase");
        }
    }
    if !words(meta, "includes")
        .iter()
        .all(|include| include == "assert.js" || include == "sta.js")
    {
        return Outcome::Skip("unsupported include");
    }
    // Do not count parse failures as passing a runtime-negative test.
    if let Err(error) = parse_script(source) {
        return Outcome::Fail(format!("parse: {error}"));
    }
    let mut vm = JsRuntime::new();
    if !flags.iter().any(|s| s == "raw")
        && let Err(error) = vm.eval_script(BOOTSTRAP)
    {
        return Outcome::Fail(format!("test harness: {error}"));
    }
    let result = vm.eval_script(source);
    match negative(meta) {
        Some(("runtime", expected)) => match result {
            Err(error) if failure_kind(&error) == expected => Outcome::Pass,
            Err(error) => Outcome::Fail(format!("expected {expected}, got {error}")),
            Ok(_) => Outcome::Fail(format!("expected runtime {expected}, did not throw")),
        },
        _ => match result {
            Ok(_) => Outcome::Pass,
            Err(error) => Outcome::Fail(format!("runtime: {error}")),
        },
    }
}

#[derive(Debug)]
struct Options {
    root: PathBuf,
    manifest: PathBuf,
    json_out: Option<PathBuf>,
    limit: usize,
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing Test262 test root")?;
    let mut manifest = None;
    let mut json_out = None;
    let mut limit = usize::MAX;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => manifest = args.next().map(PathBuf::from),
            "--json-out" => json_out = args.next().map(PathBuf::from),
            "--limit" => {
                limit = args
                    .next()
                    .ok_or("missing --limit value")?
                    .parse()
                    .map_err(|_| "invalid --limit value")?;
            }
            _ => return Err(format!("unexpected argument {arg}")),
        }
    }
    Ok(Options {
        root,
        manifest: manifest.ok_or("missing --manifest")?,
        json_out,
        limit,
    })
}

fn manifest_revision(source: &str) -> &str {
    source
        .lines()
        .find_map(|line| line.strip_prefix("# upstream="))
        .unwrap_or("unverified")
}

fn verify_upstream(config: &Options) -> Result<bool, String> {
    let manifest = fs::read_to_string(&config.manifest).map_err(|e| e.to_string())?;
    let expected = manifest_revision(&manifest);
    let Some(repo) = config.root.parent() else {
        return Ok(false);
    };
    if !repo.join(".git").exists() {
        return Ok(false);
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("cannot verify Test262 revision: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot verify Test262 checkout at {}",
            repo.display()
        ));
    }
    let actual = String::from_utf8_lossy(&output.stdout);
    if actual.trim() != expected {
        return Err(format!(
            "Test262 revision mismatch: manifest expects {expected}, checkout is {}",
            actual.trim()
        ));
    }
    Ok(true)
}

fn escape_json(value: &str) -> String {
    let mut json = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            x if x < '\u{0020}' => {
                use std::fmt::Write;
                write!(json, "\\u{:04x}", x as u32).expect("String write");
            }
            c => json.push(c),
        }
    }
    json.push('"');
    json
}

type CaseReport = (String, String, String);

fn run(config: &Options) -> Result<(Counts, Vec<CaseReport>), String> {
    let manifest = fs::read_to_string(&config.manifest).map_err(|e| e.to_string())?;
    let mut counts = Counts::default();
    let mut cases = Vec::new();
    for entry in manifest
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .take(config.limit)
    {
        if entry.starts_with('/') || entry.contains('\\') || entry.split('/').any(|s| s == "..") {
            return Err(format!("unsafe Test262 manifest entry: {entry}"));
        }
        let path = config.root.join(entry);
        let source = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let outcome = evaluate(&source);
        counts.listed += 1;
        let (status, detail) = match outcome {
            Outcome::Pass => {
                counts.passed += 1;
                if frontmatter(&source).and_then(negative).is_some() {
                    counts.negative_runtime += 1;
                } else {
                    counts.positive += 1;
                }
                ("pass", String::new())
            }
            Outcome::Fail(message) => {
                counts.failed += 1;
                ("fail", message)
            }
            Outcome::Skip(reason) => {
                counts.skipped += 1;
                match reason {
                    "module" => counts.skipped_modules += 1,
                    "async" => counts.skipped_async += 1,
                    "onlyStrict" => counts.skipped_strict += 1,
                    "parse negative" => counts.skipped_parse_negative += 1,
                    "unsupported include" => counts.skipped_includes += 1,
                    _ => counts.skipped_unsupported_flags += 1,
                }
                ("skip", reason.to_string())
            }
        };
        cases.push((entry.to_owned(), status.to_owned(), detail));
    }
    Ok((counts, cases))
}

fn report_json(
    config: &Options,
    counts: &Counts,
    cases: &[(String, String, String)],
    revision_verified: bool,
) -> Result<(), String> {
    let Some(path) = config.json_out.as_ref() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let manifest = fs::read_to_string(&config.manifest).map_err(|e| e.to_string())?;
    let mut json = format!(
        concat!(
            "{{\n  \"schema_version\": 1,\n",
            "  \"suite\": {},\n",
            "  \"upstream\": {}, \"revision_verified\": {},\n",
            "  \"listed\": {}, \"attempted\": {}, \"passed\": {}, \"failed\": {},\n",
            "  \"skipped\": {}, \"skipped_modules\": {}, \"skipped_async\": {},\n",
            "  \"skipped_strict\": {}, \"skipped_parse_negative\": {},\n",
            "  \"skipped_includes\": {}, \"skipped_other\": {},\n",
            "  \"positive_passed\": {}, \"runtime_negative_passed\": {},\n",
            "  \"percent\": {:.2},\n  \"cases\": [\n"
        ),
        escape_json(
            &config
                .manifest
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy(),
        ),
        escape_json(manifest_revision(&manifest)),
        revision_verified,
        counts.listed,
        counts.attempted(),
        counts.passed,
        counts.failed,
        counts.skipped,
        counts.skipped_modules,
        counts.skipped_async,
        counts.skipped_strict,
        counts.skipped_parse_negative,
        counts.skipped_includes,
        counts.skipped_unsupported_flags,
        counts.positive,
        counts.negative_runtime,
        counts.percent()
    );
    for (index, (name, status, detail)) in cases.iter().enumerate() {
        let comma = if index + 1 == cases.len() { "" } else { "," };
        json.push_str(&format!(
            "    {{\"file\":{},\"status\":{},\"detail\":{}}}{comma}\n",
            escape_json(name),
            escape_json(status),
            escape_json(detail)
        ));
    }
    json.push_str("  ]\n}\n");
    fs::write(path, json).map_err(|e| e.to_string())
}

fn main() {
    let config = options().unwrap_or_else(|error| {
        eprintln!("{error}");
        eprintln!("usage: test262_runtime_probe <test262/test path> --manifest PATH [--limit N] [--json-out PATH]");
        std::process::exit(2);
    });
    let verified = verify_upstream(&config).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    let (counts, cases) = run(&config).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    println!("Test262 runtime subset (classic sloppy scripts; pinned external fixtures)");
    println!("upstream_revision_verified={verified}");
    println!(
        "listed={} attempted={} passed={} failed={} skipped={} percent={:.2}",
        counts.listed,
        counts.attempted(),
        counts.passed,
        counts.failed,
        counts.skipped,
        counts.percent()
    );
    for (file, _status, detail) in cases
        .iter()
        .filter(|(_, status, _)| status == "fail")
        .take(20)
    {
        println!("  FAIL {file}: {detail}");
    }
    if let Err(error) = report_json(&config, &counts, &cases, verified) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_multiline_test262_metadata_without_silently_ignoring_includes() {
        assert_eq!(
            words(
                "includes:\n  - assert.js\n  - compareArray.js\nflags: [noStrict]",
                "includes"
            ),
            vec!["assert.js", "compareArray.js"]
        );
        assert_eq!(
            words("flags: [\n  onlyStrict,\n  async\n]\nincludes: []", "flags"),
            vec!["onlyStrict", "async"]
        );
        assert_eq!(
            words("includes: []\nflags: [noStrict]", "includes"),
            Vec::<String>::new()
        );
        assert_eq!(
            evaluate("/*---\nincludes:\n  - assert.js\n  - compareArray.js\n---*/\nassert(true);"),
            Outcome::Skip("unsupported include")
        );
    }

    #[test]
    fn metadata_and_negative_phase_are_safe() {
        let metadata = "flags: [noStrict]\nincludes: [assert.js, sta.js]\nnegative:\n  phase: runtime\n  type: TypeError";
        assert_eq!(words(metadata, "flags"), vec!["noStrict"]);
        assert_eq!(words(metadata, "includes"), vec!["assert.js", "sta.js"]);
        assert_eq!(negative(metadata), Some(("runtime", "TypeError")));
    }
    #[test]
    fn actual_runtime_assertions_can_pass_and_fail() {
        let positive = "/*---\nincludes: [assert.js]\n---*/\nassert.sameValue(1 + 2, 3);";
        assert_eq!(evaluate(positive), Outcome::Pass);
        let failure = "/*---\n---*/\nassert.sameValue(1 + 2, 4);";
        assert!(matches!(evaluate(failure), Outcome::Fail(_)));
        let negative = "/*---\nnegative:\n  phase: runtime\n  type: TypeError\n---*/\nthrow new TypeError('expected');";
        assert_eq!(evaluate(negative), Outcome::Pass);
        let wrong_negative = "/*---\nnegative:\n  phase: runtime\n  type: TypeError\n---*/\nthrow new ReferenceError('wrong');";
        assert!(matches!(evaluate(wrong_negative), Outcome::Fail(_)));
    }
    #[test]
    fn skips_are_not_counted_as_passes() {
        assert_eq!(
            evaluate("/*---\nflags: [module]\n---*/\n"),
            Outcome::Skip("module")
        );
        assert_eq!(
            evaluate("/*---\nflags: [onlyStrict]\n---*/\n"),
            Outcome::Skip("onlyStrict")
        );
        assert_eq!(
            evaluate("/*---\nnegative:\n  phase: parse\n  type: SyntaxError\n---*/\n"),
            Outcome::Skip("parse negative")
        );
        assert_eq!(
            evaluate("/*---\nincludes: [propertyHelper.js]\n---*/\n"),
            Outcome::Skip("unsupported include")
        );
        assert!(escape_json("a\n\"b").contains("\\n\\\"b"));
    }
}
