// Original same-window postMessage subset. All delivery is queued through
// OPBrowser's native timer/event-task pipeline; no external windows/frames.
window.parent = window;
window.top = window;
window.onmessage = null;

window.postMessage = function(message, targetOrigin) {
    // HTML defaults the omitted targetOrigin to "/" (same-origin).
    if (targetOrigin === undefined) targetOrigin = "/";
    if (typeof targetOrigin === "object" && targetOrigin !== null) {
        var options = targetOrigin;
        targetOrigin = options.targetOrigin === undefined ? "/" : options.targetOrigin;
    }
    if (typeof targetOrigin !== "string") {
        throw new TypeError("postMessage target origin must be a string");
    }
    if (targetOrigin.length > 2048) {
        throw new TypeError("postMessage target origin too long");
    }
    var origin = location.origin;
    if (targetOrigin !== "*" && targetOrigin !== "/" && targetOrigin !== origin) {
        // Delivery to an unmatched target origin is silently discarded.
        return;
    }
    if (typeof message === "function" || typeof message === "symbol") {
        throw new TypeError("postMessage value cannot be cloned");
    }
    var cloned = message;
    if (message !== null && typeof message === "object") {
        // Bounded JSON-safe structured data subset. Unsupported non-JSON
        // objects and cyclic graphs reject instead of sharing live references.
        var encoded = JSON.stringify(message);
        if (typeof encoded !== "string" || encoded.length > 16384) {
            throw new TypeError("postMessage value exceeds clone subset");
        }
        cloned = JSON.parse(encoded);
    }
    setTimeout(function() {
        var event = new Event("message");
        event.data = cloned;
        event.origin = origin;
        event.source = window;
        window.dispatchEvent(event);
    }, 0);
};
