//! Locate and link the native `libloaf` (the `bun_embed` staticlib/cdylib from
//! the Bun fork). See docs/building.md for the full story.
//!
//! Three outcomes:
//!   1. `LOAF_LIB_DIR` is set        -> link that prebuilt library.
//!   2. feature `build-from-source`  -> build `bun_embed` from a Bun checkout.
//!   3. neither                      -> emit nothing, so `cargo check`/rlib
//!      builds still succeed on any platform (extern symbols stay unresolved
//!      until something actually links a binary). If feature `link` is on we
//!      fail loudly instead, because the caller explicitly asked to link.

use std::env;
use std::path::PathBuf;

fn main() {
    // Register the cfg we may set so a plain `cargo build` doesn't warn.
    println!("cargo:rustc-check-cfg=cfg(loaf_linked)");

    println!("cargo:rerun-if-env-changed=LOAF_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LOAF_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=LOAF_BUN_SRC");
    println!("cargo:rerun-if-env-changed=LOAF_LINK_STATIC");

    // Re-export the header location to dependents that want to run cbindgen /
    // include the C ABI (DEP_LOAF_INCLUDE).
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:include={}", manifest.join("include").display());

    let want_link = cfg!(feature = "link");
    let from_source = cfg!(feature = "build-from-source");

    if from_source {
        build_from_source();
        return;
    }

    match env::var_os("LOAF_LIB_DIR") {
        Some(dir) => link_prebuilt(PathBuf::from(dir)),
        None if want_link => panic!(
            "loaf-sys: feature `link` is enabled but no native library was found.\n\
             Set LOAF_LIB_DIR to a directory containing libloaf.a / libloaf.so / \
             libloaf.dylib, or enable the `build-from-source` feature.\n\
             See docs/building.md."
        ),
        None => {
            // Check-only build: do not emit link directives. The safe crate
            // still type-checks; nothing that executes JS is linked.
            println!(
                "cargo:warning=loaf-sys: no LOAF_LIB_DIR set and feature `link` \
                 is off; building without the native runtime (check-only). \
                 JS execution is unavailable until libloaf is linked."
            );
        }
    }
}

fn link_prebuilt(dir: PathBuf) {
    println!("cargo:rustc-link-search=native={}", dir.display());

    // Prefer static unless told otherwise; a static libloaf pulls in the whole
    // Bun/JSC archive, which is usually what an embedder wants.
    let static_link = env::var("LOAF_LINK_STATIC")
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(true);

    if static_link {
        println!("cargo:rustc-link-lib=static=loaf");
        link_native_cxx_deps();
    } else {
        println!("cargo:rustc-link-lib=dylib=loaf");
    }

    println!("cargo:rustc-cfg=loaf_linked");
    println!("cargo:lib_dir={}", dir.display());
}

/// A static libloaf bundles JavaScriptCore (C++), so the final link needs the
/// C++ runtime and a few platform libraries. A dynamic libloaf carries these
/// itself, so this only applies to the static path.
fn link_native_cxx_deps() {
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=m");
    } else if target.contains("windows") {
        // MSVC links the C++ runtime automatically; nothing extra by default.
    }
}

/// Build `bun_embed` from a Bun fork checkout. This mirrors what Bun's own
/// build does and is intentionally strict about prerequisites, because a
/// half-configured source build fails in confusing ways deep inside cargo.
fn build_from_source() {
    let src = env::var_os("LOAF_BUN_SRC")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            panic!(
                "loaf-sys: feature `build-from-source` is enabled but LOAF_BUN_SRC \
             is not set. Point it at a checkout of the Bun fork that contains \
             the `bun_embed` crate (src/embed). See docs/building.md."
            )
        });

    let embed = src.join("src").join("embed");
    if !embed.join("Cargo.toml").exists() {
        panic!(
            "loaf-sys: LOAF_BUN_SRC={} does not contain src/embed/Cargo.toml \
             (the bun_embed crate). Is this the right Bun fork?",
            src.display()
        );
    }

    // The actual invocation is delegated to Bun's build orchestration, which
    // knows the pinned nightly toolchain, -Zbuild-std members, JSC prebuilds,
    // and fat-LTO link. We shell out to it and then link the artifact it drops
    // into `build/`. Kept minimal here on purpose; see docs/building.md.
    println!(
        "cargo:warning=loaf-sys: building bun_embed from {} — this uses Bun's \
         own toolchain and can take many minutes.",
        embed.display()
    );

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let status = std::process::Command::new("bun")
        .current_dir(&src)
        .args(["run", "build:embed", "--out"])
        .arg(&out)
        .status();

    match status {
        Ok(s) if s.success() => link_prebuilt(out),
        Ok(s) => panic!("loaf-sys: `bun run build:embed` failed ({s})."),
        Err(e) => panic!(
            "loaf-sys: could not launch Bun to build bun_embed: {e}. Install \
             Bun and ensure the fork exposes the `build:embed` script."
        ),
    }
}
