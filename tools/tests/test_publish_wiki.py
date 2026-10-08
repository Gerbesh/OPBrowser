"""GitHub Wiki publisher conversion tests, no remote access required."""
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import publish_wiki


class WikiPublisherTests(unittest.TestCase):
    def test_internal_wiki_links_become_published_page_urls(self):
        out = publish_wiki.convert(
            "[Status](Current-Status.md) [API](Code-Slicer.md#scope)"
        )
        self.assertIn(
            "https://github.com/Gerbesh/OPBrowser/wiki/Current-Status", out
        )
        self.assertIn(
            "https://github.com/Gerbesh/OPBrowser/wiki/Code-Slicer#scope", out
        )
        self.assertNotIn("Current-Status.md", out)

    def test_repo_docs_point_back_to_main(self):
        self.assertEqual(
            publish_wiki.convert("[Docs](../docs/COMPATIBILITY.md)"),
            "[Docs](https://github.com/Gerbesh/OPBrowser/blob/main/docs/COMPATIBILITY.md)",
        )

    def test_does_not_change_external_links_or_images(self):
        original = ("[CSS](https://www.w3.org/TR/css-color-4/) "
                    "![image](https://example.com/image.png)")
        self.assertEqual(publish_wiki.convert(original), original)

    def test_missing_page_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "Missing wiki page"):
            publish_wiki.convert("[Missing](No-Such-Wiki-Page.md)")

    def test_all_pages_can_be_rendered(self):
        source = publish_wiki.render()
        self.assertGreaterEqual(len(source), 23)
        self.assertIn("_Sidebar.md", source)
        self.assertIn(
            "https://github.com/Gerbesh/OPBrowser/wiki/Current-Status",
            source["_Sidebar.md"],
        )


if __name__ == "__main__":
    unittest.main()
