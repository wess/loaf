//! Fallback definitions of the `libloaf` C ABI, compiled only when the native
//! library is **not** linked (feature `link` off). They let any binary that
//! depends on loaf link and run, while making the runtime degrade gracefully:
//! creating a runtime returns null (surfaced as an error), and every other
//! entry point is an inert no-op. When the real library is linked these are
//! absent and the genuine symbols resolve instead.
//!
//! Keep in sync with the `extern "C"` block in `lib.rs`.

use core::ffi::{c_char, c_void};

use crate::{
    LoafBytes, LoafEvalOptions, LoafFinalizer, LoafHostFn, LoafRuntime, LoafRuntimeOptions,
    LoafStatus, LoafType, LoafValue, LOAF_ABI_VERSION,
};

use core::ptr;

#[no_mangle]
pub extern "C" fn loaf_abi_version() -> u32 {
    LOAF_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn loaf_runtime_new(_opts: *const LoafRuntimeOptions) -> *mut LoafRuntime {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_runtime_free(_rt: *mut LoafRuntime) {}

#[no_mangle]
pub extern "C" fn loaf_globals(_rt: *mut LoafRuntime) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_value_free(_v: *mut LoafValue) {}

#[no_mangle]
pub extern "C" fn loaf_value_dup(_rt: *mut LoafRuntime, _v: *mut LoafValue) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_value_type(_rt: *mut LoafRuntime, _v: *mut LoafValue) -> LoafType {
    LoafType::Other
}

#[no_mangle]
pub extern "C" fn loaf_value_strict_equals(
    _rt: *mut LoafRuntime,
    _a: *mut LoafValue,
    _b: *mut LoafValue,
) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn loaf_undefined(_rt: *mut LoafRuntime) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_null(_rt: *mut LoafRuntime) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_boolean(_rt: *mut LoafRuntime, _b: i32) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_number(_rt: *mut LoafRuntime, _n: f64) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_string(
    _rt: *mut LoafRuntime,
    _utf8: *const c_char,
    _len: usize,
) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_truthy(_rt: *mut LoafRuntime, _v: *mut LoafValue) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn loaf_get_number(
    _rt: *mut LoafRuntime,
    _v: *mut LoafValue,
    _out: *mut f64,
) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn loaf_get_string(
    _rt: *mut LoafRuntime,
    _v: *mut LoafValue,
    _out: *mut LoafBytes,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_bytes_free(_bytes: LoafBytes) {}

#[no_mangle]
pub extern "C" fn loaf_object_new(_rt: *mut LoafRuntime) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_object_get(
    _rt: *mut LoafRuntime,
    _obj: *mut LoafValue,
    _key: *const c_char,
    _klen: usize,
    _out: *mut *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_object_set(
    _rt: *mut LoafRuntime,
    _obj: *mut LoafValue,
    _key: *const c_char,
    _klen: usize,
    _val: *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_object_delete(
    _rt: *mut LoafRuntime,
    _obj: *mut LoafValue,
    _key: *const c_char,
    _klen: usize,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_object_has(
    _rt: *mut LoafRuntime,
    _obj: *mut LoafValue,
    _key: *const c_char,
    _klen: usize,
    _out: *mut i32,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_object_keys(
    _rt: *mut LoafRuntime,
    _obj: *mut LoafValue,
    _out_array: *mut *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_array_new(_rt: *mut LoafRuntime) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_array_length(
    _rt: *mut LoafRuntime,
    _arr: *mut LoafValue,
    _out: *mut u32,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_array_get(
    _rt: *mut LoafRuntime,
    _arr: *mut LoafValue,
    _i: u32,
    _out: *mut *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_array_set(
    _rt: *mut LoafRuntime,
    _arr: *mut LoafValue,
    _i: u32,
    _val: *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_array_push(
    _rt: *mut LoafRuntime,
    _arr: *mut LoafValue,
    _val: *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_function_new(
    _rt: *mut LoafRuntime,
    _cb: LoafHostFn,
    _userdata: *mut c_void,
    _fin: Option<LoafFinalizer>,
) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_call(
    _rt: *mut LoafRuntime,
    _f: *mut LoafValue,
    _this_val: *mut LoafValue,
    _argv: *const *mut LoafValue,
    _argc: usize,
    _out_ret: *mut *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_construct(
    _rt: *mut LoafRuntime,
    _f: *mut LoafValue,
    _argv: *const *mut LoafValue,
    _argc: usize,
    _out_ret: *mut *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_eval(
    _rt: *mut LoafRuntime,
    _src: *const c_char,
    _srclen: usize,
    _opts: *const LoafEvalOptions,
    _out_ret: *mut *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_run_event_loop(_rt: *mut LoafRuntime) {}

#[no_mangle]
pub extern "C" fn loaf_tick(_rt: *mut LoafRuntime) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn loaf_is_promise(_rt: *mut LoafRuntime, _v: *mut LoafValue) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn loaf_await(
    _rt: *mut LoafRuntime,
    _promise: *mut LoafValue,
    _out_ret: *mut *mut LoafValue,
) -> LoafStatus {
    LoafStatus::Internal
}

#[no_mangle]
pub extern "C" fn loaf_take_pending_exception(_rt: *mut LoafRuntime) -> *mut LoafValue {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn loaf_exception_message(
    _rt: *mut LoafRuntime,
    _err: *mut LoafValue,
    _out: *mut LoafBytes,
) -> LoafStatus {
    LoafStatus::Internal
}
