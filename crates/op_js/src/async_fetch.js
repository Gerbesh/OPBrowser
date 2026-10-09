// M4.14: bounded, same-origin Fetch/Request/Headers subset.
// Header storage, validation and transmission are native to our original VM.
function Request(input, init) {
    if (input === undefined || input === null) {
        throw new TypeError("Request requires a URL");
    }
    var url = input;
    var method = "GET";
    var headers = new Headers();
    var mode = "same-origin";
    var credentials = "omit";
    var redirect = "follow";
    var signal = null;
    if (input._opRequest === true) {
        url = input.url;
        method = input.method;
        headers = new Headers(input.headers);
        mode = input.mode;
        credentials = input.credentials;
        redirect = input.redirect;
        signal = input.signal;
    }
    if (init !== undefined && init !== null) {
        if (init.method !== undefined) method = init.method;
        if (init.headers !== undefined) headers = new Headers(init.headers);
        if (init.mode !== undefined) mode = init.mode;
        if (init.credentials !== undefined) credentials = init.credentials;
        if (init.redirect !== undefined) redirect = init.redirect;
        if (init.signal !== undefined) signal = init.signal;
        if (init.body !== undefined ||
            init.cache !== undefined || init.referrer !== undefined ||
            init.referrerPolicy !== undefined || init.integrity !== undefined ||
            init.keepalive !== undefined) {
            throw new TypeError("Unsupported RequestInit field");
        }
    }
    if (method === "get") method = "GET";
    if (method !== "GET") throw new TypeError("Only GET is supported");
    if (mode !== "same-origin") throw new TypeError("Only same-origin is supported");
    if (credentials !== "omit") throw new TypeError("Only credentials: omit is supported");
    if (redirect !== "follow" && redirect !== "error") {
        throw new TypeError("Only follow/error redirect modes are supported");
    }
    this._opRequest = true;
    this.url = url;
    this.method = method;
    this.headers = headers;
    this.mode = mode;
    this.credentials = credentials;
    this.redirect = redirect;
    this.signal = signal;
}

function Response(body, metadata) {
    this._body = body;
    this.bodyUsed = false;
    this.status = metadata.status;
    this.statusText = metadata.statusText;
    this.url = metadata.url;
    this.redirected = metadata.redirected;
    this.headers = metadata.headers;
    this.ok = this.status >= 200 && this.status <= 299;
}
Response.prototype.text = function() {
    if (this.bodyUsed) return Promise.reject(new TypeError("Body already consumed"));
    this.bodyUsed = true;
    return Promise.resolve(this._body);
};
Response.prototype.json = function() {
    if (this.bodyUsed) return Promise.reject(new TypeError("Body already consumed"));
    this.bodyUsed = true;
    return Promise.resolve(this._body).then(function(source) {
        return JSON.parse(source);
    });
};

function fetch(input, init) {
    return new Promise(function(resolve, reject) {
        var signal = null;
        var onAbort = null;
        try {
            var request = new Request(input, init);
            signal = request.signal;
            var pending = true;
            onAbort = function() {
                if (!pending) return;
                pending = false;
                reject(signal.reason);
            };
            if (signal !== null) {
                if (signal.aborted) {
                    // Native validation still runs, but it never queues a
                    // request for a pre-aborted signal.
                    opFetchText(request.url, function(){}, true,
                        request.headers, request.redirect === "error", signal);
                    reject(signal.reason);
                    return;
                }
                signal.addEventListener("abort", onAbort, {once:true});
            }
            opFetchText(request.url, function(text, error, metadata) {
                if (!pending) return;
                pending = false;
                if (signal !== null) signal.removeEventListener("abort", onAbort);
                if (error !== null) reject(new TypeError(error));
                else resolve(new Response(text, metadata));
            }, true, request.headers, request.redirect === "error", signal);
        } catch (error) {
            if (signal !== null && onAbort !== null &&
                typeof signal.removeEventListener === "function") {
                try { signal.removeEventListener("abort", onAbort); }
                catch (ignored) {}
            }
            reject(error);
        }
    });
}
