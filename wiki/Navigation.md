# Navigation

OPBrowser connects its engine-level navigation history to a native address bar,
Go/Back/Forward/Reload buttons and keyboard shortcuts. The engine remains separate
from Windows controls so the same history rules can later be used by tabs.

Start `target\release\op_browser.exe`, enter `https://example.com` and press Enter
or Go. An explicit HTTP/HTTPS URL can also be the first command-line argument.
Ctrl+L selects the address, F5 reloads, and the mouse wheel scrolls the document.
Blue, underlined page links are clickable and show a hand cursor. The start page
contains an Example Domain link; examples/navigation/index.html demonstrates local
relative navigation.

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
- follow_link(href): resolves against the last successfully loaded document address,
  then performs normal navigation;
- go_back(): reloads the previous historical request;
- go_forward(): reloads the next historical request;
- reload(): reloads the current request without adding a history entry.

## Invariants

History mutation is commit-after-success. A failed load does not create a broken
history entry or change the current index.

A new navigation after going Back discards the old forward branch, matching normal
browser-session behavior.

Back, forward and reload do not create duplicate entries.

## Native event path

The window translates Enter and button/shortcut input into NavigationEvent values.
Button commands are queued out of the window procedure to avoid callback reentry.
op_browser sends a command plus viewport dimensions to a single worker that owns
Engine. It loads/renders and sends the page and history-button flags to the UI.
Only the UI thread replaces the display list, updates controls and repaints.

A Win32 timer polls results every 30 ms while loading; it is stopped when loading
finishes. The status line shows Loading, Ready or the load error. Navigation is
serialized: buttons are disabled and further navigation shortcuts are ignored
while a request is in flight. Paint, address editing, scroll and close stay live.
A failed load keeps the previous page and history, allowing correction of the URL.

## Current limits

Pages show static HTML text with heading/paragraph defaults, including blocks inside
structural containers. Links open in the current window through the worker and
history. Inline href spans survive line wrapping; native hit regions are measured
from painted glyphs, adjusted for scroll and cleared on display replacement.
Relative network links use the final response URL, including redirects on reload;
local pages support relative filesystem links. Unsupported schemes show a load
error and preserve the previous page. They never launch another application.

CSS, images and JavaScript remain future work. Fragment-only links reload without
anchor scrolling; HTML base elements, target/download behavior and the complete
named character-reference table
remain future work. Resizing moves controls; text reflows on next navigation.

Common named/numeric references already decode in href attributes before layout,
so links such as `?a=1&amp;b=2` navigate to `?a=1&b=2`.
Back/Forward fetch the historical request again; there is no page cache yet.
