# Original JavaScript GC stress: repeatable browser-owner probe

## Procedure (Windows 11, 2026-10-13)

Build and execute a synthetic DOM page in the original OPBrowser engine. It
registers a 1 ms interval whose callback creates 80 transient self-referencing
JavaScript objects and updates a persistent global. The native page worker
runs the usual timer loop and browser-owned idle GC checkpoints.

```powershell
cargo build -p op_engine --release --bin gc_stress_probe
target\release\gc_stress_probe.exe --seconds 1800 > gc-stress-30min.csv
```

The binary records one CSV sample per second: elapsed seconds, native page
ticks, successful/failed GC passes, reserved and live object/environment slots,
process working set (Windows K32GetProcessMemoryInfo; Linux /proc/self/status),
last GC pause and maximum GC pause (microseconds). Exit is nonzero if the
synthetic workload did not collect anything or a GC attempt failed. It does
**not** automatically assert that a memory plateau exists.

## Initial debug-build 12-second measurement

Executed `target\debug\gc_stress_probe.exe --seconds 12` on the Windows
workspace after implementing G3 AbortSignal.any ownership. This short run
checks the instrumentation and callback pipeline; it does NOT prove a
30-minute plateau or representative browsing performance.

| Sample | GC successes | Failures | Reserved objects | Reserved envs | Process working set |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 s | 0 | 0 | 171 | 1 | 10,559,488 bytes |
| 3 s | 206 | 0 | 414 | 247 | 10,735,616 bytes |
| 6 s | 407 | 0 | 414 | 247 | 10,809,344 bytes |
| 9 s | 610 | 0 | 414 | 247 | 10,833,920 bytes |
| 11 s | 748 | 0 | 414 | 247 | 10,833,920 bytes |

Final (12 s): **7,288 worker ticks, 817 successful GC passes, 0 failures**;
reserved arenas **414 object / 247 environment slots**; worst observed
stop-the-world GC pause **1,164 µs**. Repeated short-run samples show bounded
reserved arenas and a nearly flat working set after initial warmup.

### Exit criteria still open

A proper 1,800-second run on the same build must record the post-warmup
working-set minimum/maximum, growth slope, outliers and pause percentiles.
This is a **synthetic single-page timer workload**, not real-world navigation,
DOM mutations, concurrent fetch/abort races or multi-tab browsing. It must
be supplemented by those workloads before claiming GC complete.
