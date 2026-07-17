//! Raw, unsafe FFI bindings to `libloaf` — the stable C ABI exposed by the
//! `bun_embed` crate in the Bun fork. These declarations mirror
//! `include/loaf.h` one-to-one; keep the two in lockstep. The safe wrapper
//! lives in the `loaf` crate.
//!
//! Nothing here is safe to call unless the native library is linked (see
//! `build.rs`). The signatures always compile so downstream code type-checks
//! on any platform.
#![allow(non_camel_case_types)]

// When the native library is not linked, provide inert fallback symbols so any
// binary that depends on loaf still links (and degrades gracefully at runtime).
#[cfg(not(feature = "link"))]
mod stub;

use core::ffi::{c_char, c_void};

/// Current ABI version this binding set targets. Must equal
/// [`loaf_abi_version`] at runtime.
pub const LOAF_ABI_VERSION: u32 = 1;

/// Opaque runtime: a JavaScriptCore VM + global object + event loop.
#[repr(C)]
pub struct LoafRuntime {
    _private: [u8; 0],
}

/// Opaque, GC-protected handle to a JS value.
#[repr(C)]
pub struct LoafValue {
    _private: [u8; 0],
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoafStatus {
    Ok = 0,
    Exception = 1,
    Syntax = 2,
    Type = 3,
    Utf8 = 4,
    Internal = 5,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoafType {
    Undefined = 0,
    Null = 1,
    Boolean = 2,
    Number = 3,
    String = 4,
    Symbol = 5,
    Object = 6,
    Array = 7,
    Function = 8,
    Promise = 9,
    BigInt = 10,
    Other = 11,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoafLang {
    Js = 0,
    Ts = 1,
    Jsx = 2,
    Tsx = 3,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoafModuleKind {
    Script = 0,
    Module = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LoafRuntimeOptions {
    pub install_bun_globals: i32,
    pub install_console: i32,
    pub heap_size_hint: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LoafEvalOptions {
    pub lang: LoafLang,
    pub module: LoafModuleKind,
    pub filename: *const c_char,
    pub filename_len: usize,
}

/// A UTF-8 buffer owned by libloaf; release with [`loaf_bytes_free`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LoafBytes {
    pub ptr: *const u8,
    pub len: usize,
}

/// Host callback invoked when JS calls a function made by [`loaf_function_new`].
pub type LoafHostFn = unsafe extern "C" fn(
    rt: *mut LoafRuntime,
    userdata: *mut c_void,
    this_val: *mut LoafValue,
    argv: *const *mut LoafValue,
    argc: usize,
    out_ret: *mut *mut LoafValue,
) -> LoafStatus;

/// Runs when the JS function is garbage-collected, to release `userdata`.
pub type LoafFinalizer = unsafe extern "C" fn(userdata: *mut c_void);

extern "C" {
    // lifecycle
    pub fn loaf_abi_version() -> u32;
    pub fn loaf_runtime_new(opts: *const LoafRuntimeOptions) -> *mut LoafRuntime;
    pub fn loaf_runtime_free(rt: *mut LoafRuntime);
    pub fn loaf_globals(rt: *mut LoafRuntime) -> *mut LoafValue;

    // value lifetime
    pub fn loaf_value_free(v: *mut LoafValue);
    pub fn loaf_value_dup(rt: *mut LoafRuntime, v: *mut LoafValue) -> *mut LoafValue;
    pub fn loaf_value_type(rt: *mut LoafRuntime, v: *mut LoafValue) -> LoafType;
    pub fn loaf_value_strict_equals(
        rt: *mut LoafRuntime,
        a: *mut LoafValue,
        b: *mut LoafValue,
    ) -> i32;

    // primitive constructors
    pub fn loaf_undefined(rt: *mut LoafRuntime) -> *mut LoafValue;
    pub fn loaf_null(rt: *mut LoafRuntime) -> *mut LoafValue;
    pub fn loaf_boolean(rt: *mut LoafRuntime, b: i32) -> *mut LoafValue;
    pub fn loaf_number(rt: *mut LoafRuntime, n: f64) -> *mut LoafValue;
    pub fn loaf_string(rt: *mut LoafRuntime, utf8: *const c_char, len: usize) -> *mut LoafValue;

    // primitive accessors
    pub fn loaf_truthy(rt: *mut LoafRuntime, v: *mut LoafValue) -> i32;
    pub fn loaf_get_number(rt: *mut LoafRuntime, v: *mut LoafValue, out: *mut f64) -> i32;
    pub fn loaf_get_string(
        rt: *mut LoafRuntime,
        v: *mut LoafValue,
        out: *mut LoafBytes,
    ) -> LoafStatus;
    pub fn loaf_bytes_free(bytes: LoafBytes);

    // objects
    pub fn loaf_object_new(rt: *mut LoafRuntime) -> *mut LoafValue;
    pub fn loaf_object_get(
        rt: *mut LoafRuntime,
        obj: *mut LoafValue,
        key: *const c_char,
        klen: usize,
        out: *mut *mut LoafValue,
    ) -> LoafStatus;
    pub fn loaf_object_set(
        rt: *mut LoafRuntime,
        obj: *mut LoafValue,
        key: *const c_char,
        klen: usize,
        val: *mut LoafValue,
    ) -> LoafStatus;
    pub fn loaf_object_delete(
        rt: *mut LoafRuntime,
        obj: *mut LoafValue,
        key: *const c_char,
        klen: usize,
    ) -> LoafStatus;
    pub fn loaf_object_has(
        rt: *mut LoafRuntime,
        obj: *mut LoafValue,
        key: *const c_char,
        klen: usize,
        out: *mut i32,
    ) -> LoafStatus;
    pub fn loaf_object_keys(
        rt: *mut LoafRuntime,
        obj: *mut LoafValue,
        out_array: *mut *mut LoafValue,
    ) -> LoafStatus;

    // arrays
    pub fn loaf_array_new(rt: *mut LoafRuntime) -> *mut LoafValue;
    pub fn loaf_array_length(
        rt: *mut LoafRuntime,
        arr: *mut LoafValue,
        out: *mut u32,
    ) -> LoafStatus;
    pub fn loaf_array_get(
        rt: *mut LoafRuntime,
        arr: *mut LoafValue,
        i: u32,
        out: *mut *mut LoafValue,
    ) -> LoafStatus;
    pub fn loaf_array_set(
        rt: *mut LoafRuntime,
        arr: *mut LoafValue,
        i: u32,
        val: *mut LoafValue,
    ) -> LoafStatus;
    pub fn loaf_array_push(
        rt: *mut LoafRuntime,
        arr: *mut LoafValue,
        val: *mut LoafValue,
    ) -> LoafStatus;

    // functions
    pub fn loaf_function_new(
        rt: *mut LoafRuntime,
        cb: LoafHostFn,
        userdata: *mut c_void,
        fin: Option<LoafFinalizer>,
    ) -> *mut LoafValue;
    pub fn loaf_call(
        rt: *mut LoafRuntime,
        f: *mut LoafValue,
        this_val: *mut LoafValue,
        argv: *const *mut LoafValue,
        argc: usize,
        out_ret: *mut *mut LoafValue,
    ) -> LoafStatus;
    pub fn loaf_construct(
        rt: *mut LoafRuntime,
        f: *mut LoafValue,
        argv: *const *mut LoafValue,
        argc: usize,
        out_ret: *mut *mut LoafValue,
    ) -> LoafStatus;

    // eval
    pub fn loaf_eval(
        rt: *mut LoafRuntime,
        src: *const c_char,
        srclen: usize,
        opts: *const LoafEvalOptions,
        out_ret: *mut *mut LoafValue,
    ) -> LoafStatus;

    // promises / event loop
    pub fn loaf_run_event_loop(rt: *mut LoafRuntime);
    pub fn loaf_tick(rt: *mut LoafRuntime) -> i32;
    pub fn loaf_is_promise(rt: *mut LoafRuntime, v: *mut LoafValue) -> i32;
    pub fn loaf_await(
        rt: *mut LoafRuntime,
        promise: *mut LoafValue,
        out_ret: *mut *mut LoafValue,
    ) -> LoafStatus;

    // exceptions
    pub fn loaf_take_pending_exception(rt: *mut LoafRuntime) -> *mut LoafValue;
    pub fn loaf_exception_message(
        rt: *mut LoafRuntime,
        err: *mut LoafValue,
        out: *mut LoafBytes,
    ) -> LoafStatus;
}
