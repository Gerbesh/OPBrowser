//! Repeatable synthetic browser GC load; no Chromium or external renderer.
//! Run: cargo run -p op_engine --bin gc_stress_probe --release -- --seconds 1800
use op_engine::Engine;
use std::time::{Duration, Instant};

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
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seconds" => {
                seconds = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .filter(|s| (1..=3600).contains(s))
                    .expect("--seconds must be an integer in 1..=3600");
            }
            _ => panic!("usage: gc_stress_probe [--seconds 1..3600]"),
        }
    }
    let mut engine = Engine::new();
    engine.set_html_page(
        r#"<div id="display">GC synthetic timer page</div>
          <script>
          var count=0;
          var persistent={marker:42};
          setInterval(function(){
              for(var i=0;i<80;i=i+1) {
                  var transient={index:i};
                  transient.self=transient;
              }
              count=count+1;
          },1);
          </script>"#,
        640,
        480,
    );
    let report = engine.active_script_report().expect("active page");
    assert_eq!(report.failed, 0, "stress page JS must compile");
    let start = Instant::now();
    let stop = start + Duration::from_secs(seconds);
    let mut next_sample = start;
    let mut ticks = 0u64;
    println!(
        "seconds,ticks,gc_ok,gc_errors,objects_reserved,envs_reserved,objects_live,envs_live,rss_bytes,last_pause_us,max_pause_us"
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
            println!(
                "{},{},{},{},{},{},{},{},{},{},{}",
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
                telemetry.max_pause.as_micros()
            );
            next_sample += Duration::from_secs(1);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let telemetry = engine
        .active_script_gc_telemetry()
        .expect("active scripted page");
    eprintln!(
        "PROBE_RESULT duration_s={} ticks={} collections={} failures={} reserved_objects={} reserved_envs={} max_pause_us={}",
        seconds,
        ticks,
        telemetry.successful,
        telemetry.failed,
        telemetry.heap.object_slots,
        telemetry.heap.environment_slots,
        telemetry.max_pause.as_micros()
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
}
