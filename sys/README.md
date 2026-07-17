# loaf-sys

Raw FFI bindings and native-library linkage for [`loaf`](https://github.com/wess/loaf),
which embeds the Bun runtime in Rust.

This crate declares the `libloaf` C ABI (mirroring [`include/loaf.h`](include/loaf.h))
and owns the build script that locates and links the native library. You almost
certainly want the safe [`loaf`](https://crates.io/crates/loaf) crate instead.

When the native library is not linked, `loaf-sys` provides inert fallback
symbols so any binary still links; see [`docs/building.md`](../docs/building.md).
