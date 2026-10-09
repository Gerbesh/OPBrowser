//! Fixed-revision WPT DOM smoke probe. Executes the ORIGINAL pinned fixture
//! source with a deliberately limited synchronous testharness adapter.
//! This is NOT the official WPT harness or an overall DOM pass rate.
use op_engine::Engine;
use op_paint::PaintCommand;
use std::path::Path;
use std::process::Command;

const WPT_REVISION: &str = "97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d";
const CASE: &str = "dom/nodes/Node-childNodes-cache.html";
const STATUS: &str = "OPBROWSER_WPT_DOM_PASS";

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

fn instrument(original: &str) -> Result<String, String> {
    // The fixture contains a single inline test(function() {...}) and
    // testharness/report script imports. Do not reinterpret other fixtures
    // with this adapter: incompatible tests are explicitly unattempted.
    if original.matches("test(function()").count() != 1 {
        return Err("fixture test() structure changed; review adapter".into());
    }
    let mut source = original.to_owned();
    for import in [
        r#"<script src="/resources/testharness.js"></script>"#,
        r#"<script src="/resources/testharnessreport.js"></script>"#,
    ] {
        if !source.contains(import) {
            return Err(format!(
                "missing pinned fixture harness reference: {import}"
            ));
        }
        source = source.replace(import, "");
    }
    let shim = concat!(
        "<script>",
        "function assert_equals(actual,expected){",
        "if(!Object.is(actual,expected))throw new Error('WPT assert_equals');",
        "}",
        "function test(callback){",
        "try{callback(); document.getElementById('opb-wpt-status').textContent='OPBROWSER_WPT_DOM_PASS';}",
        "catch(error){document.getElementById('opb-wpt-status').textContent='OPBROWSER_WPT_DOM_FAIL';}",
        "}",
        "</script>"
    );
    let location = source
        .rfind("<script>")
        .ok_or("fixture inline test script missing")?;
    source.insert_str(location, shim);
    // Inject a visible pass/fail marker after the fixture's target container,
    // without replacing or adjusting the actual test assertions.
    let anchor = r#"<div id="target"><div id="first"></div><div id="second"></div><div id="third"></div><div id="last"></div></div>"#;
    let replacement = format!("{anchor}<p id=\"opb-wpt-status\">OPBROWSER_WPT_DOM_NOT_RUN</p>");
    if !source.contains(anchor) {
        return Err("original WPT target markup changed".into());
    }
    Ok(source.replacen(anchor, &replacement, 1))
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
    let rev = git(path, &["rev-parse", "HEAD"])?;
    if rev.trim() != WPT_REVISION {
        return Err(format!(
            "wrong WPT commit {}; expected {WPT_REVISION}",
            rev.trim()
        ));
    }
    let manifest = include_str!("../../../../compat/wpt-dom-smoke-v1.tsv");
    let mut cases = 0usize;
    let mut explicit_skips = 0usize;
    let mut attempted = 0usize;
    for line in manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
    {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err("invalid WPT smoke manifest row".into());
        }
        cases += 1;
        match fields[1] {
            "attempt" if fields[0] == CASE => attempted += 1,
            "skip" => {
                explicit_skips += 1;
                println!("SKIP {}: {}", fields[0], fields[2]);
            }
            other => {
                return Err(format!(
                    "unsupported manifest operation or fixture: {other}"
                ));
            }
        }
    }
    if attempted != 1 || cases != 4 {
        return Err("WPT DOM v1 manifest unexpectedly changed".into());
    }
    let contents = git(path, &["show", &format!("HEAD:{CASE}")])?;
    let html = instrument(&contents)?;
    let mut engine = Engine::new();
    let list = engine.set_html_page(&html, 800, 600);
    let report = engine
        .active_script_report()
        .ok_or("engine returned no script execution report")?;
    let passed = report.failed == 0
        && list
            .commands
            .iter()
            .any(|cmd| matches!(cmd,PaintCommand::Text {text,..} if text.contains(STATUS)));
    // Distinguish executed tests from WPT sections we cannot presently run.
    println!("WPT pinned DOM smoke (original upstream file, limited harness adapter)");
    println!("upstream_revision_verified=true");
    println!(
        "selected={cases} attempted={attempted} passed={} failed={} skipped={explicit_skips}",
        usize::from(passed),
        usize::from(!passed)
    );
    println!("fixture={CASE}");
    println!(
        "script_report=executed:{} failed:{} skipped:{}",
        report.executed, report.failed, report.skipped
    );
    if !passed {
        return Err("original WPT DOM fixture did not pass".into());
    }
    Ok(())
}
