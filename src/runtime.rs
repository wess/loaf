//! The [`Runtime`] handle: a single JavaScriptCore VM with Bun's globals.

use std::ptr;
use std::rc::Rc;

use crate::array::Array;
use crate::chunk::Chunk;
use crate::convert::{FromJs, IntoJsMulti};
use crate::error::{Error, Result};
use crate::function::{self, Function};
use crate::handle::{Inner, Ref};
use crate::object::Object;
use crate::string::JsString;
use crate::sys;
use crate::value::Value;

/// A running Bun runtime.
///
/// Created with [`Runtime::new`] or [`Runtime::builder`]. Cheap to clone (the
/// underlying VM is reference-counted); every handle you obtain keeps the VM
/// alive. A VM belongs to one thread, so this type is `!Send` and `!Sync`.
#[derive(Clone)]
pub struct Runtime {
    pub(crate) inner: Rc<Inner>,
}

impl Runtime {
    /// Create a runtime with Bun's default globals (`Bun`, `fetch`, `console`,
    /// timers, …) installed.
    pub fn new() -> Result<Runtime> {
        Runtime::builder().build()
    }

    /// Start configuring a runtime.
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::default()
    }

    pub(crate) fn from_options(opts: sys::LoafRuntimeOptions) -> Result<Runtime> {
        // Safety: opts is a valid, fully-initialized options struct.
        let raw = unsafe { sys::loaf_runtime_new(&opts) };
        if raw.is_null() {
            return Err(Error::Runtime("failed to create Bun runtime".into()));
        }
        Ok(Runtime {
            inner: Rc::new(Inner { raw }),
        })
    }

    /// The global object (`globalThis`).
    pub fn globals(&self) -> Object {
        let raw = unsafe { sys::loaf_globals(self.inner.raw) };
        Object(Ref::from_owned(&self.inner, raw))
    }

    /// Evaluate a JavaScript expression and convert the result to `T`.
    ///
    /// A convenience for `self.load(src).eval()`.
    pub fn eval<T: FromJs>(&self, src: &str) -> Result<T> {
        self.load(src).eval()
    }

    /// Begin loading a chunk of source. Configure it (language, name, module
    /// mode) then finish with [`Chunk::eval`] or [`Chunk::exec`].
    pub fn load(&self, src: impl Into<String>) -> Chunk<'_> {
        Chunk::new(self, src.into())
    }

    /// Create a JS string.
    pub fn create_string(&self, s: &str) -> JsString {
        let raw = unsafe { sys::loaf_string(self.inner.raw, s.as_ptr().cast(), s.len()) };
        JsString(Ref::from_owned(&self.inner, raw))
    }

    /// Create an empty JS object.
    pub fn create_object(&self) -> Object {
        let raw = unsafe { sys::loaf_object_new(self.inner.raw) };
        Object(Ref::from_owned(&self.inner, raw))
    }

    /// Create an empty JS array.
    pub fn create_array(&self) -> Array {
        let raw = unsafe { sys::loaf_array_new(self.inner.raw) };
        Array(Ref::from_owned(&self.inner, raw))
    }

    /// Expose a Rust closure to JavaScript as a function.
    ///
    /// The argument tuple type drives argument parsing ([`crate::FromJsMulti`]);
    /// the `Ok` type drives the return value ([`IntoJsMulti`]).
    pub fn create_function<F, A, R>(&self, func: F) -> Result<Function>
    where
        F: Fn(&Runtime, A) -> Result<R> + 'static,
        A: crate::convert::FromJsMulti,
        R: IntoJsMulti,
    {
        function::create(self, func)
    }

    /// Run the event loop until it is idle, draining microtasks, timers, and
    /// pending I/O (resolves promises, runs `setTimeout`, etc.).
    pub fn run_event_loop(&self) {
        unsafe { sys::loaf_run_event_loop(self.inner.raw) }
    }

    /// Run a single turn of the event loop. Returns `true` if work remains.
    pub fn tick(&self) -> bool {
        unsafe { sys::loaf_tick(self.inner.raw) != 0 }
    }

    /// Drive the event loop until `value` (a promise) settles, returning its
    /// fulfilled value converted to `T`, or the rejection as an error.
    ///
    /// Non-promise values are returned as-is.
    pub fn await_value<T: FromJs>(&self, value: Value) -> Result<T> {
        let held = value.materialize(&self.inner);
        let mut out = ptr::null_mut();
        let st = unsafe { sys::loaf_await(self.inner.raw, held.raw, &mut out) };
        drop(held);
        self.check(st)?;
        let v = Value::from_raw(&self.inner, out);
        T::from_js(v, self)
    }

    /// Call a global function by name (`globalThis[name](...args)`).
    pub fn call<R: FromJs>(&self, name: &str, args: impl IntoJsMulti) -> Result<R> {
        let f: Function = self.globals().get(name)?;
        f.call(args)
    }

    // ---- internal error plumbing ----------------------------------------

    /// Map an FFI status into a `Result`, pulling the pending exception when the
    /// call threw.
    pub(crate) fn check(&self, status: sys::LoafStatus) -> Result<()> {
        match status {
            sys::LoafStatus::Ok => Ok(()),
            sys::LoafStatus::Exception => Err(self.take_exception()),
            sys::LoafStatus::Syntax => Err(Error::Syntax(self.pending_message("syntax error"))),
            sys::LoafStatus::Type => Err(Error::Runtime(self.pending_message("type error"))),
            sys::LoafStatus::Utf8 => Err(Error::Utf8),
            sys::LoafStatus::Internal => Err(Error::Runtime("internal runtime error".into())),
        }
    }

    /// Take the pending JS exception and wrap it as an [`Error::Js`], keeping
    /// the thrown value.
    pub(crate) fn take_exception(&self) -> Error {
        let raw = unsafe { sys::loaf_take_pending_exception(self.inner.raw) };
        if raw.is_null() {
            return Error::Js {
                message: "uncaught exception".into(),
                value: None,
            };
        }
        let message = self
            .format_exception(raw)
            .unwrap_or_else(|| "uncaught exception".into());
        let value = Some(Value::from_raw(&self.inner, raw));
        Error::Js { message, value }
    }

    /// Take the pending exception (if any), format it, and discard the value.
    fn pending_message(&self, fallback: &str) -> String {
        let raw = unsafe { sys::loaf_take_pending_exception(self.inner.raw) };
        if raw.is_null() {
            return fallback.to_string();
        }
        let msg = self
            .format_exception(raw)
            .unwrap_or_else(|| fallback.to_string());
        unsafe { sys::loaf_value_free(raw) };
        msg
    }

    /// Format a thrown value as a message string via the runtime.
    fn format_exception(&self, raw: *mut sys::LoafValue) -> Option<String> {
        let mut out = sys::LoafBytes {
            ptr: ptr::null(),
            len: 0,
        };
        let st = unsafe { sys::loaf_exception_message(self.inner.raw, raw, &mut out) };
        if st != sys::LoafStatus::Ok {
            return None;
        }
        let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) };
        let msg = String::from_utf8_lossy(bytes).into_owned();
        unsafe { sys::loaf_bytes_free(out) };
        Some(msg)
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

/// Builder for a [`Runtime`], mirroring `LoafRuntimeOptions`.
#[derive(Debug, Clone)]
pub struct RuntimeBuilder {
    opts: sys::LoafRuntimeOptions,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        RuntimeBuilder {
            opts: sys::LoafRuntimeOptions {
                install_bun_globals: 1,
                install_console: 1,
                heap_size_hint: 0,
            },
        }
    }
}

impl RuntimeBuilder {
    /// Install Bun's global APIs (`Bun`, `fetch`, timers, …). Default `true`.
    pub fn bun_globals(mut self, enabled: bool) -> Self {
        self.opts.install_bun_globals = enabled as i32;
        self
    }

    /// Install `console`. Default `true`.
    pub fn console(mut self, enabled: bool) -> Self {
        self.opts.install_console = enabled as i32;
        self
    }

    /// Hint the initial heap size, in bytes. `0` uses the engine default.
    pub fn heap_size_hint(mut self, bytes: usize) -> Self {
        self.opts.heap_size_hint = bytes;
        self
    }

    /// Build the runtime.
    pub fn build(self) -> Result<Runtime> {
        Runtime::from_options(self.opts)
    }
}
