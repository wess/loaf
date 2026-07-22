# loaf

Embed the [Bun](https://bun.com) JavaScript/TypeScript runtime in Rust — the
way [`mlua`](https://github.com/mlua-rs/mlua) embeds Lua.

```rust
use loaf::Runtime;

let rt = Runtime::new()?;

// Evaluate an expression and convert the result to a Rust type.
let sum: f64 = rt.eval("1 + 2")?;
assert_eq!(sum, 3.0);

// Expose a Rust function to JavaScript.
let add = rt.create_function(|_, (a, b): (f64, f64)| Ok(a + b))?;
rt.globals().set("add", add)?;
let n: f64 = rt.eval("add(20, 22)")?;
assert_eq!(n, 42.0);

// TypeScript works out of the box — that's the point of embedding *Bun*.
let greeting: String = rt
    .load("const who: string = 'world'; `hello ${who}`")
    .typescript()
    .eval()?;
assert_eq!(greeting, "hello world");
# Ok::<(), loaf::Error>(())
```

## Why

Since Bun's 2026 rewrite from Zig to Rust, its JavaScript core (`bun_jsc`, a
binding to JavaScriptCore) is Rust — but it is built to produce *one binary*,
not a library you can drop into a project. loaf makes it a library: a small,
`mlua`-shaped API on stable Rust, backed by Bun's real runtime.

You get a full JS/TS engine — modern JavaScript, TypeScript and JSX, the event
loop, promises, and (optionally) Bun's globals — instead of a bare embeddable
scripting language.

## Feature tour

| You want to… | loaf | mlua analog |
| --- | --- | --- |
| Create a runtime | `Runtime::new()` | `Lua::new()` |
| Eval an expression | `rt.eval::<T>("…")` | `lua.load("…").eval()` |
| Configure a chunk | `rt.load(src).typescript().name("f.ts").eval()` | `lua.load(src).set_name(…).eval()` |
| Read/write globals | `rt.globals().get`/`.set` | `lua.globals().get`/`.set` |
| Expose a Rust fn | `rt.create_function(\|_, args\| …)` | `lua.create_function(…)` |
| Objects & arrays | `Object`, `Array` | `Table` |
| Convert values | `FromJs` / `IntoJs` (+ `…Multi`) | `FromLua` / `IntoLua` (+ `…Multi`) |
| Variadics | `Variadic<T>` | `Variadic<T>` |
| Run async work | `rt.run_event_loop()`, `rt.await_value(p)` | `call_async`, coroutines |

Argument tuples and return tuples spread positionally, so a host function can
take several typed arguments and hand several back:

```rust
let minmax = rt.create_function(|_, xs: Vec<f64>| {
    let lo = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Ok((lo, hi)) // -> a JS array [lo, hi]
})?;
```

See [`examples/`](examples): `eval`, `registerfn`, `typescript`, `hostobject`.

## Architecture

loaf is layered so that everything exotic about Bun's build stays behind a
stable C ABI:

```
your Rust  ──►  loaf (safe API)  ──►  loaf-sys (raw FFI)  ──►  libloaf (C ABI)  ──►  bun_jsc + bun_runtime + JSC
                  this repo               sys/                   the Bun fork
```

- **`loaf`** (this crate) — safe, ergonomic, `mlua`-shaped API.
- **`loaf-sys`** (`sys/`) — raw FFI + the build script that finds/links the
  native library.
- **`libloaf`** — the `bun_embed` crate added to a
  [fork of Bun](https://github.com/wess/bun/pull/1) (`src/embed`). It exposes
  the C ABI in [`sys/include/loaf.h`](sys/include/loaf.h) over Bun's internals.

Full details in [`docs/architecture.md`](docs/architecture.md); the ABI catalog
is in [`docs/abi.md`](docs/abi.md).

## Status & platforms

Bun's Rust rewrite currently targets **Linux x64 glibc**; macOS, Windows, and
ARM are on its roadmap. Accordingly:

- The `loaf` safe API **compiles on every platform** (it links `loaf-sys`'s
  fallback symbols when the native library is absent), so your code and docs
  build in CI anywhere. Without the native library, `Runtime::new()` returns an
  error rather than failing to build.
- **Executing JavaScript** needs the native `libloaf`, which builds where Bun's
  native build works (Linux x64 today).

Build modes and the platform matrix are documented in
[`docs/building.md`](docs/building.md).

## Installing

```toml
[dependencies]
loaf = { git = "https://github.com/wess/loaf" }
```

To actually run JavaScript, build with the `link` feature and point loaf at a
prebuilt `libloaf` (see [building](docs/building.md)):

```sh
export LOAF_LIB_DIR=/path/to/libloaf
cargo build --features link
```

Cargo features:

- `link` — require and link the native `libloaf` (needed to execute JS).
- `build-from-source` — build `libloaf` from a Bun fork checkout instead of a
  prebuilt library.
- `serde` — serde-based conversion between JS values and Rust types.

## Threading

A JavaScriptCore VM belongs to one thread, so `Runtime` is `!Send + !Sync`, like
`mlua::Lua` without the `send` feature. Use one runtime per thread; run
independent runtimes on other threads for parallelism.

## License

MIT © Wess Cope

♥ [Sponsor this project](https://github.com/sponsors/wess)
