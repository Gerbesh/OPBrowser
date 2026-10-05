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

## Current GitHub Actions status

The Windows CI workflow is committed and GitHub discovers it correctly, but remote
jobs are currently blocked before runner startup. GitHub's check annotation states
that the account is locked due to a billing issue.

Until that external account state is cleared, local completion still requires the
full format, Clippy, test, paint-smoke, and release-build checks. A red GitHub Actions
run with zero executed steps must not be interpreted as an OPBrowser code failure.
