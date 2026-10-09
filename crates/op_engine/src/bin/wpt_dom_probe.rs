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
const MANIFEST: &str = include_str!("../../../../compat/wpt-dom-smoke-v2.tsv");

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
fn instrument(original: &str) -> Result<String, String> {
    if original.matches("test(function()").count() != 1 {
        return Err("unsupported WPT test() structure; review adapter".into());
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
    let shim = concat!(
        "<script>",
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
        "try {callback(); document.getElementById('opb-wpt-status').textContent='OPBROWSER_WPT_DOM_PASS';}",
        "catch(error){document.getElementById('opb-wpt-status').textContent='OPBROWSER_WPT_DOM_FAIL';}",
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
        if columns.len() != 3 {
            return Err("invalid pinned DOM manifest row".into());
        }
        let (fixture, status, reason) = (columns[0], columns[1], columns[2]);
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
                skipped += 1;
                println!("SKIP {fixture}: {reason}");
            }
            "attempt" => {
                attempted += 1;
                let source = git(path, &["show", &format!("HEAD:{fixture}")])?;
                let html = instrument(&source)?;
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
                    && text.iter().any(|text| text.contains(PASS))
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
    if selected != 10 || attempted != 7 || skipped != 3 {
        return Err("frozen WPT DOM v2 manifest shape changed".into());
    }
    println!("WPT DOM smoke v2: limited synchronous harness, original pinned fixture assertions");
    println!("upstream_revision_verified=true");
    println!(
        "selected={selected} attempted={attempted} passed={passed} failed={failed} skipped={skipped}"
    );
    if failed > 0 {
        return Err(format!("{failed} WPT DOM fixture(s) failed"));
    }
    Ok(())
}
