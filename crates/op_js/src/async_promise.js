// Self-hosted Promise subset, executed by OPBrowser's own JS interpreter.
// The host-owned queueMicrotask supplies FIFO jobs and bounded checkpoints.
function Promise(executor) {
    var promise = this;
    promise._promiseBrand = true;
    promise._state = 0;
    promise._value = undefined;
    promise._first = null;
    promise._last = null;
    var alreadyResolved = false;

    promise._enqueue = function(reaction) {
        queueMicrotask(function() {
            try {
                var handler = reaction.rejected;
                if (promise._state === 1) handler = reaction.fulfilled;
                if (handler === undefined || handler === null) {
                    if (promise._state === 1) reaction.resolve(promise._value);
                    else reaction.reject(promise._value);
                } else {
                    reaction.resolve(handler(promise._value));
                }
            } catch (error) {
                reaction.reject(error);
            }
        });
    };

    promise._settle = function(state, value) {
        if (promise._state !== 0) return;
        promise._state = state;
        promise._value = value;
        var current = promise._first;
        promise._first = null;
        promise._last = null;
        while (current !== null) {
            var next = current.next;
            promise._enqueue(current);
            current = next;
        }
    };

    function reject(reason) {
        promise._settle(2, reason);
    }
    function adopt(value) {
        if (promise._state !== 0) return;
        if (value === promise) {
            reject(new TypeError("Promise cannot resolve to itself"));
            return;
        }
        if (value !== null && value !== undefined) {
            try {
                var then = value.then;
                if (then !== undefined && then !== null) {
                    var called = false;
                    value.then(function(next) {
                        if (!called) {
                            called = true;
                            adopt(next);
                        }
                    }, function(reason) {
                        if (!called) {
                            called = true;
                            reject(reason);
                        }
                    });
                    return;
                }
            } catch (error) {
                reject(error);
                return;
            }
        }
        promise._settle(1, value);
    }
    function resolve(value) {
        if (alreadyResolved) return;
        alreadyResolved = true;
        adopt(value);
    }
    function rejectOnce(reason) {
        if (alreadyResolved) return;
        alreadyResolved = true;
        reject(reason);
    }
    if (executor === undefined || executor === null) {
        throw new TypeError("Promise executor must be callable");
    }
    try {
        executor(resolve, rejectOnce);
    } catch (error) {
        rejectOnce(error);
    }
}

Promise.prototype.then = function(onFulfilled, onRejected) {
    var parent = this;
    return new Promise(function(resolve, reject) {
        var reaction = {
            fulfilled: onFulfilled,
            rejected: onRejected,
            resolve: resolve,
            reject: reject,
            next: null
        };
        if (parent._state === 0) {
            if (parent._last === null) parent._first = reaction;
            else parent._last.next = reaction;
            parent._last = reaction;
        } else {
            parent._enqueue(reaction);
        }
    });
};
Promise.prototype.catch = function(onRejected) {
    return this.then(undefined, onRejected);
};
Promise.prototype.finally = function(onFinally) {
    if (onFinally === undefined || onFinally === null) return this.then();
    return this.then(
        function(value) {
            return Promise.resolve(onFinally()).then(function() { return value; });
        },
        function(reason) {
            return Promise.resolve(onFinally()).then(function() { throw reason; });
        }
    );
};
Promise.resolve = function(value) {
    if (value !== null && value !== undefined && value._promiseBrand === true) return value;
    return new Promise(function(resolve) { resolve(value); });
};
Promise.reject = function(reason) {
    return new Promise(function(resolve, reject) { reject(reason); });
};

// M4.15: combinators over bounded array/array-like input. Future iterations
// will use the ECMAScript iteration protocol, Symbol.iterator and species.
function _opPromiseLength(values) {
    if (values === null || values === undefined ||
        values.length === undefined || values.length === null) {
        throw new TypeError("Promise combinator requires an array-like value");
    }
    var count = values.length;
    if (count < 0 || count > 256 || count % 1 !== 0) {
        throw new TypeError("Promise combinator array length exceeds budget");
    }
    return count;
}
Promise.all = function(values) {
    return new Promise(function(resolve, reject) {
        try {
            var count = _opPromiseLength(values);
            var result = [];
            var remaining = count;
            if (count === 0) { resolve(result); return; }
            var i = 0;
            while (i < count) {
                (function(index) {
                    Promise.resolve(values[index]).then(
                        function(value) {
                            result[index] = value;
                            remaining = remaining - 1;
                            if (remaining === 0) resolve(result);
                        },
                        function(error) { reject(error); }
                    );
                })(i);
                i = i + 1;
            }
        } catch (error) { reject(error); }
    });
};
Promise.race = function(values) {
    return new Promise(function(resolve, reject) {
        try {
            var count = _opPromiseLength(values);
            var i = 0;
            while (i < count) {
                Promise.resolve(values[i]).then(resolve, reject);
                i = i + 1;
            }
        } catch (error) { reject(error); }
    });
};
Promise.allSettled = function(values) {
    return new Promise(function(resolve, reject) {
        try {
            var count = _opPromiseLength(values);
            var result = [];
            var remaining = count;
            if (count === 0) { resolve(result); return; }
            var i = 0;
            while (i < count) {
                (function(index) {
                    Promise.resolve(values[index]).then(
                        function(value) {
                            result[index] = {status: "fulfilled", value: value};
                            remaining = remaining - 1;
                            if (remaining === 0) resolve(result);
                        },
                        function(error) {
                            result[index] = {status: "rejected", reason: error};
                            remaining = remaining - 1;
                            if (remaining === 0) resolve(result);
                        }
                    );
                })(i);
                i = i + 1;
            }
        } catch (error) { reject(error); }
    });
};
Promise.any = function(values) {
    return new Promise(function(resolve, reject) {
        try {
            var count = _opPromiseLength(values);
            var errors = [];
            var remaining = count;
            function allRejected() {
                var error = new Error("All promises were rejected");
                error.name = "AggregateError";
                error.errors = errors;
                reject(error);
            }
            if (count === 0) { allRejected(); return; }
            var i = 0;
            while (i < count) {
                (function(index) {
                    Promise.resolve(values[index]).then(resolve, function(error) {
                        errors[index] = error;
                        remaining = remaining - 1;
                        if (remaining === 0) allRejected();
                    });
                })(i);
                i = i + 1;
            }
        } catch (error) { reject(error); }
    });
};
