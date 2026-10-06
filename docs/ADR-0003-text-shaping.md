# ADR-0003: Windows text shaping service

Status: Accepted for the Windows backend.

Date: 2026-10-06

## Context

The current renderer measures and paints text with GDI. Correct modern web text requires
Unicode shaping, bidi handling, script-specific glyph selection, fallback fonts and
high-quality metrics. Reimplementing an entire shaping stack is not a useful expression of
the project's original-browser-engine goal.

The project already permits focused operating-system infrastructure such as WinHTTP for
transport/TLS and WIC for raster decoding while retaining OPBrowser ownership of web
semantics and policy.

## Decision

The Windows backend may use DirectWrite for font discovery, shaping, fallback, glyph
metrics and rasterization. OPBrowser continues to own DOM/CSS interpretation,
font-selection policy, line breaking, inline formatting, layout, decoration geometry,
paint ordering, resource policy and web-facing APIs.

No browser engine or JavaScript engine may be introduced through this allowance.

The shaping boundary stays behind a platform-neutral text interface so another platform
can provide an equivalent native shaper without changing `op_layout`.

## Consequences

GDI remains a temporary backend while DirectWrite integration is developed. Complex
scripts and bidi are not considered supported merely because this ADR permits DirectWrite;
support requires integration tests and WPT coverage.
