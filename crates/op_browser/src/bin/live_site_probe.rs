//! Live public-site diagnostic: the OPBrowser engine and native renderer,
//! never a Chromium/WebView-based substitute. Invoked by tools/live_web_suite.py.
//! No account, cookie injection, authentication bypass or raw HTML retention.
use op_engine::{Engine, RenderedPage};
use op_paint::PaintCommand;
use op_platform_win::render_display_list_to_bgra;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn report(key: &str, value: impl std::fmt::Display) {
    let sanitized = value.to_string().replace(['\t', '\r', '\n'], " ");
    println!("OP_SITE\t{key}\t{sanitized}");
    let _ = io::stdout().flush();
}

fn paint_counts(commands: &[PaintCommand], totals: &mut (usize, usize, usize)) {
    for command in commands {
        match command {
            PaintCommand::Text { .. } => totals.0 += 1,
            PaintCommand::Image { .. } | PaintCommand::BackgroundImage { .. } => totals.1 += 1,
            PaintCommand::Clipped { command, .. } => {
                paint_counts(std::slice::from_ref(command.as_ref()), totals);
            }
            _ => totals.2 += 1,
        }
    }
}

fn sample_text(commands: &[PaintCommand], sample: &mut String) {
    for command in commands {
        match command {
            PaintCommand::Text { text, .. } => {
                if sample.len() < 240 {
                    sample.push_str(text);
                    sample.push(' ');
                }
            }
            PaintCommand::Clipped { command, .. } => {
                sample_text(std::slice::from_ref(command.as_ref()), sample);
            }
            _ => {}
        }
    }
}

fn bmp_from_bgra(pixels: &[u8], width: i32, height: i32) -> io::Result<Vec<u8>> {
    let expected = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|n| n.checked_mul(4));
    if expected != Some(pixels.len()) || height == i32::MIN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid BGRA size",
        ));
    }
    let data_len = u32::try_from(pixels.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "bitmap too large"))?;
    let file_len = data_len
        .checked_add(54)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bitmap too large"))?;
    let mut bmp = Vec::with_capacity(file_len as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_len.to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54_u32.to_le_bytes());
    bmp.extend_from_slice(&40_u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&(-height).to_le_bytes());
    bmp.extend_from_slice(&1_u16.to_le_bytes());
    bmp.extend_from_slice(&32_u16.to_le_bytes());
    bmp.extend_from_slice(&0_u32.to_le_bytes());
    bmp.extend_from_slice(&data_len.to_le_bytes());
    bmp.extend_from_slice(&[0; 16]);
    bmp.extend_from_slice(pixels);
    Ok(bmp)
}

fn snapshot(page: &RenderedPage, path: &PathBuf, width: i32, height: i32) -> Result<usize, String> {
    let pixels = render_display_list_to_bgra(&page.display_list, width, height)?;
    let painted = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] < 247 || pixel[1] < 247 || pixel[2] < 247)
        .count();
    fs::write(
        path,
        bmp_from_bgra(&pixels, width, height).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(painted)
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut url = None;
    let mut bitmap = None;
    let mut settle_ms = 1300_u64;
    let mut width = 1280_i32;
    let mut height = 720_i32;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--url" => url = args.next(),
            "--bitmap" => bitmap = args.next().map(PathBuf::from),
            "--settle-ms" => {
                settle_ms = args
                    .next()
                    .ok_or("missing settle-ms")?
                    .parse()
                    .map_err(|_| "bad settle-ms")?
            }
            "--width" => {
                width = args
                    .next()
                    .ok_or("missing width")?
                    .parse()
                    .map_err(|_| "bad width")?
            }
            "--height" => {
                height = args
                    .next()
                    .ok_or("missing height")?
                    .parse()
                    .map_err(|_| "bad height")?
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let url = url.ok_or("missing --url")?;
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("live probe requires an HTTP(S) URL".into());
    }
    if !(320..=1920).contains(&width) || !(240..=1200).contains(&height) || settle_ms > 10_000 {
        return Err("invalid viewport or settle duration".into());
    }

    report("phase", "navigation");
    let mut engine = Engine::new();
    let started = Instant::now();
    let mut page = match engine.navigate(&url, width, height) {
        Ok(page) => page,
        Err(error) => {
            report("load_ms", started.elapsed().as_millis());
            report("navigate_error", error);
            return Err("document navigation failed".into());
        }
    };
    report("load_ms", started.elapsed().as_millis());
    report("resolved_url", &page.address);
    report("mime_type", &page.mime_type);
    report("phase", "settling");

    let settle_started = Instant::now();
    let mut ticks = 0_u64;
    let mut repaints = 0_u64;
    while settle_started.elapsed() < Duration::from_millis(settle_ms) {
        if let Some(updated) = engine.tick_timers(width, height) {
            page = updated;
            repaints += 1;
        }
        ticks += 1;
        std::thread::sleep(Duration::from_millis(10));
    }
    report("settle_ticks", ticks);
    report("settle_repaints", repaints);
    report(
        "script_executed",
        engine.active_script_report().map_or(0, |r| r.executed),
    );
    report(
        "script_failed",
        engine.active_script_report().map_or(0, |r| r.failed),
    );
    report(
        "script_skipped",
        engine.active_script_report().map_or(0, |r| r.skipped),
    );
    if let Some(diagnostics) = engine.active_script_diagnostics() {
        report("script_diagnostics_count", diagnostics.len());
        for (index, diagnostic) in diagnostics.iter().enumerate() {
            report(&format!("script_diag_{index}"), diagnostic);
        }
    }
    report(
        "dom_mutations",
        engine.active_script_report().map_or(0, |r| r.mutations),
    );
    report(
        "style_errors",
        engine.active_style_errors().map_or(0, |r| r.len()),
    );
    report(
        "native_dom_slots",
        engine.active_document_node_slots().unwrap_or(0),
    );
    if let Some(errors) = engine.active_event_listener_errors() {
        report("event_listener_diagnostics_count", errors.len());
        for (index, error) in errors.iter().take(16).enumerate() {
            report(
                &format!("event_listener_diag_{index}"),
                error.chars().take(160).collect::<String>(),
            );
        }
    }
    if let Some(gc) = engine.active_script_gc_telemetry() {
        report("js_gc_ok", gc.successful);
        report("js_gc_errors", gc.failed);
        report("js_gc_max_us", gc.max_pause.as_micros());
    }
    if let Some(gc) = engine.active_host_gc_checkpoint_telemetry() {
        report("host_gc_max_us", gc.max_pause.as_micros());
    }
    if let Some(gc) = engine.active_document_gc_telemetry() {
        report("native_gc_ok", gc.successful);
        report("native_gc_errors", gc.failed);
    }
    let mut counts = (0, 0, 0);
    paint_counts(&page.display_list.commands, &mut counts);
    report("paint_commands", page.display_list.commands.len());
    report("text_primitives", counts.0);
    report("image_primitives", counts.1);
    report("other_primitives", counts.2);
    let mut sample = String::new();
    sample_text(&page.display_list.commands, &mut sample);
    report(
        "paint_text_excerpt",
        sample.chars().take(240).collect::<String>(),
    );
    if let Some(bitmap) = bitmap {
        report("phase", "screenshot");
        let nonwhite = snapshot(&page, &bitmap, width, height)?;
        report("nonwhite_pixels", nonwhite);
        report("bitmap", bitmap.display());
    }
    report("status", "completed_diagnostic_only");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("OP_SITE_ERROR: {error}");
        std::process::exit(2);
    }
}
