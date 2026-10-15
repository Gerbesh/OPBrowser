//! Repeatable synthetic browser GC load; no Chromium or external renderer.
//! Run: cargo run -p op_engine --bin gc_stress_probe --release -- --seconds 1800
use op_engine::Engine;
use std::time::{Duration, Instant};

const TIMER_PAGE: &str = r#"<body><div>GC synthetic timer page</div>
    <script>
    var count=0;
    var persistent={marker:42,count:0};
    var identity=persistent;
    setInterval(function(){
        if(persistent!==identity || persistent.marker!==42 || persistent.count!==count)
            throw new Error('persistent timer identity lost');
        for(var i=0;i<80;i=i+1) {
            var transient={index:i};
            transient.self=transient;
        }
        count=count+1;
        persistent.count=count;
    },1);
    </script></body>"#;

const DOM_PAGE: &str = r#"<body><div>GC synthetic detached DOM page</div>
    <script>
    var persistent={marker:42,hits:0};
    var identity=persistent;
    var count=0;
    var retained=document.createElement('div');
    var retainedText=document.createTextNode('retained subtree');
    retained.appendChild(retainedText);
    retained.addEventListener('click',function(){persistent.hits=persistent.hits+1;});
    document.body.appendChild(retained);
    document.body.removeChild(retained);
    setInterval(function(){
        if(persistent!==identity || persistent.marker!==42)
            throw new Error('persistent DOM workload identity lost');
        if(retained.firstChild!==retainedText || retainedText.parentNode!==retained)
            throw new Error('retained detached subtree identity lost');
        retained.removeChild(retainedText);
        retained.appendChild(retainedText);
        retained.click();
        count=count+1;
        if(persistent.hits!==count) throw new Error('retained DOM listener lost');
        var node=document.createElement('div');
        var text=document.createTextNode('detached GC workload');
        node.appendChild(text);
        node.addEventListener('click',function(){
            if(node.firstChild!==text) throw new Error('detached subtree identity lost');
        });
        document.body.appendChild(node);
        document.body.removeChild(node);
        for(var i=0;i<80;i=i+1) {
            var transient={index:i};
            transient.self=transient;
        }
    },10);
    </script></body>"#;

fn percentile_us(telemetry: &op_js::JsGcTelemetry, percentile: u8) -> u128 {
    telemetry
        .pause_histogram
        .percentile_upper_bound(percentile)
        .map_or(0, |pause| pause.as_nanos().div_ceil(1000))
}

#[cfg(windows)]
fn process_rss_bytes() -> Option<u64> {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    // SAFETY: the stack buffer matches the C structure and the pseudo-handle
    // refers to this process for the duration of the call.
    unsafe {
        let mut info: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        info.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut info, info.cb) == 0 {
            None
        } else {
            Some(info.WorkingSetSize as u64)
        }
    }
}

#[cfg(target_os = "linux")]
fn process_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let rest = line.strip_prefix("VmRSS:")?;
        rest.split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
            .map(|kb| kb * 1024)
    })
}

#[cfg(not(any(windows, target_os = "linux")))]
fn process_rss_bytes() -> Option<u64> {
    None
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut seconds = 15u64;
    let mut workload = String::from("timer");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seconds" => {
                seconds = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .filter(|s| (1..=3600).contains(s))
                    .expect("--seconds must be an integer in 1..=3600");
            }
            "--workload" => {
                workload = args.next().expect("--workload requires timer or dom");
                assert!(
                    matches!(workload.as_str(), "timer" | "dom"),
                    "--workload requires timer or dom"
                );
            }
            _ => panic!("usage: gc_stress_probe [--seconds 1..3600] [--workload timer|dom]"),
        }
    }
    let mut engine = Engine::new();
    engine.set_html_page(
        if workload == "dom" {
            DOM_PAGE
        } else {
            TIMER_PAGE
        },
        640,
        480,
    );
    let report = engine.active_script_report().expect("active page");
    assert_eq!(report.failed, 0, "stress page JS must compile");
    let parser_native_collections = engine
        .active_document_gc_telemetry()
        .expect("active page")
        .successful;
    let start = Instant::now();
    let stop = start + Duration::from_secs(seconds);
    let mut next_sample = start;
    let mut ticks = 0u64;
    eprintln!("PROBE_START workload={workload} duration_s={seconds}");
    println!(
        "seconds,ticks,gc_ok,gc_errors,objects_reserved,envs_reserved,objects_live,envs_live,rss_bytes,last_pause_us,max_pause_us,native_dom_slots,pause_samples,p50_upper_us,p95_upper_us,p99_upper_us,native_dom_live,native_gc_ok,native_gc_errors,native_reclaimed,native_last_pause_us,native_max_pause_us,pauses_over_16ms,pauses_over_50ms,worst_mark_us,worst_metadata_us,worst_sweep_us,host_gc_samples,host_last_us,host_max_us,host_over_16ms,host_over_50ms,worst_host_js_us,worst_host_native_us,worst_host_metadata_us,worst_host_resources_us"
    );
    loop {
        let now = Instant::now();
        if now >= stop {
            break;
        }
        let _ = engine.tick_timers(640, 480);
        ticks += 1;
        if now >= next_sample {
            let telemetry = engine
                .active_script_gc_telemetry()
                .expect("active scripted page");
            let native = engine.active_document_gc_telemetry().expect("active page");
            let slowest = telemetry.max_pause_phases.unwrap_or_default();
            let host = engine
                .active_host_gc_checkpoint_telemetry()
                .expect("active page host");
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                now.duration_since(start).as_secs(),
                ticks,
                telemetry.successful,
                telemetry.failed,
                telemetry.heap.object_slots,
                telemetry.heap.environment_slots,
                telemetry.live_objects,
                telemetry.live_environments,
                process_rss_bytes().unwrap_or(0),
                telemetry.last_pause.map_or(0, |p| p.as_micros()),
                telemetry.max_pause.as_micros(),
                engine.active_document_node_slots().expect("active page"),
                telemetry.pause_histogram.samples,
                percentile_us(&telemetry, 50),
                percentile_us(&telemetry, 95),
                percentile_us(&telemetry, 99),
                native.live_nodes,
                native.successful,
                native.failed,
                native.reclaimed,
                native.last_pause.map_or(0, |p| p.as_micros()),
                native.max_pause.as_micros(),
                telemetry.pauses_over_16ms,
                telemetry.pauses_over_50ms,
                slowest.mark.as_micros(),
                slowest.metadata.as_micros(),
                slowest.sweep.as_micros(),
                host.samples,
                host.last_pause.as_micros(),
                host.max_pause.as_micros(),
                host.pauses_over_16ms,
                host.pauses_over_50ms,
                host.worst_phases.js.as_micros(),
                host.worst_phases.native.as_micros(),
                host.worst_phases.metadata.as_micros(),
                host.worst_phases.resources.as_micros()
            );
            assert_eq!(native.failed, 0, "native GC root validation failed");
            assert_eq!(telemetry.failed, 0, "idle GC failed during workload");
            assert_eq!(
                engine.active_script_report().expect("active page").failed,
                0,
                "workload JavaScript failed"
            );
            if workload == "dom" {
                assert!(
                    engine.active_document_node_slots().expect("active page") < 20_000,
                    "native DOM arena reached the host creation limit; JS GC alone does not reclaim it"
                );
            }
            next_sample += Duration::from_secs(1);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let telemetry = engine
        .active_script_gc_telemetry()
        .expect("active scripted page");
    let native = engine.active_document_gc_telemetry().expect("active page");
    let host = engine
        .active_host_gc_checkpoint_telemetry()
        .expect("active page host");
    assert_eq!(native.failed, 0, "native GC root validation failed");
    eprintln!(
        "PROBE_RESULT workload={} duration_s={} ticks={} collections={} failures={} reserved_objects={} reserved_envs={} native_dom_slots={} pause_samples={} p50_upper_us={} p95_upper_us={} p99_upper_us={} max_pause_us={} native_dom_live={} native_gc_ok={} native_gc_errors={} native_reclaimed={} native_max_pause_us={} pauses_over_16ms={} pauses_over_50ms={} worst_mark_us={} worst_metadata_us={} worst_sweep_us={} host_gc_samples={} host_max_us={} host_over_16ms={} host_over_50ms={} worst_host_js_us={} worst_host_native_us={} worst_host_metadata_us={} worst_host_resources_us={}",
        workload,
        seconds,
        ticks,
        telemetry.successful,
        telemetry.failed,
        telemetry.heap.object_slots,
        telemetry.heap.environment_slots,
        engine.active_document_node_slots().expect("active page"),
        telemetry.pause_histogram.samples,
        percentile_us(&telemetry, 50),
        percentile_us(&telemetry, 95),
        percentile_us(&telemetry, 99),
        telemetry.max_pause.as_micros(),
        native.live_nodes,
        native.successful,
        native.failed,
        native.reclaimed,
        native.max_pause.as_micros(),
        telemetry.pauses_over_16ms,
        telemetry.pauses_over_50ms,
        telemetry
            .max_pause_phases
            .unwrap_or_default()
            .mark
            .as_micros(),
        telemetry
            .max_pause_phases
            .unwrap_or_default()
            .metadata
            .as_micros(),
        telemetry
            .max_pause_phases
            .unwrap_or_default()
            .sweep
            .as_micros(),
        host.samples,
        host.max_pause.as_micros(),
        host.pauses_over_16ms,
        host.pauses_over_50ms,
        host.worst_phases.js.as_micros(),
        host.worst_phases.native.as_micros(),
        host.worst_phases.metadata.as_micros(),
        host.worst_phases.resources.as_micros()
    );
    assert_eq!(
        host.samples + parser_native_collections,
        native.successful,
        "every post-parser native sweep in the event-loop workload must have a host checkpoint"
    );
    assert_eq!(
        telemetry.pause_histogram.samples as usize, telemetry.successful,
        "every successful automatic collection must contribute one pause"
    );
    assert!(
        telemetry.successful > 0,
        "synthetic workload failed to exercise automatic GC"
    );
    assert_eq!(
        telemetry.failed, 0,
        "idle GC failed during synthetic workload"
    );
    assert_eq!(
        engine.active_script_report().expect("active page").failed,
        0,
        "JavaScript timer callback failed during synthetic GC workload"
    );
    if workload == "dom" {
        assert!(
            engine.active_document_node_slots().expect("active page") < 20_000,
            "native DOM arena reached the host creation limit"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stress_pages_exercise_gc_and_preserve_live_identity() {
        for page in [TIMER_PAGE, DOM_PAGE] {
            let mut engine = Engine::new();
            engine.set_html_page(page, 640, 480);
            assert_eq!(engine.active_script_report().unwrap().failed, 0);
            let initial = engine.active_script_gc_telemetry().unwrap().successful;
            let stop = Instant::now() + Duration::from_secs(3);
            while Instant::now() < stop
                && engine.active_script_gc_telemetry().unwrap().successful < initial + 2
            {
                let _ = engine.tick_timers(640, 480);
                std::thread::sleep(Duration::from_millis(2));
            }
            assert_eq!(engine.active_script_report().unwrap().failed, 0);
            assert!(engine.active_event_listener_errors().unwrap().is_empty());
            let telemetry = engine.active_script_gc_telemetry().unwrap();
            assert!(
                telemetry.successful >= initial + 2,
                "timer tasks must collect beyond startup GC"
            );
            assert_eq!(telemetry.failed, 0);
            assert_eq!(engine.active_document_gc_telemetry().unwrap().failed, 0);
            assert_eq!(
                telemetry.pause_histogram.samples as usize,
                telemetry.successful
            );
        }
    }
}
