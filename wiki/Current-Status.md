# Current Project Status

Last reviewed: **8 October 2026**. The [published GitHub Wiki](https://github.com/Gerbesh/OPBrowser/wiki) tracks the main repository's `wiki/` sources. This is a development snapshot, not a
claim that OPBrowser is ready for everyday browsing.

## Working today

- Native Windows 11 browser window with address field, navigation, back/forward,
  reload, scrolling and retained page reflow.
- Independent HTML tokenizer/tree builder, DOM, CSS tokenizer/cascade, text
  layout, display-list painter and native GDI presentation.
- Local/file/data and bounded HTTP(S) document loading; author stylesheets,
  images, basic SVG text, CSS backgrounds, relative/absolute/fixed layout,
  floats, basic flex/table formatting, selected compositing/filter effects.
- Windows WIC-backed raster decoding and ICC color conversion.
- Independent JavaScript lexer/parser/bytecode VM now executes bounded
  classic inline **and same-origin external** scripts in document order,
  supporting `document.getElementById` and textContent updates before
  CSS/layout. Events, async/defer scheduling and broad Web APIs remain absent.
- Network request filtering foundation; full adblock UI/subscription pipeline
  is **not** implemented.

## Verified compatibility snapshot

| Metric | Result | Meaning |
| --- | --- | --- |
| WPT Static v1 strict | **197/200 (98.5%)** | Fixed subset, exact RGB comparison |
| WPT Static v1 WPT-authored fuzzy | **198/200 (99%)** | Separate opt-in report |
| WPT Positioning v1 strict | **53/100 (53%)** | Fixed 100 reftests |
| Test262 Parser v1 | **523/1983 (26.37%)** | Parse-only sample, not JS runtime coverage |

The two outstanding Rec.2020 cases have documented **known
legacy-reference divergence** status and remain counted as failures.
See [Known Test Divergences](Known-Test-Divergences.md).

## Biggest gaps

Complete page-script lifecycle, broad DOM/Web API bindings, events, modern JavaScript runtime
coverage, modern layout modes, complete CSS painting/stacking, browser tabs,
tab freezing, browser task manager, settings and full content blocker remain
major workstreams. The 98.5% metric measures only **200 selected static tests**.

See [Project Overview](Project-Overview.md), [Architecture](Architecture.md),
[Code Graph](Code-Graph.md), [Code Slicer](Code-Slicer.md), and
[Development Workflow](Development-Workflow.md).
