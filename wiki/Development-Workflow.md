# Development Workflow

Every coherent update follows the same loop:

1. implement one bounded change;
2. update the project plan/status if needed;
3. append the change to the development log;
4. update code graph/code slices when architecture changes;
5. update relevant wiki pages;
6. run rustfmt, Clippy, tests, and startup smoke tests;
7. create one descriptive Git commit;
8. push it to the public repository.

Red builds are not considered completed updates.

See AGENTS.md for the exact repository rules.

The test suite includes bounded loopback HTTP fixtures and native control dispatch /
repaint coverage. `cargo run -p op_browser -- --navigation-smoke-test` additionally
checks Enter input -> command channel -> navigation worker -> result channel ->
native repaint using an offline data URL; CI runs it after the startup smoke.
`cargo run -p op_browser -- --link-smoke-test` additionally verifies a native link
click, relative-file loading, history and destination repaint using local fixtures.
External network verification is separate:
`cargo run -p op_browser -- --navigation-smoke-test https://example.com`.

The complete named-reference source is pinned in crates/op_html/data/entities.tsv.
`python tools/generate_html_entities.py --check` verifies the committed compact Rust
table in CI; normal builds do not run a generator or access the network. Exhaustive
Cargo tests cover every source spelling in text/attributes/RCDATA and prefix recovery.
`cargo run -p op_browser -- --link-smoke-test examples/encoding/named-references.html`
checks Unicode text and decoded links through native painting and click dispatch.
See [HTML Text Decoding](HTML-Text-Decoding.md) for provenance and supported rules.

## Current GitHub Actions status

The Windows CI workflow is committed and GitHub discovers it correctly, but remote
jobs are currently blocked before runner startup. GitHub's check annotation states
that the account is locked due to a billing issue.

Until that external account state is cleared, local completion still requires the
full format, Clippy, test, paint-smoke, and release-build checks. A red GitHub Actions
run with zero executed steps must not be interpreted as an OPBrowser code failure.
