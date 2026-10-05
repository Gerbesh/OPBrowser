# OPBrowser project rules

These rules are mandatory for every engineering update.

## Product invariants

- OPBrowser is an original browser and web engine.
- Do not embed, fork, wrap, or silently depend on Chromium/Blink, WebKit, Gecko,
  Servo as a ready-made engine, WebView2, CEF, V8, SpiderMonkey,
  JavaScriptCore, QuickJS, or another ready-made browser/JavaScript engine.
- Windows APIs and focused infrastructure libraries are allowed when they do not
  replace OPBrowser-owned HTML, DOM, CSS, layout, painting, JavaScript, or Web APIs.
- Prefer lightweight dependencies and measurable resource costs.
- Telemetry is off/absent by default.

## Definition of Done for every coherent update

1. Implement the code.
2. Update docs/PROJECT_PLAN.md when scope/status changes.
3. Append what changed and why to docs/DEV_LOG.md.
4. Update docs/CODE_GRAPH.md when modules, dependencies, or key types change.
5. Update docs/CODE_SLICES.md when an end-to-end feature slice changes.
6. Update wiki/ pages when user/developer-facing architecture or workflow changes.
7. Run:
   - cargo fmt --all -- --check
   - cargo clippy --workspace --all-targets -- -D warnings
   - cargo test --workspace
   - cargo run -p op_browser -- --smoke-test when browser startup is affected
8. Do not leave the repository in a known-red state.
9. Commit the coherent update with a descriptive Git commit message.
10. Push the commit to the public GitHub repository when a remote is available.

## Commit policy

- One coherent user-visible/project-state update per commit.
- Avoid giant "misc changes" commits.
- Documentation that describes a code change belongs in the same commit.
- Never rewrite public history without an explicit reason.

## Architecture documentation

- docs/CODE_GRAPH.md is the human-readable dependency and key-type graph.
- docs/CODE_SLICES.md tracks vertical end-to-end slices from input to pixels/behavior.
- wiki/ is the repository-local source for project wiki content.
- docs/DEV_LOG.md is append-only project history, except typo corrections.
