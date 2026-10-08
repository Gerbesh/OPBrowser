# Roadmap

The goal is a native Windows browser with an independently owned web
engine. Milestones are capability-oriented; the small WPT static sample
is not a proxy for full completion.

1. **Rendering and CSS:** improve positioning (current pinned 53/100),
   layout, painting, SVG and color-management fidelity.
2. **JavaScript engine:** expand ECMAScript grammar/runtime support and
   honest Test262 coverage. Existing parse-only metric is 523/1983.
3. **Page scripting:** M4.1 executes bounded classic inline scripts
   with `document.getElementById`/textContent DOM mutations. M4.2 adds
   filtered local/same-origin HTTP(S) `script src` loading and document
   ordering. Next: parser-blocking/defer/async timing, DOM event dispatch,
   script security policies and broader Web APIs.
4. **Browser features:** tabs, task manager, page isolation, session
   restore, background tab freezing, content blocker subscriptions/UI.
5. **Quality:** bounded memory/CPU benchmarks, security hardening,
   accessible controls, external conformance by subsystem.

For up-to-date technical worklists see
[PROJECT_PLAN.md](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md).
For current limitations see [Current Status](Current-Status.md).
