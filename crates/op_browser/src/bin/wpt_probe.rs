use op_engine::Engine;
use op_platform_win::render_display_list_to_bgra;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_WIDTH: i32 = 800;
const DEFAULT_HEIGHT: i32 = 600;
const FAILURE_LOG_LIMIT: usize = 30;
const FAILURE_DUMP_LIMIT: usize = 12;

#[derive(Default)]
struct Counts {
    total: usize,
    passed: usize,
    failed: usize,
    errors: usize,
}

#[derive(Debug)]
struct Config {
    root: PathBuf,
    manifest: PathBuf,
    width: i32,
    height: i32,
    channel_tolerance: u8,
    max_different_pixels: usize,
    json_out: Option<PathBuf>,
    dump_failures: Option<PathBuf>,
}

fn main() {
    let config = parse_args().unwrap_or_else(|error| {
        eprintln!("{error}");
        eprintln!(
            "usage: wpt_probe <wpt-root> <manifest.tsv> [--width N] [--height N] \
             [--channel-tolerance N] [--max-different-pixels N] [--json-out PATH] \
             [--dump-failures DIRECTORY]"
        );
        std::process::exit(2);
    });

    let manifest = fs::read_to_string(&config.manifest).unwrap_or_else(|error| {
        eprintln!(
            "failed to read WPT manifest {}: {error}",
            config.manifest.display()
        );
        std::process::exit(2);
    });
    let upstream = manifest_value(&manifest, "upstream").unwrap_or("unknown");
    let suite = manifest_suite_name(&config.manifest);
    let entries = parse_manifest(&manifest).unwrap_or_else(|error| {
        eprintln!("invalid WPT manifest: {error}");
        std::process::exit(2);
    });
    if entries.is_empty() {
        eprintln!("WPT manifest contains no tests");
        std::process::exit(2);
    }

    let mut counts = Counts::default();
    let mut logged_failures = 0usize;
    if let Some(directory) = &config.dump_failures {
        fs::create_dir_all(directory).unwrap_or_else(|error| {
            eprintln!(
                "cannot create failure dump {}: {error}",
                directory.display()
            );
            std::process::exit(2);
        });
    }

    for (test, reference) in entries {
        counts.total += 1;
        let test_pixels = render(&config.root, &test, config.width, config.height);
        let reference_pixels = render(&config.root, &reference, config.width, config.height);

        match (test_pixels, reference_pixels) {
            (Ok(test_pixels), Ok(reference_pixels)) => {
                let different =
                    different_pixels(&test_pixels, &reference_pixels, config.channel_tolerance);
                if different <= config.max_different_pixels {
                    counts.passed += 1;
                } else {
                    counts.failed += 1;
                    if counts.failed <= FAILURE_DUMP_LIMIT
                        && let Some(directory) = &config.dump_failures
                        && let Err(error) = dump_bitmaps(
                            directory,
                            counts.total,
                            (config.width, config.height),
                            &test_pixels,
                            &reference_pixels,
                        )
                    {
                        eprintln!("failed to dump case {}: {error}", counts.total);
                        std::process::exit(2);
                    }
                    if logged_failures < FAILURE_LOG_LIMIT {
                        println!("FAIL {test} != {reference}: different_pixels={different}");
                        logged_failures += 1;
                    }
                }
            }
            (test_result, reference_result) => {
                counts.failed += 1;
                counts.errors += 1;
                if logged_failures < FAILURE_LOG_LIMIT {
                    let test_error = test_result
                        .err()
                        .unwrap_or_else(|| "test rendered successfully".to_owned());
                    let reference_error = reference_result
                        .err()
                        .unwrap_or_else(|| "reference rendered successfully".to_owned());
                    println!(
                        "ERROR {test} vs {reference}: test={test_error}; reference={reference_error}"
                    );
                    logged_failures += 1;
                }
            }
        }
    }

    let percent = percent(counts.passed, counts.total);
    println!("WPT reftest subset");
    println!("suite={suite}");
    println!("upstream={upstream}");
    println!("tests_checked={}", counts.total);
    println!("passed={}", counts.passed);
    println!("failed={}", counts.failed);
    println!("render_errors={}", counts.errors);
    println!("percent={percent:.2}");
    if let Some(directory) = &config.dump_failures {
        println!(
            "failure_bitmaps_saved_at={} (up to {FAILURE_DUMP_LIMIT} cases)",
            directory.display()
        );
    }

    if let Some(path) = config.json_out
        && let Err(error) = write_json(&path, &suite, upstream, &counts, percent)
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
        .ok_or_else(|| "missing WPT root".to_owned())?;
    let manifest = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing WPT manifest".to_owned())?;

    let mut config = Config {
        root,
        manifest,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        channel_tolerance: 0,
        max_different_pixels: 0,
        json_out: None,
        dump_failures: None,
    };

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--width" => {
                config.width = parse_positive_i32(args.next(), "--width")?;
            }
            "--height" => {
                config.height = parse_positive_i32(args.next(), "--height")?;
            }
            "--channel-tolerance" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--channel-tolerance requires a number".to_owned())?;
                config.channel_tolerance = value
                    .parse::<u8>()
                    .map_err(|_| "--channel-tolerance must be in 0..=255".to_owned())?;
            }
            "--max-different-pixels" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--max-different-pixels requires a number".to_owned())?;
                config.max_different_pixels = value.parse::<usize>().map_err(|_| {
                    "--max-different-pixels must be a non-negative integer".to_owned()
                })?;
            }
            "--json-out" => {
                config.json_out = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--json-out requires a path".to_owned())?,
                ));
            }
            "--dump-failures" => {
                config.dump_failures =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        "--dump-failures requires a directory".to_owned()
                    })?));
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }

    Ok(config)
}

fn manifest_suite_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("wpt-reftest")
        .to_owned()
}

fn parse_positive_i32(value: Option<String>, name: &str) -> Result<i32, String> {
    let value = value.ok_or_else(|| format!("{name} requires a number"))?;
    let parsed = value
        .parse::<i32>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed <= 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

fn parse_manifest(source: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((test, reference)) = line.split_once('\t') else {
            return Err(format!("line {} is not test<TAB>reference", index + 1));
        };
        let test = test.trim();
        let reference = reference.trim();
        if test.is_empty() || reference.is_empty() {
            return Err(format!("line {} contains an empty path", index + 1));
        }
        entries.push((test.to_owned(), reference.to_owned()));
    }
    Ok(entries)
}

fn manifest_value<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("# {key}=");
    source
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
}

fn render(root: &Path, relative: &str, width: i32, height: i32) -> Result<Vec<u8>, String> {
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!("missing file {}", path.display()));
    }
    let source = path
        .to_str()
        .ok_or_else(|| format!("non-Unicode path {}", path.display()))?;
    let engine = Engine::new();
    let page = engine
        .render_source(source, width, height)
        .map_err(|error| error.to_string())?;
    render_display_list_to_bgra(&page.display_list, width, height)
}

/// Keep the probe dependency-free: 32-bit top-down Windows BMP stores the
/// native renderer's BGRA pixels directly, without an image encoder.
fn bmp_from_bgra(pixels: &[u8], width: i32, height: i32) -> std::io::Result<Vec<u8>> {
    let expected = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|pixels| pixels.checked_mul(4));
    if expected != Some(pixels.len()) || height == i32::MIN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "BGRA dimensions do not match the pixel buffer",
        ));
    }
    let data_len = u32::try_from(pixels.len()).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "bitmap exceeds 4 GiB")
    })?;
    let file_len = data_len.checked_add(54).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "bitmap exceeds 4 GiB")
    })?;
    let mut bmp = Vec::with_capacity(file_len as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_len.to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&(-height).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&data_len.to_le_bytes());
    bmp.extend_from_slice(&[0; 16]);
    bmp.extend_from_slice(pixels);
    Ok(bmp)
}

fn dump_bitmaps(
    directory: &Path,
    index: usize,
    dimensions: (i32, i32),
    actual: &[u8],
    expected: &[u8],
) -> std::io::Result<()> {
    for (suffix, pixels) in [("actual", actual), ("reference", expected)] {
        let path = directory.join(format!("case-{index:04}-{suffix}.bmp"));
        fs::write(path, bmp_from_bgra(pixels, dimensions.0, dimensions.1)?)?;
    }
    Ok(())
}

fn different_pixels(left: &[u8], right: &[u8], tolerance: u8) -> usize {
    if left.len() != right.len() || !left.len().is_multiple_of(4) {
        return usize::MAX;
    }
    let (left_pixels, _) = left.as_chunks::<4>();
    let (right_pixels, _) = right.as_chunks::<4>();
    left_pixels
        .iter()
        .zip(right_pixels)
        .filter(|(left, right)| {
            (0..3).any(|channel| left[channel].abs_diff(right[channel]) > tolerance)
        })
        .count()
}

fn percent(passed: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        passed as f64 * 100.0 / total as f64
    }
}

fn write_json(
    path: &Path,
    suite: &str,
    upstream: &str,
    counts: &Counts,
    percent: f64,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let json = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": 1,\n",
            "  \"suite\": \"{}\",\n",
            "  \"upstream\": \"{}\",\n",
            "  \"total\": {},\n",
            "  \"passed\": {},\n",
            "  \"failed\": {},\n",
            "  \"errors\": {},\n",
            "  \"percent\": {:.2}\n",
            "}}\n"
        ),
        suite, upstream, counts.total, counts.passed, counts.failed, counts.errors, percent
    );
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_comparison_ignores_alpha_and_honors_tolerance() {
        let left = [10, 20, 30, 0, 40, 50, 60, 0];
        let right = [11, 20, 30, 255, 40, 52, 60, 255];
        assert_eq!(different_pixels(&left, &right, 0), 2);
        assert_eq!(different_pixels(&left, &right, 1), 1);
        assert_eq!(different_pixels(&left, &right, 2), 0);
    }

    #[test]
    fn derives_suite_name_from_manifest_filename() {
        assert_eq!(
            manifest_suite_name(Path::new("compat/wpt-positioning-v1.tsv")),
            "wpt-positioning-v1"
        );
    }

    #[test]
    fn top_down_bmp_wraps_bgra_without_reordering_channels() {
        let pixels = [10, 20, 30, 255, 40, 50, 60, 255];
        let bmp = bmp_from_bgra(&pixels, 2, 1).unwrap();
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()), 62);
        assert_eq!(i32::from_le_bytes(bmp[22..26].try_into().unwrap()), -1);
        assert_eq!(&bmp[54..], &pixels);
        assert!(bmp_from_bgra(&pixels, 3, 1).is_err());
        assert!(bmp_from_bgra(&pixels, 2, -1).is_err());
    }

    #[test]
    fn parses_manifest_comments_and_entries() {
        let entries =
            parse_manifest("# header\na.html\tref-a.html\n\nb.html\tref-b.html\n").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], ("a.html".to_owned(), "ref-a.html".to_owned()));
    }
}
