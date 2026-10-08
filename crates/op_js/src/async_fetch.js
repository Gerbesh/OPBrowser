// First standards-shaped fetch/Response slice. Network policy lives in op_net.
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

function fetch(url, init) {
    return new Promise(function(resolve, reject) {
        if (init !== undefined && init !== null) {
            if (init.method !== undefined && init.method !== "GET") {
                reject(new TypeError("Only GET is supported"));
                return;
            }
            if (init.body !== undefined || init.headers !== undefined || init.credentials !== undefined) {
                reject(new TypeError("Request init options are not supported yet"));
                return;
            }
        }
        try {
            opFetchText(url, function(text, error, metadata) {
                if (error !== null) reject(new TypeError(error));
                else resolve(new Response(text, metadata));
            }, true);
        } catch (error) {
            reject(error);
        }
    });
}
