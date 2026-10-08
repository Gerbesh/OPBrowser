# OPBrowser Wiki

OPBrowser is an experimental Windows 11 browser built around an original web engine
and original ECMAScript engine.

## Start here

- [Current Status (verified metrics and missing features)](Current-Status.md)
- [Project Overview](Project-Overview.md)
- [Roadmap](Roadmap.md)
- [Known Test Divergences (Rec.2020)](Known-Test-Divergences.md)
- [Code Graph (crate dependencies)](Code-Graph.md)
- [Code Slicer (source-backed feature paths)](Code-Slicer.md)
- [Architecture](Architecture.md)
- [Rendering Pipeline](Rendering-Pipeline.md)
- [CSS Syntax Foundation](CSS-Syntax-Foundation.md)
- [CSS Block Box Model](CSS-Box-Model.md)
- [Stylesheet Loading](Stylesheet-Loading.md)
- [Document Source Loading](Document-Source-Loading.md)
- [HTML Text Decoding](HTML-Text-Decoding.md)
- [Image Loading](Image-Loading.md)
- [Inline Layout](Inline-Layout.md)
- [Page Reflow](Page-Reflow.md)
- [Navigation](Navigation.md)
- [JavaScript Engine](JavaScript-Engine.md)
- [Request Filtering](Request-Filtering.md)
- [Development Workflow](Development-Workflow.md)
- [Compatibility Strategy](Compatibility-Strategy.md)

The canonical implementation plan is
[docs/PROJECT_PLAN.md](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md).
This `wiki/` directory is the version-controlled source of the
[published OPBrowser GitHub Wiki](https://github.com/Gerbesh/OPBrowser/wiki).
Run `python tools/publish_wiki.py --target target/opbrowser-wiki-publish --write`
against a checkout of the separate Wiki Git repository, then commit and
push that repository. Source links are rewritten for GitHub Wiki during
publication; editing a page on GitHub directly can cause it to drift
from these tracked sources.
