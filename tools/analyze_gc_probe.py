"""Validate GC stress CSV and report Windows private-commit/working-set trends."""
import argparse
import csv
import json
import statistics
from pathlib import Path


def stats(rows, field, warmup):
    vals = [int(r[field]) for r in rows if int(r["seconds"]) >= warmup and int(r[field]) > 0]
    if len(vals) < 2:
        return None
    k = max(1, len(vals) // 5)
    a, b = statistics.median(vals[:k]), statistics.median(vals[-k:])
    mx, my = (len(vals) - 1) / 2, statistics.mean(vals)
    slope = 60 * sum((i - mx) * (v - my) for i, v in enumerate(vals)) / sum((i - mx) ** 2 for i in range(len(vals)))
    return dict(min=min(vals), max=max(vals), first=vals[0], last=vals[-1],
                early_median=a, late_median=b, median_change=b-a,
                bytes_per_minute=round(slope, 2))


def analyze(path, expected, warmup):
    with path.open(encoding="utf-8-sig", newline="") as f:
        rows = list(csv.DictReader(f))
    if not rows or [int(r["seconds"]) for r in rows] != list(range(len(rows))):
        raise ValueError("missing, duplicate, or unordered per-second samples")
    if expected is not None and len(rows) != expected:
        raise ValueError(f"expected {expected} seconds; found {len(rows)}")
    for name in ("ticks", "gc_ok", "native_gc_ok", "host_gc_samples"):
        if name not in rows[0]:
            continue
        nums = [int(r[name]) for r in rows]
        if any(b < a for a, b in zip(nums, nums[1:])):
            raise ValueError(f"decreasing counter: {name}")
    if "pause_samples" in rows[0] and any(int(r["pause_samples"]) != int(r["gc_ok"]) for r in rows):
        raise ValueError("GC histogram samples differ from successful collections")
    last = rows[-1]
    errors = int(last["gc_errors"]) + int(last["native_gc_errors"])
    return dict(result="PASS" if errors == 0 else "FAIL", seconds=len(rows),
                last_ticks=int(last["ticks"]), last_js_gc=int(last["gc_ok"]),
                last_native_gc=int(last["native_gc_ok"]), last_gc_errors=errors,
                last_native_slots=int(last["native_dom_slots"]),
                max_js_pause_us=int(last.get("max_pause_us", 0)),
                max_host_pause_us=int(last.get("host_max_us", 0)),
                host_over_16ms=int(last.get("host_over_16ms", 0)),
                host_over_50ms=int(last.get("host_over_50ms", 0)),
                working_set_bytes=stats(rows, "rss_bytes", warmup),
                private_commit_bytes=stats(rows, "private_commit_bytes", warmup)
                if "private_commit_bytes" in last else None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path)
    parser.add_argument("--expected-seconds", type=int)
    parser.add_argument("--warmup", type=int, default=60)
    args = parser.parse_args()
    summary = analyze(args.csv, args.expected_seconds, args.warmup)
    print(json.dumps(summary, indent=2))
    return 0 if summary["result"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
