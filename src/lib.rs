//! # loaf
//!
//! Embed the [Bun](https://bun.com) JavaScript/TypeScript runtime in Rust, the
//! way [`mlua`](https://docs.rs/mlua) embeds Lua.
//!
//! ```ignore
//! use loaf::Runtime;
//!
//! let rt = Runtime::new()?;
//!
//! // Evaluate an expression and convert the result to a Rust type.
//! let sum: f64 = rt.eval("1 + 2")?;
//! assert_eq!(sum, 3.0);
//!
//! // Expose a Rust function to JavaScript.
//! let add = rt.create_function(|_, (a, b): (f64, f64)| Ok(a + b))?;
//! rt.globals().set("add", add)?;
//! let n: f64 = rt.eval("add(20, 22)")?;
//! assert_eq!(n, 42.0);
//!
//! // TypeScript works out of the box — that is the point of embedding *Bun*.
//! let greeting: String = rt
//!     .load("const who: string = 'world'; `hello ${who}`")
//!     .typescript()
//!     .eval()?;
//! assert_eq!(greeting, "hello world");
//! # Ok::<(), loaf::Error>(())
//! ```
//!
//! ## Architecture
//!
//! loaf is a thin, safe adapter over Bun's own runtime, reached through a
//! small, stable C ABI (`libloaf`). See `docs/architecture.md`. The C ABI lives
//! in the [`loaf_sys`] crate; the native library is built from a fork of Bun
//! that adds a `bun_embed` crate. Until that native library is linked (see
//! `docs/building.md`), this crate type-checks but cannot execute JavaScript.
//!
//! ## Threading
//!
//! A JavaScriptCore VM belongs to one thread, so [`Runtime`] is `!Send` and
//! `!Sync`. Use one runtime per thread; spin up independent runtimes on other
//! threads if you need parallelism.

#![deny(rust_2018_idioms)]
#![warn(missing_debug_implementations)]

pub(crate) use loaf_sys as sys;

mod array;
mod chunk;
mod convert;
mod error;
mod function;
mod handle;
mod object;
mod runtime;
mod string;
mod value;

pub use array::Array;
pub use chunk::{Chunk, Lang};
pub use convert::{FromJs, FromJsMulti, IntoJs, IntoJsMulti, Variadic};
pub use error::{Error, Result};
pub use function::Function;
pub use object::Object;
pub use runtime::{Runtime, RuntimeBuilder};
pub use string::JsString;
pub use value::{Value, ValueType};

/// Common imports for working with loaf. `use loaf::prelude::*;`
pub mod prelude {
    pub use crate::{
        Array, Chunk, Error, FromJs, FromJsMulti, Function, IntoJs, IntoJsMulti, JsString, Object,
        Result, Runtime, Value, ValueType, Variadic,
    };
}
