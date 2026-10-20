# Navigation

## Page typing distinct from address-bar input (20 October 2026)

Windows native toolbar address remains a real separate EDIT child; page HWND WM_CHAR is routed only to the original browser's currently focused HTML input/textarea. Unicode characters, including surrogate-pair emoji, are sent after cancelable keydown, while controls such as Backspace/Delete/Enter work through the existing DOM keyboard path. This prevents typing into a web form from changing the browser address. `examples/forms/index.html` exercises it.

## Native keyboard events gate Tab and Enter defaults (20 October 2026)

Page keydown/keyup now passes through native Windows input -> ordered browser worker -> original cancelable JavaScript DOM KeyboardEvent path. If keydown listeners preventDefault, neither Tab moves focus nor Enter activates a link; keyboard event DOM changes remain. Without cancelation, Tab/Shift+Tab scrolls focused elements into view and Enter runs trusted click/default navigation, updating the address/history. Mouse focus changes update document.activeElement and dispatch focus events before click. Ctrl+L, F5 and entering a URL in the toolbar remain handled separately by native chrome.

## Keyboard focus reveal and browser viewport (20 October 2026)

Tab and Shift+Tab still follow native DOM/tabindex order. Their selected target now scrolls into view through original nested CSS overflow containers and then the Windows root document, without altering hit-test clipping or navigation history. The worker passes optional focus document coordinates only alongside the keyboard-focus repaint. Native Enter hyperlink default activation and the toolbar address field remain unchanged. Focus transitions invoke JS blur/focusout/focus/focusin listeners with normal capture/bubble phases; keydown/keyup support is still pending.

## Keyboard Enter on focused anchors (20 October 2026)

When the page's native Windows window has focus, Tab and Shift+Tab move across valid connected DOM focus targets, and Enter activates the current focused anchor using the original browser trusted cancelable click path. JavaScript can prevent the default hyperlink navigation; href is read after the handler runs. If navigation occurs, the worker treats the new address/history as a loaded page instead of an in-place reflow. Enter in the URL field continues to submit the typed address, not activate the page link. Clicking the viewport restores focus to the page HWND. No automatic offscreen-focus scroll or full keyboard DOM event system yet.

## Native hyperlink default action honors JavaScript cancellation (20 October 2026)

Real Win32 painted-text and linked-image activation now verifies the topmost source-DOM anchor, dispatches one trusted cancelable `click` with the original JavaScript event path, and navigates only when the event is not canceled. This lets client-side click handlers intercept anchors without unwanted full-page navigation. DOM changes from listeners are preserved on canceled clicks. Distinct native controls for Back, Forward and typed-URL navigation are unaffected. Script-initiated `Element.click()` default navigation, clicked anchor mutations, and keyboard activation remain P0 tasks.

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
- reflow(): rebuilds layout/paint from retained DOM/images without fetching or
  changing history; resize input invokes this on the same worker.

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
finishes. The status line shows Loading, Layout, Ready or the load error. Navigation is
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

PNG/JPEG/GIF/BMP images and image links now participate in the same worker/history
path; see [Image Loading](Image-Loading.md) for bounds and
[Inline Layout](Inline-Layout.md) for mixed text/image lines.
CSS and JavaScript remain future work. Fragment-only links reload without anchor
scrolling; HTML base elements and target/download behavior remain future work.
Resizing moves controls immediately and reflows retained page data after a 120 ms
pause. It preserves address edits and pixel scroll within the new bounds, rebuilds
link hit regions and changes no history. See [Page Reflow](Page-Reflow.md).

All standard named/numeric references decode in href attributes before layout,
so links such as `?a=1&amp;b=2` navigate to `?a=1&b=2`.
Back/Forward fetch the historical request again; there is no page cache yet.
