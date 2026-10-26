// First-party XHR GET subset. The only network access is OPBrowser's
// bounded, same-origin native opFetchText request/completion queue.
// Unsupported HTTP methods, synchronous mode and request bodies fail explicitly.
function XMLHttpRequest() {
    if (!(this instanceof XMLHttpRequest)) {
        throw new TypeError("XMLHttpRequest requires new");
    }
    this.readyState = 0;
    this.responseType = "";
    this.response = null;
    this.responseText = "";
    this.responseURL = "";
    this.status = 0;
    this.statusText = "";
    this.timeout = 0;
    this.withCredentials = false;
    this.onreadystatechange = null;
    this.onload = null;
    this.onerror = null;
    this.onabort = null;
    this.onloadend = null;
    this.onloadstart = null;
    this._method = "";
    this._url = "";
    this._headers = null;
    this._requestHeaders = new Headers();
    this._listeners = [];
    this._epoch = 0;
    this._sending = false;
}
XMLHttpRequest.UNSENT = 0;
XMLHttpRequest.OPENED = 1;
XMLHttpRequest.HEADERS_RECEIVED = 2;
XMLHttpRequest.LOADING = 3;
XMLHttpRequest.DONE = 4;
XMLHttpRequest.prototype.UNSENT = 0;
XMLHttpRequest.prototype.OPENED = 1;
XMLHttpRequest.prototype.HEADERS_RECEIVED = 2;
XMLHttpRequest.prototype.LOADING = 3;
XMLHttpRequest.prototype.DONE = 4;

XMLHttpRequest.prototype._emit = function(type) {
    var evt = {type:type, target:this, currentTarget:this, isTrusted:true};
    var handler = this["on" + type];
    if (typeof handler === "function") handler.call(this, evt);
    var callbacks = this._listeners;
    var size = callbacks.length;
    for (var i=0;i<size;i++) {
        var entry = callbacks[i];
        if (entry.type === type) {
            if (typeof entry.callback === "function") entry.callback.call(this, evt);
            else if (entry.callback !== null &&
                     typeof entry.callback.handleEvent === "function") {
                entry.callback.handleEvent(evt);
            }
        }
    }
};
XMLHttpRequest.prototype.addEventListener = function(type, callback) {
    if (typeof callback !== "function" &&
        (callback === null || typeof callback !== "object")) return;
    if (typeof type !== "string" || type.length > 64 || this._listeners.length >= 64) {
        throw new TypeError("invalid XHR event listener");
    }
    for (var i=0;i<this._listeners.length;i++) {
        var entry = this._listeners[i];
        if (entry.type === type && entry.callback === callback) return;
    }
    this._listeners.push({type:type,callback:callback});
};
XMLHttpRequest.prototype.removeEventListener = function(type, callback) {
    var remaining = [];
    for (var i=0;i<this._listeners.length;i++) {
        var entry = this._listeners[i];
        if (entry.type !== type || entry.callback !== callback) remaining.push(entry);
    }
    this._listeners = remaining;
};
XMLHttpRequest.prototype.open = function(method, url, async) {
    if (method !== "GET" && method !== "get") {
        throw new TypeError("XHR supports GET only");
    }
    if (arguments.length < 2 || url === null || url === undefined) {
        throw new TypeError("XHR.open requires URL");
    }
    if (async === false) {
        throw new TypeError("synchronous XMLHttpRequest is not supported");
    }
    this._epoch++;
    this._sending = false;
    this._method = "GET";
    this._url = String(url);
    this._headers = null;
    this._requestHeaders = new Headers();
    this.status = 0;
    this.statusText = "";
    this.responseURL = "";
    this.response = null;
    this.responseText = "";
    this.readyState = 1;
    this._emit("readystatechange");
};
XMLHttpRequest.prototype.setRequestHeader = function(name, value) {
    if (this.readyState !== 1 || this._sending) {
        throw new TypeError("XHR headers require opened and unsent request");
    }
    this._requestHeaders.append(name, value);
};
XMLHttpRequest.prototype.getResponseHeader = function(name) {
    if (this.readyState < 2 || this._headers === null) return null;
    return this._headers.get(name);
};
XMLHttpRequest.prototype.getAllResponseHeaders = function() {
    // The backing Headers currently exposes get/has, not enumeration.
    // Do not incorrectly pretend a nonempty response has no headers.
    if (this.readyState < 2) return "";
    throw new TypeError("XHR full response header enumeration unsupported");
};
XMLHttpRequest.prototype.overrideMimeType = function() {
    throw new TypeError("XHR overrideMimeType unsupported");
};
XMLHttpRequest.prototype.abort = function() {
    var active = this._sending;
    this._epoch++;
    this._sending = false;
    this._headers = null;
    this.status = 0;
    this.statusText = "";
    this.responseURL = "";
    this.response = null;
    this.responseText = "";
    if (active) {
        this.readyState = 4;
        this._emit("readystatechange");
        this._emit("abort");
        this._emit("loadend");
    }
    this.readyState = 0;
};
XMLHttpRequest.prototype.send = function(body) {
    if (this.readyState !== 1 || this._sending) {
        throw new TypeError("XHR.send requires an open, unsent request");
    }
    if (body !== undefined && body !== null) {
        throw new TypeError("XHR GET request bodies are not supported");
    }
    if (this.timeout !== 0 || this.withCredentials !== false) {
        throw new TypeError("XHR timeout and credentialed requests unsupported");
    }
    if (this.responseType !== "" && this.responseType !== "text" &&
        this.responseType !== "json") {
        throw new TypeError("XHR responseType unsupported");
    }
    var self = this;
    var epoch = ++this._epoch;
    this._sending = true;
    this._emit("loadstart");
    try {
        opFetchText(this._url, function(text, error, metadata) {
            if (self._epoch !== epoch) return;
            self._sending = false;
            if (error !== null) {
                self.status = 0;
                self.statusText = "";
                self.response = null;
                self.responseText = "";
                self.readyState = 4;
                self._emit("readystatechange");
                self._emit("error");
                self._emit("loadend");
                return;
            }
            self.status = metadata.status;
            self.statusText = metadata.statusText;
            self.responseURL = metadata.url;
            self._headers = metadata.headers;
            self.readyState = 2;
            self._emit("readystatechange");
            self.readyState = 3;
            self._emit("readystatechange");
            self.responseText = text;
            if (self.responseType === "json") {
                try { self.response = JSON.parse(text); }
                catch (parseError) { self.response = null; }
            } else {
                self.response = text;
            }
            self.readyState = 4;
            self._emit("readystatechange");
            self._emit("load");
            self._emit("loadend");
        }, true, this._requestHeaders);
    } catch (error) {
        this._sending = false;
        throw error;
    }
};
