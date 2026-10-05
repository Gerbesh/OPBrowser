# Navigation

OPBrowser now has an engine-level navigation history core. It is intentionally
separate from the native Windows UI so the same rules can later be used by tabs,
keyboard shortcuts, address-bar navigation and scripted navigation.

## State

NavigationState stores:

- an ordered list of NavigationEntry values;
- the current history index;
- the original request string;
- the normalized loaded address;
- the loaded MIME type.

## Operations

Engine exposes:

- navigate(source): loads/renders and commits a new history entry;
- go_back(): reloads the previous historical request;
- go_forward(): reloads the next historical request;
- reload(): reloads the current request without adding a history entry.

## Invariants

History mutation is commit-after-success. A failed load does not create a broken
history entry or change the current index.

A new navigation after going Back discards the old forward branch, matching normal
browser-session behavior.

Back, forward and reload do not create duplicate entries.

## Next wiring step

The engine API exists, but the Win32 window currently receives only its initial
DisplayList. The next step is an application event path that can:

1. receive native navigation commands;
2. call Engine navigation methods;
3. replace the window DisplayList;
4. invalidate/repaint the native surface;
5. update visible address/navigation controls.
