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

## G3 weak-timeout change: 5-second native-worker sanity probe

Ran `cargo run -p op_engine --bin gc_stress_probe -- --seconds 5`
after weakening orphan AbortSignal.timeout deadline ownership. Observed
3,269 worker ticks, **356 successful collections, zero failed passes**,
414 reserved object slots, 247 reserved environment slots and maximum
GC pause **385 microseconds**. Five sampled working sets ranged from
9,801,728 to 10,035,200 bytes. This validates the synthetic worker
path only and is **not** a plateau proof or comparable throughput benchmark.

## G3 detached-DOM ownership: 12-second native-worker regression run

Executed `cargo run -p op_engine --bin gc_stress_probe -- --seconds 12`
after the detached DOM tracing and native-cache cleanup changes.
Observed 7,709 worker ticks, **846 successful collections, 0 failures**,
and reserved arenas of **414 object / 247 environment slots**. Maximum
reported GC pause was **493 microseconds**. Sampled working set ranged from
9,830,400 bytes at start to a maximum of 10,137,600 bytes during the trial.
An independent 5-second run observed 354 successful collections and zero
failures, but briefly reserved 475/329 slots. Short probes can vary by timer
scheduling and are not proof of a 30-minute memory plateau. The 40-cycle DOM
reclamation test is a separate VM unit test, not part of this timer workload.

## G3 native-unwrapped DOM and fetch-root audit: 12-second probe

Executed `cargo run -p op_engine --bin gc_stress_probe -- --seconds 12`
after native descendant retention/cleanup and new fetch GC race tests.
Observed 7,402 worker ticks, **825 successful GC passes, zero failures**,
reserved arenas of **414 object / 247 environment slots**, and maximum
reported pause **1,551 microseconds**. Sampled Windows working set ranged
from **9,859,072** bytes at initialization to **10,145,792** bytes at peak.
This run predates the final guard that skips native subtree-map construction
when no DOM wrappers are collectible; the full Rust tests were rerun after
that optimization. A 12-second synthetic timer probe does not demonstrate
a 30-minute memory plateau or concurrent real-network behavior.

## G3 budget-headroom checkpoint: 12-second worker sanity run

After enabling arena-headroom-triggered idle GC, ran
`cargo run -p op_engine --bin gc_stress_probe -- --seconds 12`.
The worker completed 7,582 ticks, **831 successful GC passes, zero
errors**, with 414 reserved object slots and 247 environment slots.
Maximum observed stop-the-world pause was **1,724 microseconds**. Sampled
Windows process working set ranged from 9,846,784 bytes at startup to
10,133,504 bytes by second 11. The synthetic worker exercises timer and
JavaScript allocation churn, not tight configured-budget limits (verified
separately by three VM unit tests). This is not a long-run plateau proof.

### Exit criteria still open

A proper 1,800-second run on the same build must record the post-warmup
working-set minimum/maximum, growth slope, outliers and pause percentiles.
This is a **synthetic single-page timer workload**, not real-world navigation,
DOM mutations, concurrent fetch/abort races or multi-tab browsing. It must
be supplemented by those workloads before claiming GC complete.
