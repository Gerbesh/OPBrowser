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
