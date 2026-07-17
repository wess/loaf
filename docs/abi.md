# The libloaf C ABI

`libloaf` is the native library loaf links. Its full contract is
[`sys/include/loaf.h`](../sys/include/loaf.h); this page is the annotated
catalog. The Rust mirror is in `sys/src/lib.rs`, and the implementation lives in
the `bun_embed` crate inside the Bun fork.

## Model

- A `LoafRuntime*` owns a JavaScriptCore VM + global object + event loop. It is
  **single-threaded** — never touch one runtime from two threads.
- A `LoafValue*` is an opaque, **GC-protected** handle. The caller owns every
  handle a function returns and releases it with `loaf_value_free` (all handles
  are also released when the runtime is freed). Function *arguments* are
  **borrowed** — the callee never takes ownership.
- Fallible calls return a `LoafStatus`. On `LOAF_ERR_EXCEPTION` the thrown value
  is left **pending** on the runtime; retrieve it with
  `loaf_take_pending_exception`. Nothing unwinds across the boundary.

## Catalog

### Lifecycle
| Function | Purpose |
| --- | --- |
| `loaf_abi_version() -> u32` | ABI version (`1`); compare to `LOAF_ABI_VERSION`. |
| `loaf_runtime_new(opts) -> LoafRuntime*` | Create a runtime (null on failure). |
| `loaf_runtime_free(rt)` | Destroy a runtime and all its handles. |
| `loaf_globals(rt) -> LoafValue*` | The global object (`globalThis`). |

### Value lifetime & type
| Function | Purpose |
| --- | --- |
| `loaf_value_free(v)` | Release a handle. |
| `loaf_value_dup(rt, v) -> LoafValue*` | Clone a handle (root a second reference). |
| `loaf_value_type(rt, v) -> LoafType` | Coarse type tag. |
| `loaf_value_strict_equals(rt, a, b) -> int` | `a === b`. |

### Primitives
`loaf_undefined`, `loaf_null`, `loaf_boolean(b)`, `loaf_number(n)`,
`loaf_string(utf8, len)` construct values. `loaf_truthy(v)`,
`loaf_get_number(v, &out)`, `loaf_get_string(v, &LoafBytes)` read them.
`loaf_bytes_free` releases a string buffer.

### Objects
`loaf_object_new`, and `loaf_object_get` / `_set` / `_delete` / `_has` /
`_keys`, all keyed by a UTF-8 `(ptr, len)`.

### Arrays
`loaf_array_new`, `loaf_array_length`, `loaf_array_get` / `_set` / `_push`.

### Functions
| Function | Purpose |
| --- | --- |
| `loaf_function_new(rt, cb, userdata, fin)` | Wrap a C callback as a JS function. `fin` runs when the closure is dropped. |
| `loaf_call(rt, fn, this, argv, argc, &out)` | Call a function. |
| `loaf_construct(rt, fn, argv, argc, &out)` | `new fn(...)`. |

The callback shape:

```c
LoafStatus (*LoafHostFn)(LoafRuntime *rt, void *userdata,
                         LoafValue *this_val,
                         LoafValue *const *argv, size_t argc,
                         LoafValue **out_ret);
```

Return `LOAF_OK` with an owned `*out_ret` (or NULL for `undefined`), or
`LOAF_ERR_EXCEPTION` with the value to throw in `*out_ret`.

### Eval
`loaf_eval(rt, src, len, opts, &out)` evaluates source. `LoafEvalOptions` selects
the dialect (`Js` / `Ts` / `Jsx` / `Tsx`) and script-vs-module; TS/JSX are
transpiled by Bun before evaluation.

### Promises & event loop
| Function | Purpose |
| --- | --- |
| `loaf_run_event_loop(rt)` | Drain microtasks, timers, and I/O to idle. |
| `loaf_tick(rt) -> int` | One turn; nonzero if work remains. |
| `loaf_is_promise(rt, v) -> int` | Whether `v` is a promise. |
| `loaf_await(rt, promise, &out)` | Spin the loop until settled; rejection becomes a pending exception. |

### Exceptions
| Function | Purpose |
| --- | --- |
| `loaf_take_pending_exception(rt) -> LoafValue*` | Take the pending thrown value (null if none). |
| `loaf_exception_message(rt, err, &LoafBytes)` | Format a thrown value as a UTF-8 message. |

## Mapping to the safe API

| C ABI | `loaf` (safe) |
| --- | --- |
| `LoafRuntime*` | `Runtime` |
| `LoafValue*` | `Value` / `Object` / `Array` / `Function` / `JsString` |
| `loaf_eval` | `Runtime::eval`, `Chunk::eval` / `exec` |
| `loaf_function_new` + trampoline | `Runtime::create_function` |
| pending exception | `Error::Js { message, value }` |
| `loaf_await` | `Runtime::await_value` |
