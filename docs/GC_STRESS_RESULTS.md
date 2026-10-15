# Original JavaScript GC stress: repeatable browser-owner probe

## 30-minute Windows Release GC with complete host pauses and private commit (2026-10-15)

The completed `gc_stress_probe --seconds 1800 --workload dom` captured
**1,800 one-second samples** on Windows 11, Rust 1.99.0. This is a
single-page synthetic DOM/JavaScript workload. It measures Windows
`PrivateUsage` (private committed bytes) separately from the OS-trimmable
`WorkingSetSize`, plus whole post-parser host GC checkpoint wall time.

| Measurement | Completed run |
| --- | ---: |
| Page task ticks | 1,225,904 |
| Successful JS / native GC sweeps | 83,777 / 83,777 |
| Full post-parser host GC checkpoints | 83,776 |
| GC/JavaScript callback errors | 0 |
| Reclaimed native DOM nodes | 335,126 |
| Native arena reserved / live at end | 19 / 13 slots |
| JS reserved object / environment slots at end | 619 / 333 |
| JS pause histogram p50 / p95 / p99 bucket upper bounds | 132 / 132 / 263 us |
| Worst whole-host GC checkpoint | **448,180 us (448.18 ms)** |
| Host pauses >=16 ms / >=50 ms | **6 / 2** |
| Worst host pause: JS / native / VM metadata / page resources | 448,158 / 14 / 6 / under 1 us |
| Windows Private Commit min / max (after 60s warmup) | 2,150,400 / 2,473,984 bytes |
| Private Commit first / last fifth medians (after warmup) | 2,334,720 / 2,404,352 bytes |
| Change in Private Commit fifth medians | +69,632 bytes |
| Private Commit least-squares trend after warmup | +3,695 bytes/min |
| Working Set first / last fifth medians | 9,015,296 / 9,199,616 bytes |

**Assessment:** PASS for 30-minute synthetic correctness and bounded native
arena reuse. Process private commit is roughly flat after warmup, with
no obvious runaway growth; a small upward drift remains and cannot prove
absence of all leaks. The **tail-latency gate is NOT passed**: two
host checkpoints exceeded 50 ms; the maximum 448 ms occurred around
second 152. The recorded inner VM breakdown on the worst JS pause
(mark 107,841 us; metadata 213,437 us; sweep 83 us) does not account
for the entire 448 ms wall time. OS scheduling/preemption and unmeasured
boundary overhead remain plausible, not confirmed. Other pauses over
16 ms appeared around seconds 82-83, 836 and 1278. Real-site navigation,
multi-tab memory, long HTTP/Promise churn, CPU-time attribution and
repeatability under different scheduling conditions remain open.

Reproduce using `target/release/gc_stress_probe.exe --seconds 1800
--workload dom`; analyze with `python tools/analyze_gc_probe.py
docs/gc/2026-10-15-windows-dom-host-private-1800s.csv --expected-seconds 1800`.

Evidence: [raw CSV](gc/2026-10-15-windows-dom-host-private-1800s.csv),
[stderr final counters](gc/2026-10-15-windows-dom-host-private-1800s.log),
[SHA-256 and run manifest](gc/2026-10-15-windows-dom-host-private-1800s-manifest.json).

## Windows complete host GC checkpoint: 120-second Release DOM run (2026-10-15)

The browser page host now measures one complete post-callback GC checkpoint
from the JS trigger through native `Document` reclaim, VM native metadata
cleanup and pruning of weak image/stylesheet/scroll resources. Parser-internal
GC is deliberately excluded from the per-page host counter. Windows 11
Release run: `gc_stress_probe --seconds 120 --workload dom` with all 120
one-second CSV samples.

| Counter | Measured result |
| --- | ---: |
| Worker ticks | 81,549 |
| Automatic JS / native sweeps | 5,596 / 5,596 |
| Complete **post-parser host GC checkpoints** | 5,595 |
| JavaScript, native-GC or callback errors | 0 |
| Reclaimed native DOM nodes | 22,380 |
| Reserved / live native DOM slots at end | 15 / 13 |
| Max complete host GC pause | **368 microseconds** |
| Same worst host checkpoint: JS / native / metadata / page resources | 360 / 4 / 2 / 0 microseconds |
| Host checkpoints >=16 ms / >=50 ms | 0 / 0 |
| JS GC p50 / p95 / p99 bucket upper bounds | 132 / 132 / 263 microseconds |
| Working set min / max / last sample | 8,282,112 / 8,970,240 / 8,916,992 bytes |

The final counters include the parser's initial JS/native collection, while
per-page host instrumentation starts when the event loop is active. For each
CSV sample, `host_gc_samples == native_gc_ok - 1` held. Max wall times of
the independent JS/native collectors need not come from the worst complete
host checkpoint. All wall times may include OS scheduling. This synthetic
2-minute profile does not explain or disprove the historical 273 ms spike;
real HTTP navigation, multi-tab workloads and private committed-memory
measurement remain open.

A deterministic engine regression injects an old-generation completion whose
request ID matches a new page's pending `fetch`. The old response is rejected,
the new-generation response fulfills the Promise, and no callback errors occur.
A separate test checks that complete host GC telemetry includes phase timing
and resets when a new page is loaded.

Raw [CSV](gc/2026-10-15-windows-host-checkpoint-dom-120s.csv),
[log](gc/2026-10-15-windows-host-checkpoint-dom-120s.log) and
[reproduction manifest](gc/2026-10-15-windows-host-checkpoint-dom-120s-manifest.json).

## Windows phase-aware GC regression: 120-second Release DOM workload (2026-10-15)

After instrumenting `op_js` collection phases and adding a composite-fetch GC
regression, an optimized Windows 11 `gc_stress_probe --seconds 120 --workload dom`
ran to completion on the browser-owned worker. **82,593 ticks, 5,588 JS
collections and 5,588 native collections; no GC, callback or root failures.**
Native sweep reclaimed **22,350** nodes, retained 17 reserved/13 live native
slots, and JS arenas reserved 508 object/250 environment slots at exit.
All 5,588 automatic pause samples matched successful collections.

| Result | Measured value |
| --- | ---: |
| JS GC p50 / p95 / p99 bucket upper bounds | 132 / 132 / 263 us |
| Worst JS GC wall pause | 4,671 us |
| Breakdown of that same worst pass: mark / metadata / sweep | 4,353 / 22 / 293 us |
| Pauses >=16 ms / >=50 ms | 0 / 0 |
| Worst native GC wall pause | 154 us |
| Working set min / max / last sample | 8,278,016 / 9,023,488 / 8,998,912 bytes |
| Native reserved slots observed | 11..17 |

Mark/metadata/sweep timings are wall-clock attribution for the *single worst
successful automatic JS GC*; they do not exclude OS scheduling and do not
include the later native collection or host metadata/cache cleanup. This run
did not reproduce the historical 273,375-us outlier from the 30-minute run,
so the latter's root cause is **not yet established**. A test also covers a
fetch retained via `AbortSignal.any` over 16 intervening JS sweeps, abort
delivery, eventual cleanup and ignoring a stale native reply.

Raw [CSV](gc/2026-10-15-windows-dom-gc-phases-120s.csv),
[log](gc/2026-10-15-windows-dom-gc-phases-120s.log) and
[provenance manifest](gc/2026-10-15-windows-dom-gc-phases-120s-manifest.json).

## 30-minute Windows Release DOM/JS GC gate (2026-10-14)

On source commit `c281d39`, the optimized `gc_stress_probe --seconds 1800
--workload dom` completed **1,161,484 worker ticks, 80,939 JS GC and
80,939 native GC passes, zero GC/JS callback errors**, and reclaimed
**323,854 native DOM nodes**. The 1,800 CSV samples cover seconds 0..1799.
Native DOM arena reserved slots grew only from 11 to **21** while live
nodes finished at 13; JS reserved objects/environments finished at 730/416.
All 80,939 automatic JS pause samples match successful collections.

| Diagnostic | Result |
| --- | ---: |
| JS GC p50 / p95 / p99 bucket upper bounds | 132 / 132 / 525 us |
| Worst JS / native mark-sweep wall pause | 273,375 / 1,001 us |
| Process working set, whole run min / max | 3,981,312 / 10,457,088 bytes |
| Post-warmup (60..1799) RSS slope | -4,072.14 bytes/s |
| First ten minutes (60..599) RSS slope | +218.83 bytes/s |
| GC or callback failures | 0 |

Windows trimmed the process working set in the last third of the run;
negative RSS slope does not imply the collector alone reclaimed that memory.
No sustained process working-set rise was observed. The native arena did
not leak under this workload. Rare **273 ms JS pause outlier** requires
targeted profiling; elapsed wall-clock pauses include OS scheduling.
Only this synthetic single-page DOM workload passes the 30-minute gate.
Private committed bytes, whole host checkpoint timing, fetch/Promise
and real-page/multi-tab browsing remain unverified.

Raw [1,800-row Windows CSV](gc/2026-10-14-windows-native-dom-1800s.csv),
[final log](gc/2026-10-14-windows-native-dom-1800s.log) and
[source/binary manifest](gc/2026-10-14-windows-native-dom-1800s-manifest.json).

## Native Windows DOM regression: 15-second debug run (2026-10-14)

Validated current upstream commit `6e00f76` on Windows 11 with
`cargo run -q -p op_engine --bin gc_stress_probe -- --seconds 15 --workload dom`.
Full workspace Rust tests, strict Clippy, browser startup smoke, and code
intelligence passed on this Windows environment. The real page worker
performed **9,941 ticks and 702 JavaScript GC passes** with zero GC or
callback errors. Native GC completed 702 passes and reclaimed **2,804
nodes**, with **15 native reserved slots / 13 live** at completion.
JS reserved arenas were 397 objects / 167 environments. The measured
JS pause histogram upper bounds were p50=263, p95=525, p99=525 us;
maximum JS/native GC pauses were 977/143 us.
The 14-second RSS sample was 10,407,936 bytes vs 9,981,952 bytes
at startup. This is a short debug-build diagnostic, **not** a 30-minute
Windows plateau or representative site compatibility test.
Raw [CSV](gc/2026-10-14-windows-native-dom-15s.csv),
[log](gc/2026-10-14-windows-native-dom-15s.log) and
[build/source manifest](gc/2026-10-14-windows-native-dom-manifest.json)
preserve provenance and exact counters.

## Native DOM arena sweep: 180-second Linux profile (2026-10-14)

Native Document now releases orphan strings/attributes/tree links and reuses
slots with checked generations. Connected nodes and complete components owned
by live JS nodes/views/queries/callbacks survive. Native GC waits until parsing
is finished and the JS root pass succeeds; native allocation pressure can
force a validated checkpoint. Text replacement also clears old child roots.
Weak page image/stylesheet/scroll resources drop dead node identities.

On the final release build, `--seconds 180 --workload dom` completed with zero
JS, JS-GC and native-GC failures. Source is the accompanying commit based on
`17439ec`; [manifest](gc/2026-10-14-linux-native-dom-manifest.json) pins source
file and binary hashes. The shared cloud executor also ran cross-Clippy and
documentation checks; timing is diagnostic rather than an isolated benchmark.

| Counter | Result |
| --- | ---: |
| JS collections / histogram samples | 8,385 / 8,385 |
| Native collections / reclaimed nodes | 8,385 / 33,544 |
| Final JS reserved objects / environments | 508 / 250 |
| Final native reserved slots / live nodes | 17 / 11 |
| Native slots, samples 10..179 | 15..17 |
| Native live nodes, samples 10..179 | 11..13 |
| RSS min / max, samples 10..179 | 3,526,656 / 3,616,768 bytes |
| RSS OLS slope, samples 10..179 | 537.96 bytes/s |
| JS p50 / p95 / p99 upper bounds | 66 / 263 / 525 us |
| Maximum JS / native mark-sweep wall pause | 2,691 / 1,565 us |

Raw [CSV](gc/2026-10-14-linux-native-dom-180s.csv) and
[final log](gc/2026-10-14-linux-native-dom-180s.log). More than 20,000 native
nodes were reclaimed without exhausting the host arena. This verifies bounded native arena reuse for the specified workload.
The 30-minute Windows and real-site gates remain open.
Native timings exclude the later VM metadata and page-resource pruning;
total host checkpoint pause distribution remains to be measured.

Four native arena tests and six VM/engine ownership tests cover stale-handle
rejection, invalid-root atomicity, generation retirement, repeated reuse,
retained query trees/resources, parser ownership, timer roots, native pressure,
text replacement churn and wrapper tracing across unwrapped native nodes.
All 771 executable portable Rust tests pass (five previously verified
Windows-dependent cases excluded on Linux); Windows-target strict workspace
Clippy and code intelligence pass. Native Win32 and compatibility validation
run in Windows CI after publication. Larger live heaps, oversized single tasks,
fetch/Promise/navigation matrices and sustained Windows memory remain open.

## G3 host-boundary fixes and 60-second Linux profiles (2026-10-14)

The DOM profile initially failed within two seconds: native binding re-rooted
detached wrappers and the page exhausted its 256 lifetime synthetic-node
quota. Binding now preserves actual connectivity and canonical parent/child
IDs; the shared element/text allocator counts outstanding unbound nodes,
retains the 256-operation guard and never reuses synthetic identities.
Three VM regressions and both embedded workload identity checks pass.

The probe now records **every successful automatic GC pause** in a fixed-size
histogram, including collections between CSV samples. p50/p95/p99 are
nearest-rank power-of-two nanosecond bucket **upper bounds**, rounded upward
to microseconds for output. Failed attempts and explicit collections are
excluded. `pause_samples` must equal successful automatic collections.
`native_dom_slots` separately counts authoritative `Document` nodes, including
detached nodes. `rss_bytes=0` means the platform measurement is unavailable.

Rust 1.99.0, release, Linux x86_64, two independent processes in a shared cloud
executor. The profiles and Cargo checks ran concurrently; wall-clock pauses
include scheduling delays, so these are diagnostic results, not isolated
performance benchmarks. Historical source is `17439ec`, based on `7f20c5e`. The [manifest](gc/2026-10-14-linux-manifest.json) includes the binary
hash, method, raw CSV paths and final counters from stderr.

| Final 60-second counter | Timer | DOM/listeners |
| --- | ---: | ---: |
| Successful GC / pause samples | 4,169 | 2,678 |
| GC failures / workload JS failures | 0 / 0 | 0 / 0 |
| Reserved objects / environments | 556 / 411 | 508 / 250 |
| Native DOM slots, start -> finish | 9 -> 9 | 11 -> 10,729 |
| p50 / p95 / p99 upper bounds | 66 / 132 / 525 us | 66 / 263 / 525 us |
| Maximum wall-clock pause | 131,519 us | 9,994 us |
| RSS min / max, samples 10..59 | 3,371,008 / 3,371,008 bytes | 3,837,952 / 5,419,008 bytes |
| RSS OLS growth, samples 10..59 | 0 bytes/s | 33,310.66 bytes/s |

Raw [timer CSV](gc/2026-10-14-linux-timer-60s.csv) and
[DOM CSV](gc/2026-10-14-linux-dom-60s.csv), with separate final
[timer log](gc/2026-10-14-linux-timer-60s.log) and
[DOM log](gc/2026-10-14-linux-dom-60s.log). CSV samples end at second 59;
final counters cover the full requested 60 seconds.

**Blocker at 17439ec (now addressed by native sweeping):** native `op_dom::Document` retained detached nodes.
DOM RSS and native slot counts grow despite recycled JS arenas. The host
creation limit is 20,000 native slots; the DOM probe stops with a failure if
that limit is reached. A short successful run does not assert a plateau.
Native storage reclamation, bounded pause outliers and the 30-minute Windows
gate remain outstanding, as do real-network/navigation workloads.

Reproduce on Windows:

```powershell
cargo build -p op_engine --release --bin gc_stress_probe
target\release\gc_stress_probe.exe --seconds 1800 --workload timer > gc-timer.csv
target\release\gc_stress_probe.exe --seconds 1800 --workload dom > gc-dom.csv
```

On Linux, override the repository's Windows toolchain with
`cargo +stable-x86_64-unknown-linux-gnu` and run
`target/release/gc_stress_probe` with the same arguments. The DOM profile
includes cyclic JS objects, orphan node/listener closures and a retained
detached subtree/listener checked and edited after successive sweeps.
It does not perform fetch or navigation.

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
