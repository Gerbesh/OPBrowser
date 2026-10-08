# Code Slicer

The project now has an executable **curated code-slice verifier**.

Run from the repository root:

```powershell
python tools/code_intelligence.py --write
python tools/code_intelligence.py --check
```

The configuration is
[`tools/code_slices.json`](https://github.com/Gerbesh/OPBrowser/blob/main/tools/code_slices.json).
The generator checks each path and symbol in the current Rust source and
writes [GENERATED_CODE_SLICES.md](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GENERATED_CODE_SLICES.md).

Current tracked slices include document → pixels, navigation/reflow,
CSS → painting, image/ICC → GDI, the standalone JS VM integration boundary,
and WPT metric reporting. Invalid paths, missing symbols and duplicates
fail the check; the same command runs in CI.

**Scope:** this is an explicit feature-flow/source-anchor verifier. It
does **not** pretend to be whole-program Rust AST dependency analysis,
an automatic call graph, a debugger, or a dynamic trace. The maintained
deep explanations remain in
[CODE_SLICES.md](https://github.com/Gerbesh/OPBrowser/blob/main/docs/CODE_SLICES.md).

Related: [Code Graph](Code-Graph.md), [Development Workflow](Development-Workflow.md).
