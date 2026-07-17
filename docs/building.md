# Building loaf

loaf embeds the [Bun](https://github.com/oven-sh/bun) runtime. Since Bun's
2026 Rust rewrite, Bun is a Cargo workspace of ~105 crates that links a large
amount of C/C++ (JavaScriptCore, uWebSockets, BoringSSL, mimalloc, zlib,
libarchive, c-ares, …). loaf does **not** pull that graph into your crate.
Instead it links a single prebuilt native library, `libloaf`, that exposes a
stable C ABI. This is the same shape mlua uses: a `-sys` crate owns the native
build, the top crate is safe Rust on stable.

## The two build modes of `loaf-sys`

`loaf-sys/build.rs` decides how to obtain `libloaf`:

1. **System / prebuilt (default, recommended).** Point loaf at an already-built
   `libloaf` and its headers:

   ```sh
   export LOAF_LIB_DIR=/path/to/libloaf/lib      # contains libloaf.a / .so / .dylib
   export LOAF_INCLUDE_DIR=/path/to/libloaf/include
   cargo build
   ```

   `build.rs` emits `cargo:rustc-link-search` / `cargo:rustc-link-lib` and stops.
   Nothing exotic is required of your toolchain — stable Rust is fine.

2. **Build from source (`--features build-from-source`).** `build.rs` clones the
   Bun fork that carries the `bun_embed` crate (or uses `LOAF_BUN_SRC` if set),
   builds the `bun_embed` staticlib with Bun's own toolchain, and links it. This
   requires everything Bun's build requires (see below) and is slow.

If neither a prebuilt lib nor the source feature is available, `build.rs` still
lets `cargo check` succeed (it only skips emitting link directives), so the
safe API type-checks anywhere. Anything that actually *runs* JavaScript needs
the native lib and is gated behind `cfg(loaf_linked)`.

## Platform support (as of the Rust rewrite)

Bun's Rust rewrite currently targets **Linux x64 glibc**. macOS, Windows, and
ARM are on the roadmap but not yet at parity. Practically:

| Target                | `loaf` safe API compiles | native `libloaf` builds/links |
| --------------------- | :----------------------: | :---------------------------: |
| linux x86_64 glibc    | yes                      | yes                           |
| macOS arm64 / x86_64  | yes                      | not yet (tracks Bun)          |
| windows x86_64        | yes                      | not yet (tracks Bun)          |
| linux aarch64         | yes                      | not yet (tracks Bun)          |

The safe crate is written to compile on every platform so downstream code and
docs build in CI; only the linked, JS-executing paths depend on the platform
where Bun's native build works.

## Building the native lib from the fork

The `bun_embed` crate lives inside the Bun fork under `src/embed`. Its
`Cargo.toml` sets `crate-type = ["staticlib", "cdylib"]`. Build it with Bun's
build tooling (pinned nightly toolchain from `rust-toolchain.toml`, edition
2024, `-Zbuild-std` for some members, fat LTO). The produced `libloaf.a` /
`libbun_embed.a` plus the generated `loaf.h` are what mode (1) above consumes.

See `docs/architecture.md` for how the layers fit together.
