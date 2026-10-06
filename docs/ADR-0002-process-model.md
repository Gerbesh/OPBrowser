# ADR-0002: Browser/renderer process model

Status: Accepted, staged implementation.

Date: 2026-10-06

## Context

OPBrowser currently runs the native window on the UI thread and a single `Engine` on a
navigation worker thread. That was a useful M1 bootstrap, but it cannot satisfy renderer
isolation, independent tab failure, reliable task-manager accounting or aggressive tab
discarding.

Retrofitting ownership boundaries after DOM scripting, workers and media arrive would be
substantially more expensive than defining them now.

## Decision

The canonical product model is browser-process owned.

The browser process owns native windows and tabs, `op_browser_core::TabManager`, session
metadata, lifecycle policy, permissions, downloads/history/settings, request-filter
configuration, renderer creation/termination, crash recovery and task-manager aggregation.

A renderer process owns one renderer group and contains `op_engine`, DOM, computed style,
layout, retained display data, `op_js`, the page event loop and page scripting.

The first implementation target is one renderer process per active tab. Site/process
sharing can be introduced later only after isolation semantics and measurements exist.

Network transport remains an owned OPBrowser subsystem. It may move into a dedicated
network service after browser/renderer IPC is stable. Renderer code must not gain
unrestricted filesystem or arbitrary process access.

## IPC contract

IPC uses explicit versioned data rather than shared engine objects or OS handles. Initial
message families are:

- browser to renderer: navigate, resize, reload, lifecycle transition, input;
- renderer to browser: navigation result, title/status, display update, resource request,
  memory counters, crash/termination state;
- browser to renderer: resource response or blocked response.

Large raster/display payloads may gain shared-memory backing later while the logical
message contract remains transport-independent.

## Lifecycle

Canonical tab states are `active`, `background`, `throttled`, `frozen`,
`discarded` and `restoring`. Automatic discard avoids protected tabs such as active
audio/capture, transfers and unsaved form state. A discarded tab retains enough browser
metadata to recreate its renderer and navigation state.

## Consequences

The existing single worker remains temporarily supported, but new product-level state must
not be buried inside it. `op_browser_core` is UI-independent so the current window can
migrate onto this model before process creation changes.

Renderer sandboxing is a separate implementation step, not implied by merely moving work
to another process.
