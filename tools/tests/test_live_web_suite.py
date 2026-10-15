"""Regression checks for the opt-in, real-network browser compatibility runner."""
import json
import tempfile
import unittest
from pathlib import Path
from tools import live_web_suite


class LiveWebSuiteTests(unittest.TestCase):
    def test_manifest_has_named_user_sites_and_unique_real_urls(self):
        sites = live_web_suite.read_manifest(Path("tests/live_web_sites.json"))
        names = {site["id"] for site in sites}
        self.assertGreaterEqual(len(sites), 15)
        self.assertTrue({"google", "youtube", "pikabu", "reddit",
                         "deviantart"}.issubset(names))
        self.assertEqual(len({site["url"] for site in sites}), len(sites))

    def test_url_manifest_rejects_credentials(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "site.json"
            path.write_text(json.dumps({"version": 1, "sites": [
                {"id": "site", "url": "https://admin:password@example.org/",
                 "category": "unsafe"}]}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "anonymous HTTPS"):
                live_web_suite.read_manifest(path)

    def test_completed_render_is_not_a_web_compatibility_pass(self):
        self.assertEqual(live_web_suite.classify({
            "status": "completed_diagnostic_only", "paint_commands": "200",
            "script_failed": "0", "nonwhite_pixels": "5000",
            "text_primitives": "10"}, 0, False), "rendered_unverified")

    def test_blank_page_wins_over_javascript_error(self):
        self.assertEqual(live_web_suite.classify({
            "status": "completed_diagnostic_only", "paint_commands": "1",
            "script_failed": "15", "nonwhite_pixels": "0",
            "text_primitives": "0"}, 0, False), "blank_viewport")

    def test_skeleton_with_no_text_is_reported(self):
        self.assertEqual(live_web_suite.classify({
            "status": "completed_diagnostic_only", "paint_commands": "200",
            "script_failed": "5", "nonwhite_pixels": "60000",
            "text_primitives": "0"}, 0, False), "no_text_rendered")

    def test_runtime_errors_and_timeouts_are_not_passes(self):
        self.assertEqual(live_web_suite.classify({"navigate_error": "HTTP 403"},
                                                  2, False), "navigation_error")
        self.assertEqual(live_web_suite.classify({}, None, True), "timeout")
        self.assertEqual(live_web_suite.classify({}, 2, False), "probe_or_render_error")

    def test_diagnostic_markdown_does_not_claim_site_compatibility(self):
        out = live_web_suite.markdown({"timestamp_utc": "2026-10-15T00:00:00Z",
             "timeout_seconds": 20, "results": [{
                 "id": "reddit", "outcome": "blank_viewport",
                 "metrics": {"load_ms": "130", "script_failed": "1",
                             "paint_commands": "1"}, "screenshot": "reddit.png"}]})
        self.assertIn("blank_viewport", out)
        self.assertIn("not proof", out)
        self.assertIn("reddit.png", out)


if __name__ == "__main__":
    unittest.main()
