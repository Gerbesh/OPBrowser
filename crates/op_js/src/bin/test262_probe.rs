use op_js::parse_script;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Counts {
    files: usize,
    passed: usize,
    positive: usize,
    negative_parse: usize,
    skipped_modules: usize,
}

struct Config {
    root: PathBuf,
    manifest: Option<PathBuf>,
    limit: Option<usize>,
    json_out: Option<PathBuf>,
}

fn main() {
    let config = parse_args().unwrap_or_else(|error| {
        eprintln!("{error}");
        eprintln!(
            "usage: test262_probe <test262/test path> [--manifest PATH] [--limit N] [--json-out PATH]"
        );
        std::process::exit(2);
    });

    let (files, upstream) = if let Some(manifest_path) = &config.manifest {
        let manifest = fs::read_to_string(manifest_path).unwrap_or_else(|error| {
            eprintln!("failed to read {}: {error}", manifest_path.display());
            std::process::exit(2);
        });
        let upstream = manifest_value(&manifest, "upstream")
            .unwrap_or("unknown")
            .to_owned();
        let files = manifest
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|relative| config.root.join(relative))
            .collect::<Vec<_>>();
        (files, upstream)
    } else {
        let mut files = Vec::new();
        if let Err(error) = collect_js_files(&config.root, &mut files) {
            eprintln!("failed to scan {}: {error}", config.root.display());
            std::process::exit(2);
        }
        files.sort();
        (files, "unversioned-local-checkout".to_owned())
    };

    let mut counts = Counts::default();
    for path in files.into_iter().take(config.limit.unwrap_or(usize::MAX)) {
        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            eprintln!("failed to read manifest test {}: {error}", path.display());
            std::process::exit(2);
        });
        let metadata = frontmatter(&source);
        if metadata.is_some_and(is_module_test) {
            counts.skipped_modules += 1;
            continue;
        }

        let negative_parse = metadata
            .is_some_and(|meta| meta.contains("negative:") && meta.contains("phase: parse"));
        let parsed = parse_script(&source).is_ok();

        counts.files += 1;
        if negative_parse {
            counts.negative_parse += 1;
            if !parsed {
                counts.passed += 1;
            }
        } else {
            counts.positive += 1;
            if parsed {
                counts.passed += 1;
            }
        }
    }

    let percent = percent(counts.passed, counts.files);
    println!("Test262 parse probe");
    println!("upstream={upstream}");
    println!("files_checked={}", counts.files);
    println!("parse_expectation_passed={}", counts.passed);
    println!("positive_tests={}", counts.positive);
    println!("negative_parse_tests={}", counts.negative_parse);
    println!("skipped_modules={}", counts.skipped_modules);
    println!("parse_expectation_percent={percent:.2}");

    if let Some(path) = config.json_out
        && let Err(error) = write_json(&path, &upstream, &counts, percent)
    {
        eprintln!("failed to write {}: {error}", path.display());
        std::process::exit(2);
    }
}

fn parse_args() -> Result<Config, String> {
    let mut args = std::env::args().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing Test262 test root".to_owned())?;

    let mut manifest = None;
    let mut limit = None;
    let mut json_out = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--manifest" => {
                manifest = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--manifest requires a path".to_owned())?,
                ));
            }
            "--limit" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--limit requires a number".to_owned())?;
                limit = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| "--limit requires a non-negative integer".to_owned())?,
                );
            }
            "--json-out" => {
                json_out = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--json-out requires a path".to_owned())?,
                ));
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }

    Ok(Config {
        root,
        manifest,
        limit,
        json_out,
    })
}

fn percent(passed: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        passed as f64 * 100.0 / total as f64
    }
}

fn is_module_test(metadata: &str) -> bool {
    metadata.lines().any(|line| {
        let line = line.trim();
        line.starts_with("flags:")
            && line.split_once(':').is_some_and(|(_, flags)| {
                flags
                    .split([',', '[', ']', ' '])
                    .any(|flag| flag == "module")
            })
    })
}

fn manifest_value<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("# {key}=");
    source
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
}

fn write_json(path: &Path, upstream: &str, counts: &Counts, percent: f64) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let json = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": 1,\n",
            "  \"suite\": \"test262-parser-v1\",\n",
            "  \"upstream\": \"{}\",\n",
            "  \"total\": {},\n",
            "  \"passed\": {},\n",
            "  \"positive\": {},\n",
            "  \"negative_parse\": {},\n",
            "  \"skipped_modules\": {},\n",
            "  \"percent\": {:.2}\n",
            "}}\n"
        ),
        upstream,
        counts.files,
        counts.passed,
        counts.positive,
        counts.negative_parse,
        counts.skipped_modules,
        percent
    );
    fs::write(path, json)
}

fn collect_js_files(path: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "js") {
            output.push(path.to_owned());
        }
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .components()
            .any(|component| component.as_os_str() == "harness")
        {
            continue;
        }
        collect_js_files(&path, output)?;
    }
    Ok(())
}

fn frontmatter(source: &str) -> Option<&str> {
    let start = source.find("/*---")? + 5;
    let end = source[start..].find("---*/")? + start;
    Some(&source[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_flag_detection_is_token_based() {
        assert!(is_module_test("flags: [module, raw]"));
        assert!(!is_module_test("description: module\nflags: [raw]"));
    }

    #[test]
    fn manifest_metadata_is_read_from_comments() {
        let source = "# upstream=abc123\n# entries=2\na.js\nb.js\n";
        assert_eq!(manifest_value(source, "upstream"), Some("abc123"));
    }
}
