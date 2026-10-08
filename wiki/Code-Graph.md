# Code Graph

OPBrowser's crate-level Code Graph is now generated from real Cargo
dependency declarations. It currently detects **12 crates and 22 local
dependency edges** and renders them as Mermaid.

- [Generated crate graph](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GENERATED_CODE_GRAPH.md)
- [Maintained architecture/dependency explanation](https://github.com/Gerbesh/OPBrowser/blob/main/docs/CODE_GRAPH.md)
- [Code Slicer](Code-Slicer.md)

Regenerate and validate:

```powershell
python tools/code_intelligence.py --write
python tools/code_intelligence.py --check
```

The Windows CI workflow runs `--check`; if a manifest changes without
regenerating this file, CI fails. Graph arrows mean **crate dependencies**,
not an automatically inferred function-call graph. The human architectural
notes cover WinHTTP, WIC, GDI and ownership boundaries that Cargo alone
does not describe.
