"""Regression tests for post-run GC counters and memory interpretation."""
import csv
import importlib.util
import tempfile
import unittest
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1] / "analyze_gc_probe.py"
SPEC = importlib.util.spec_from_file_location("analyze_gc_probe", SOURCE)
assert SPEC and SPEC.loader
analyzer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(analyzer)

FIELDS = ["seconds", "ticks", "gc_ok", "gc_errors", "native_gc_ok",
          "native_gc_errors", "native_dom_slots", "rss_bytes",
          "private_commit_bytes", "pause_samples", "host_gc_samples"]


class AnalyzeGcProbeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.csv_path = Path(self.temp.name) / "gc.csv"

    def write(self, rows):
        with self.csv_path.open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=FIELDS)
            writer.writeheader()
            writer.writerows(rows)

    def rows(self):
        return [dict(seconds=str(i), ticks=str(i * 10), gc_ok=str(i),
                     gc_errors="0", native_gc_ok=str(i), native_gc_errors="0",
                     native_dom_slots="15", rss_bytes=str(9000 + i * 5),
                     private_commit_bytes=str(3000 + i * 3),
                     pause_samples=str(i), host_gc_samples=str(i))
                for i in range(12)]

    def test_private_commit_distinct_from_working_set(self):
        rows = self.rows()
        self.write(rows)
        result = analyzer.analyze(self.csv_path, 12, 2)
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(result["private_commit_bytes"]["last"], 3033)
        self.assertEqual(result["working_set_bytes"]["last"], 9055)
        self.assertEqual(result["host_over_16ms"], 0)

    def test_missing_second_fails_closed(self):
        rows = self.rows()
        rows.pop(5)
        self.write(rows)
        with self.assertRaisesRegex(ValueError, "missing, duplicate"):
            analyzer.analyze(self.csv_path, None, 0)

    def test_missing_histogram_pass_fails_closed(self):
        rows = self.rows()
        rows[-1]["pause_samples"] = "0"
        self.write(rows)
        with self.assertRaisesRegex(ValueError, "histogram"):
            analyzer.analyze(self.csv_path, 12, 0)

    def test_non_monotonic_counter_fails_closed(self):
        rows = self.rows()
        rows[7]["native_gc_ok"] = "1"
        self.write(rows)
        with self.assertRaisesRegex(ValueError, "decreasing counter"):
            analyzer.analyze(self.csv_path, 12, 0)

    def test_failed_collector_reports_failure(self):
        rows = self.rows()
        rows[-1]["gc_errors"] = "1"
        self.write(rows)
        self.assertEqual(analyzer.analyze(self.csv_path, 12, 0)["result"], "FAIL")


if __name__ == "__main__":
    unittest.main()
