# loaf architecture

loaf embeds [Bun](https://github.com/oven-sh/bun) in Rust the way
[`mlua`](https://github.com/mlua-rs/mlua) embeds Lua: you add one crate, get a
runtime handle, evaluate script, move values back and forth, and expose Rust
functions to the script. The difference is what sits underneath — a full
JavaScript/TypeScript runtime (JavaScriptCore + Bun's APIs) instead of the Lua
VM.

## Why a fork of Bun is required

Since Bun's 2026 Rust rewrite, Bun is a Cargo workspace of ~105 crates. The one
that matters to us is `src/jsc` (`bun_jsc`) — Bun's Rust binding to Apple's
JavaScriptCore. It is the direct analog of the Lua C API that `mlua` wraps.

But that workspace is built to produce *one binary* (`bun`), not a library:

- Every crate is an internal detail. The workspace sets
  `unreachable_pub = "deny"`; the JSC surface we need is `pub(crate)`, not
  reachable from outside.
- It pins a **nightly** toolchain, `edition = "2024"`, `panic = "abort"`,
  a custom global allocator (`bun_alloc` / mimalloc), fat LTO, and a crash
  handler that aborts the process.
- It links a large C/C++ graph (JavaScriptCore, uWebSockets, BoringSSL, zlib,
  libarchive, c-ares, …) with cross-language LTO.

Dropping those crates straight into an arbitrary host program is not viable:
the allocator, panic strategy, nightly features, and LTO are process-wide
choices that would infect (and often break) the host's crate graph.

So loaf introduces a **seam**: a new crate inside a Bun fork, `src/embed`
(`bun_embed`), that re-exports exactly the capability we need behind a stable
**C ABI**. Everything exotic about Bun's build stays *behind* that ABI, inside a
prebuilt `libloaf`. The host links a plain C library on stable Rust.

This mirrors `mlua` precisely: `mlua-sys` owns the native Lua build and exposes
the Lua C API; `mlua` is safe Rust on stable. loaf's `libloaf` is the "Lua C
API" here, only richer.

## The three layers

```
┌─────────────────────────────────────────────────────────────┐
│ your Rust program                                             │
│   let rt = loaf::Runtime::new()?;                             │
│   let n: f64 = rt.eval("1 + 2")?;                             │
└───────────────▲─────────────────────────────────────────────┘
                │ safe, ergonomic, mlua-flavored
┌───────────────┴─────────────────────────────────────────────┐
│ crate `loaf`  (this repo, root crate — stable Rust)          │
│   Runtime, Value, Object, Array, Function, JsString          │
│   FromJs / IntoJs / FromJsMulti / IntoJsMulti, Error         │
└───────────────▲─────────────────────────────────────────────┘
                │ unsafe extern "C", 1:1 with loaf.h
┌───────────────┴─────────────────────────────────────────────┐
│ crate `loaf-sys`  (this repo, sys/ — stable Rust)            │
│   raw FFI decls + build.rs (find/link libloaf)               │
└───────────────▲─────────────────────────────────────────────┘
                │ C ABI  (loaf.h)
┌───────────────┴─────────────────────────────────────────────┐
│ `libloaf`  ==  crate `bun_embed`  (the Bun fork, src/embed)  │
│   staticlib + cdylib; nightly, panic=abort, mimalloc, LTO    │
│   wraps bun_jsc + bun_transpiler + bun_event_loop +          │
│   bun_runtime and exposes extern "C" loaf_* functions        │
└───────────────▲─────────────────────────────────────────────┘
                │ Rust path-deps (internal Bun crates)
┌───────────────┴─────────────────────────────────────────────┐
│ Bun workspace: bun_jsc, bun_transpiler, bun_event_loop, …    │
│ JavaScriptCore (C++), uWebSockets, BoringSSL, mimalloc, …    │
└─────────────────────────────────────────────────────────────┘
```

Layers 2 and 3 (`loaf`, `loaf-sys`) live in *this* repo. Layer 4 (`bun_embed`)
lives in the Bun fork (`wess/bun`, `src/embed`) because it needs path-dependency
access to Bun's internal crates.

## The C ABI (`loaf.h`) — design principles

The ABI is modeled on how `mlua` bridges to Lua, adapted to JavaScriptCore's
realities (a moving GC, NaN-boxed values, UTF-16 strings, one-VM-per-thread):

- **Opaque handles, GC-rooted.** A `LoafValueRef` is an opaque, non-null pointer
  to a value the embed layer has *protected* from the GC (JSC `Strong<>`-style
  rooting). The host owns each handle and frees it (`loaf_value_free`), exactly
  like `mlua`'s owned `Value`/`Table`/`Function` references into the Lua
  registry. Primitives can be created and read without long-lived handles.

- **Status + out-params, never unwinding across the ABI.** Every fallible call
  returns a `LoafStatus` and writes its result (or the pending JS exception)
  into out-params. Rust panics abort *inside* `libloaf` (Bun's design); they
  never cross the C boundary.

- **Host functions via trampoline.** `loaf_function_new(rt, cb, userdata,
  finalizer)` wraps a C callback
  `LoafStatus (*)(rt, userdata, this, argv, argc, out_ret)` as a JSC host
  function. The `finalizer` runs when JS GCs the function, so the safe layer can
  drop the boxed Rust closure. This is how `create_function(|ctx, args| …)`
  works.

- **TypeScript is first-class.** `loaf_eval` takes a language/kind option
  (`Js` / `Ts` / `Jsx` / `Tsx`, script vs ESM). For TS/JSX the embed layer runs
  `bun_transpiler` before handing source to JSC — so `.ts` "just works", which
  is the whole point of embedding *Bun* rather than a bare JS engine.

- **Async via the event loop.** `loaf_run_event_loop` / `loaf_tick` drain
  microtasks, timers, and I/O. `loaf_await` spins the loop until a promise
  settles and returns its value — giving safe Rust a blocking bridge, with a
  future-based `async` layer available behind a feature.

See `sys/include/loaf.h` for the concrete signatures and
`docs/abi.md` for the full function catalog.

## Threading model

A JavaScriptCore `VM` is owned by a single thread. `loaf::Runtime` is therefore
`!Send + !Sync` — like `mlua::Lua` without the `send` feature. Multiple
independent runtimes on different threads are fine; a single runtime stays on
its thread.

## What loaf is not

loaf is not a re-implementation of Bun and does not vendor a second copy of
JavaScriptCore. It is a thin, safe adapter over Bun's own runtime, surfaced
through a deliberately small, stable C ABI so the enormous machinery behind it
can evolve without breaking your Rust code.
