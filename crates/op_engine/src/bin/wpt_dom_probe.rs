//! Pinned original WPT DOM source runner with a narrow sync test adapter.
//! Only explicitly listed one-test fixtures are executable; unsupported
//! WPT cases are explicit SKIPs, never counted as passes.
use op_engine::Engine;
use op_paint::PaintCommand;
use std::path::Path;
use std::process::Command;

const WPT_REVISION: &str = "97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d";
const PASS: &str = "OPBROWSER_WPT_DOM_PASS";
const FAIL: &str = "OPBROWSER_WPT_DOM_FAIL";
const NOT_RUN: &str = "OPBROWSER_WPT_DOM_NOT_RUN";
const MANIFEST: &str = include_str!("../../../../compat/wpt-dom-smoke-v3.tsv");

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| format!("git unavailable: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

/// Do not modify original WPT assertions; replace only the two external
/// harness imports, and add a reporting marker and limited harness shim.
/// Any unfamiliar fixture shape fails closed rather than "passing" by
/// silently ignoring its scripts.
fn instrument(original: &str, expected_tests: usize) -> Result<String, String> {
    if original.matches("test(function()").count() != expected_tests {
        return Err(format!(
            "WPT fixture test() count changed; expected {expected_tests}"
        ));
    }
    let mut source = original.to_owned();
    for import in [
        r#"<script src="/resources/testharness.js"></script>"#,
        r#"<script src="/resources/testharnessreport.js"></script>"#,
    ] {
        if source.matches(import).count() != 1 {
            return Err(format!(
                "pinned WPT harness reference missing/duplicated: {import}"
            ));
        }
        source = source.replacen(import, "", 1);
    }
    // Single original test() statement, no unhandled external scripts.
    if source.contains("<script src=") {
        return Err("unresolved external WPT test script".into());
    }
    let pos = source
        .rfind("<script>")
        .ok_or("fixture inline test script missing")?;
    let marker = format!("<p id=\"opb-wpt-status\">{NOT_RUN}</p>");
    // Sticky error state: later passing tests never override earlier failures.
    let shim = concat!(
        "<script>",
        "var __opb_wpt_count=0;",
        "var __opb_wpt_failed=false;",
        "function assert_equals(actual,expected){",
        "if(!Object.is(actual,expected))throw new Error('WPT assert_equals');",
        "}",
        "function assert_true(actual){",
        "if(!actual)throw new Error('WPT assert_true');",
        "}",
        "function assert_false(actual){",
        "if(actual)throw new Error('WPT assert_false');",
        "}",
        "function test(callback){",
        "__opb_wpt_count=__opb_wpt_count+1;",
        "try{callback();}catch(error){__opb_wpt_failed=true;}",
        "document.getElementById('opb-wpt-status').textContent=",
        "(__opb_wpt_failed?'OPBROWSER_WPT_DOM_FAIL_':'OPBROWSER_WPT_DOM_PASS_')+__opb_wpt_count;",
        "}",
        "</script>"
    );
    source.insert_str(pos, &format!("{marker}{shim}"));
    Ok(source)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("WPT DOM probe ERROR: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let repo = std::env::args()
        .nth(1)
        .ok_or("usage: wpt_dom_probe <pinned-WPT-git-checkout>")?;
    let path = Path::new(&repo);
    let revision = git(path, &["rev-parse", "HEAD"])?;
    if revision.trim() != WPT_REVISION {
        return Err(format!(
            "wrong WPT revision {}, expected {WPT_REVISION}",
            revision.trim()
        ));
    }

    let mut visited = std::collections::HashSet::new();
    let mut selected = 0usize;
    let mut attempted = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    for line in MANIFEST
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
    {
        let columns: Vec<_> = line.split('\t').collect();
        if columns.len() != 4 {
            return Err("invalid WPT DOM v3 manifest row".into());
        }
        let (fixture, status, count, reason) = (columns[0], columns[1], columns[2], columns[3]);
        let expected_tests = count
            .parse::<usize>()
            .map_err(|_| "invalid per-fixture test count")?;
        if expected_tests > 16 {
            return Err("WPT per-file test budget exceeded".into());
        }
        if !fixture.starts_with("dom/nodes/")
            || !fixture.ends_with(".html")
            || fixture.contains("..")
        {
            return Err(format!("invalid pinned fixture path {fixture}"));
        }
        if !visited.insert(fixture) {
            return Err(format!("duplicate WPT DOM fixture {fixture}"));
        }
        git(path, &["cat-file", "-e", &format!("HEAD:{fixture}")])?;
        selected += 1;
        match status {
            "skip" => {
                if expected_tests != 0 {
                    return Err("skip must declare zero attempted test calls".into());
                }
                skipped += 1;
                println!("SKIP {fixture}: {reason}");
            }
            "attempt" => {
                if expected_tests == 0 {
                    return Err("attempt must declare expected test calls".into());
                }
                attempted += 1;
                let source = git(path, &["show", &format!("HEAD:{fixture}")])?;
                let html = instrument(&source, expected_tests)?;
                let mut engine = Engine::new();
                let display = engine.set_html_page(&html, 800, 600);
                let report = engine
                    .active_script_report()
                    .ok_or("DOM fixture produced no script report")?;
                let text = display
                    .commands
                    .iter()
                    .filter_map(|cmd| match cmd {
                        PaintCommand::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let pass = report.failed == 0
                    && report.skipped == 0
                    && report.executed == 2
                    && text
                        .iter()
                        .any(|text| text.contains(&format!("{PASS}_{expected_tests}")))
                    && !text
                        .iter()
                        .any(|text| text.contains(FAIL) || text.contains(NOT_RUN));
                if pass {
                    passed += 1;
                    println!("PASS {fixture}");
                } else {
                    failed += 1;
                    println!(
                        "FAIL {fixture}: scripts executed={} failed={} skipped={} text={text:?}",
                        report.executed, report.failed, report.skipped
                    );
                }
            }
            _ => return Err(format!("unknown WPT manifest disposition for {fixture}")),
        }
    }
    if selected != 13 || attempted != 10 || skipped != 3 {
        return Err("frozen WPT DOM v3 manifest shape changed".into());
    }
    println!("WPT DOM smoke v3: multi-test synchronous shim, unchanged pinned fixture assertions");
    println!("upstream_revision_verified=true");
    println!(
        "selected={selected} attempted={attempted} passed={passed} failed={failed} skipped={skipped}"
    );
    if failed > 0 {
        return Err(format!("{failed} WPT DOM fixture(s) failed"));
    }
    Ok(())
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    fn fixture(bad: bool) -> String {
        let first = if bad { "2" } else { "1" };
        format!(
            "<body><script src=\"/resources/testharness.js\"></script>\
             <script src=\"/resources/testharnessreport.js\"></script>\
             <script>test(function(){{assert_equals(1,{first});}});\
             test(function(){{assert_equals(1,1);}});</script></body>"
        )
    }

    #[test]
    fn prior_failure_is_not_overwritten_by_later_success() {
        let html = instrument(&fixture(true), 2).unwrap();
        let mut engine = Engine::new();
        let page = engine.set_html_page(&html, 800, 600);
        let texts: Vec<_> = page
            .commands
            .iter()
            .filter_map(|cmd| match cmd {
                PaintCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|s| s.contains("OPBROWSER_WPT_DOM_FAIL_2")),
            "{texts:?}"
        );
        assert!(!texts.iter().any(|s| s.contains("OPBROWSER_WPT_DOM_PASS_2")));
        assert_eq!(engine.active_script_report().unwrap().failed, 0);
    }

    #[test]
    fn multiple_successes_require_exact_expected_count() {
        let html = instrument(&fixture(false), 2).unwrap();
        let mut engine = Engine::new();
        let page = engine.set_html_page(&html, 800, 600);
        assert!(page.commands.iter().any(|cmd| matches!(
            cmd,PaintCommand::Text{text,..} if text.contains("OPBROWSER_WPT_DOM_PASS_2")
        )));
        assert!(instrument(&fixture(false), 3).is_err());
        assert_eq!(engine.active_script_report().unwrap().failed, 0);
    }
}
