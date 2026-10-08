"""Offline contract tests for generated code intelligence reports."""
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import code_intelligence as ci


class CodeIntelligenceTests(unittest.TestCase):
    def test_real_crate_graph_is_deterministic(self):
        found = ci.crates()
        self.assertEqual(len(found), 12)
        graph = ci.graph_report(found)
        self.assertIn("22 local dependency edges", graph)
        self.assertEqual(graph, ci.graph_report(found))

    def test_curated_slices_have_verified_source_anchors(self):
        report = ci.slices_report(ci.crates())
        for marker in ("S1", "S2", "S3", "S4", "S5", "S6"):
            self.assertIn("## " + marker, report)
        self.assertIn("crates/op_browser/src/bin/wpt_probe.rs", report)

    def test_rejects_stale_symbol_and_single_crate_slice(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ("a", "b"):
                folder = root / "crates" / name / "src"
                folder.mkdir(parents=True)
                (folder / "lib.rs").write_text(
                    f"pub fn {name}_step() {{}}\n", encoding="utf-8"
                )
            spec = {
                "schema_version": 1,
                "slices": [{
                    "id": "S1", "title": "Test",
                    "status": "test", "limitation": "Known limitations",
                    "steps": [
                        {"path": "crates/a/src/lib.rs", "symbol": "a_step"},
                        {"path": "crates/b/src/lib.rs", "symbol": "b_step"}
                    ]
                }]
            }
            spec_path = root / "tools" / "code_slices.json"
            spec_path.parent.mkdir()
            spec_path.write_text(json.dumps(spec), encoding="utf-8")
            found = {"a": None, "b": None}
            with patch.object(ci, "ROOT", root):
                self.assertIn("a::a_step", ci.slices_report(found))
                spec["slices"][0]["steps"][1]["symbol"] = "does_not_exist"
                spec_path.write_text(json.dumps(spec), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "Stale slice anchor"):
                    ci.slices_report(found)
                spec["slices"][0]["steps"][1] = {
                    "path": "crates/a/src/lib.rs", "symbol": "a_step"
                }
                spec_path.write_text(json.dumps(spec), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "two crates"):
                    ci.slices_report(found)

    def test_local_wiki_references_resolve(self):
        self.assertEqual(ci.wiki_link_errors(), [])


if __name__ == "__main__":
    unittest.main()
