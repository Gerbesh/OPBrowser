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

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next() else {
        eprintln!("usage: test262_probe <test262/test path> [--limit N]");
        std::process::exit(2);
    };

    let mut limit = None;
    while let Some(argument) = args.next() {
        if argument == "--limit" {
            let Some(value) = args.next() else {
                eprintln!("--limit requires a number");
                std::process::exit(2);
            };
            limit = value.parse::<usize>().ok();
        } else {
            eprintln!("unknown argument: {argument}");
            std::process::exit(2);
        }
    }

    let mut files = Vec::new();
    if let Err(error) = collect_js_files(Path::new(&root), &mut files) {
        eprintln!("failed to scan {root}: {error}");
        std::process::exit(2);
    }
    files.sort();

    let mut counts = Counts::default();
    for path in files.into_iter().take(limit.unwrap_or(usize::MAX)) {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let metadata = frontmatter(&source);
        if metadata.is_some_and(|meta| meta.contains("flags: [module]")) {
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

    let percent = if counts.files == 0 {
        0.0
    } else {
        counts.passed as f64 * 100.0 / counts.files as f64
    };
    println!("Test262 parse probe");
    println!("files_checked={}", counts.files);
    println!("parse_expectation_passed={}", counts.passed);
    println!("positive_tests={}", counts.positive);
    println!("negative_parse_tests={}", counts.negative_parse);
    println!("skipped_modules={}", counts.skipped_modules);
    println!("parse_expectation_percent={percent:.2}");
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
