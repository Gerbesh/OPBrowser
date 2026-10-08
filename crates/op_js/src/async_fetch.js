// First standards-shaped fetch/Response slice. Network policy lives in op_net.
function Response(body) {
    this._body = body;
    this.bodyUsed = false;
    this.ok = true;
    this.status = 200;
    this.statusText = "OK";
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
        opFetchText(url, function(text, error) {
            if (error !== null) reject(new TypeError(error));
            else resolve(new Response(text));
        });
    });
}
