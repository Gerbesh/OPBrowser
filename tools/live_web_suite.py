"""Run opt-in public URLs using the original OPBrowser engine, not Chromium."""
import argparse
import concurrent.futures
import datetime as dt
import json
import random
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import urlsplit


def read_manifest(path):
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("version") != 1 or not data.get("sites"):
        raise ValueError("unsupported or empty manifest")
    sites = data["sites"]
    ids = [s["id"] for s in sites]
    if len(set(ids)) != len(ids):
        raise ValueError("duplicate site ID")
    for site in sites:
        if not site["id"].replace("_", "").isalnum():
            raise ValueError("unsafe site ID")
        parsed = urlsplit(site["url"])
        if parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password:
            raise ValueError("all sites require anonymous HTTPS")
    return sites


def classify(metrics, returncode, timed_out):
    if timed_out:
        return "timeout"
    if "navigate_error" in metrics:
        return "navigation_error"
    if returncode != 0:
        return "probe_or_render_error"
    if metrics.get("status") != "completed_diagnostic_only":
        return "incomplete"
    if int(metrics.get("paint_commands", 0)) <= 1 or metrics.get("nonwhite_pixels") == "0":
        return "blank_viewport"
    if int(metrics.get("text_primitives", 0)) == 0:
        return "no_text_rendered"
    if int(metrics.get("script_failed", 0)) > 0:
        return "rendered_with_js_errors"
    return "rendered_unverified"


def probe(site, binary, output, timeout, settle_ms):
    bmp = output / (site["id"] + ".bmp")
    png = output / (site["id"] + ".png")
    argv = [str(binary.resolve()), "--url", site["url"], "--bitmap",
            str(bmp.resolve()), "--settle-ms", str(settle_ms)]
    started = time.monotonic()
    timed_out, stdout, stderr, returncode = False, "", "", None
    try:
        p = subprocess.run(argv, capture_output=True, text=True, encoding="utf-8",
                           errors="replace", timeout=timeout, stdin=subprocess.DEVNULL)
        stdout, stderr, returncode = p.stdout, p.stderr, p.returncode
    except subprocess.TimeoutExpired as exc:
        timed_out = True
        stdout = exc.stdout.decode("utf-8", "replace") if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        stderr = exc.stderr.decode("utf-8", "replace") if isinstance(exc.stderr, bytes) else (exc.stderr or "")
    metrics = {}
    for line in stdout.splitlines():
        parts = line.split("\t", 2)
        if len(parts) == 3 and parts[0] == "OP_SITE":
            metrics[parts[1]] = parts[2]
    screenshot = None
    if bmp.is_file():
        try:
            from PIL import Image
            with Image.open(bmp) as image:
                image.convert("RGB").save(png, optimize=True)
            bmp.unlink()
            screenshot = png.name
        except (ImportError, OSError):
            screenshot = bmp.name
    return dict(id=site["id"], category=site["category"], url=site["url"],
                outcome=classify(metrics, returncode, timed_out),
                elapsed_seconds=round(time.monotonic() - started, 2),
                return_code=returncode, timed_out=timed_out, metrics=metrics,
                screenshot=screenshot, stderr_tail=stderr.strip()[-800:])


def markdown(report):
    lines = [
        "# OPBrowser real-web compatibility baseline", "",
        "Run: " + report["timestamp_utc"] + ". Original Rust engine, not Chromium.",
        "Timeout per site: " + str(report["timeout_seconds"]) + "s.",
        "These are live changing sites. A display list is not proof of web compatibility.",
        "",
        "| Site | Outcome | Load ms | JS failed | Paint commands | Screenshot |",
        "| --- | --- | ---: | ---: | ---: | --- |",
    ]
    for item in report["results"]:
        m = item["metrics"]
        image = "[Image](" + item["screenshot"] + ")" if item["screenshot"] else "-"
        lines.append("| {} | {} | {} | {} | {} | {} |".format(
            item["id"], item["outcome"], m.get("load_ms", "-"),
            m.get("script_failed", "-"), m.get("paint_commands", "-"), image))
    lines.extend(["", "## Limits", "",
                  "blank_viewport: white or effectively empty output, even if navigation succeeded.",
        "no_text_rendered: only skeleton/decorative paint commands; no text primitives.",
        "rendered_with_js_errors: page has pixels, but one or more JS callbacks failed.",
        "rendered_unverified: does not imply a correct layout, interactivity, or JS support.",
        "A rendered page may be incorrect or non-interactive; comparison with a reference browser is still needed.",
                  "HTTP blocks, bot challenges and geo failures are recorded, not bypassed.",
                  "No credentials, authenticated cookies, anti-bot evasion or raw HTML archives.",
                  "Detailed metrics and error messages: results.json.", ""])
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=Path("tests/live_web_sites.json"))
    parser.add_argument("--binary", type=Path, default=Path("target/release/live_site_probe.exe"))
    parser.add_argument("--output", type=Path, default=Path("artifacts/live_web/latest"))
    parser.add_argument("--only", default="")
    parser.add_argument("--max-sites", type=int, default=0)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--timeout-seconds", type=float, default=25)
    parser.add_argument("--settle-ms", type=int, default=1300)
    parser.add_argument("--shuffle-seed", type=int)
    args = parser.parse_args()
    sites = read_manifest(args.manifest)
    selected = set(filter(None, args.only.split(",")))
    if selected:
        unknown = selected - {s["id"] for s in sites}
        if unknown:
            parser.error("unknown site ID: " + ",".join(sorted(unknown)))
        sites = [s for s in sites if s["id"] in selected]
    if args.shuffle_seed is not None:
        random.Random(args.shuffle_seed).shuffle(sites)
    if args.max_sites > 0:
        sites = sites[:args.max_sites]
    if not sites or not 1 <= args.workers <= 4 or not 1 <= args.timeout_seconds <= 120 or not 0 <= args.settle_ms <= 10000:
        parser.error("invalid run limits")
    if not args.binary.is_file():
        parser.error("missing release binary; build op_browser --bin live_site_probe first")
    args.output.mkdir(parents=True, exist_ok=True)
    result = dict(timestamp_utc=dt.datetime.now(dt.timezone.utc).isoformat(),
                  timeout_seconds=args.timeout_seconds, settle_ms=args.settle_ms,
                  shuffle_seed=args.shuffle_seed, manifest=str(args.manifest),
                  binary=str(args.binary), results=[])
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(probe, s, args.binary, args.output,
                                   args.timeout_seconds, args.settle_ms) for s in sites]
        for finished in concurrent.futures.as_completed(futures):
            item = finished.result()
            result["results"].append(item)
            (args.output / "results.json").write_text(
                json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            (args.output / "REPORT.md").write_text(markdown(result), encoding="utf-8")
            print(f"{len(result['results'])}/{len(sites)} {item['id']} "
                  f"{item['outcome']} {item['elapsed_seconds']}s", flush=True)
    print("Saved", args.output / "REPORT.md", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
