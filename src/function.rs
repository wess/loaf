//! JavaScript function handle, plus the machinery that lets a Rust closure be
//! called from JavaScript.

use core::ffi::c_void;
use std::ptr;
use std::rc::{Rc, Weak};

use crate::convert::{FromJs, IntoJs, IntoJsMulti};
use crate::error::{Error, Result};
use crate::handle::{Inner, Ref};
use crate::runtime::Runtime;
use crate::sys;
use crate::value::Value;

/// An owned handle to a JavaScript function.
#[derive(Clone)]
pub struct Function(pub(crate) Ref);

impl Function {
    #[inline]
    fn rt(&self) -> Runtime {
        Runtime {
            inner: self.0.rt.clone(),
        }
    }

    /// Call the function with `this` set to `undefined`.
    pub fn call<R: FromJs>(&self, args: impl IntoJsMulti) -> Result<R> {
        self.call_with(Value::Undefined, args)
    }

    /// Call the function with an explicit `this` value.
    pub fn call_with<R: FromJs>(&self, this: impl IntoJs, args: impl IntoJsMulti) -> Result<R> {
        let rt = self.rt();
        let this_held = this.into_js(&rt)?.materialize(&rt.inner);
        let arg_refs: Vec<Ref> = args
            .into_js_multi(&rt)?
            .into_iter()
            .map(|v| v.materialize(&rt.inner))
            .collect();
        let raw_args: Vec<*mut sys::LoafValue> = arg_refs.iter().map(|r| r.raw).collect();
        let mut out = ptr::null_mut();
        let st = unsafe {
            sys::loaf_call(
                rt.inner.raw,
                self.0.raw,
                this_held.raw,
                raw_args.as_ptr(),
                raw_args.len(),
                &mut out,
            )
        };
        drop(arg_refs);
        drop(this_held);
        rt.check(st)?;
        let v = Value::from_raw(&rt.inner, out);
        R::from_js(v, &rt)
    }

    /// Invoke the function as a constructor (`new f(...)`).
    pub fn construct<R: FromJs>(&self, args: impl IntoJsMulti) -> Result<R> {
        let rt = self.rt();
        let arg_refs: Vec<Ref> = args
            .into_js_multi(&rt)?
            .into_iter()
            .map(|v| v.materialize(&rt.inner))
            .collect();
        let raw_args: Vec<*mut sys::LoafValue> = arg_refs.iter().map(|r| r.raw).collect();
        let mut out = ptr::null_mut();
        let st = unsafe {
            sys::loaf_construct(
                rt.inner.raw,
                self.0.raw,
                raw_args.as_ptr(),
                raw_args.len(),
                &mut out,
            )
        };
        drop(arg_refs);
        rt.check(st)?;
        let v = Value::from_raw(&rt.inner, out);
        R::from_js(v, &rt)
    }

    /// View this function as a generic [`Value`].
    pub fn into_value(self) -> Value {
        Value::Function(self)
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function").finish_non_exhaustive()
    }
}

/// A type-erased host closure: values in, values out.
type HostCall = Box<dyn Fn(&Runtime, Vec<Value>) -> Result<Vec<Value>>>;

/// Type-erased boxed closure plus a weak handle back to its runtime.
struct HostState {
    rt: Weak<Inner>,
    call: HostCall,
}

/// Build a JS function backed by a Rust closure. Called by
/// [`Runtime::create_function`].
pub(crate) fn create<F, A, R>(rt: &Runtime, func: F) -> Result<Function>
where
    F: Fn(&Runtime, A) -> Result<R> + 'static,
    A: crate::convert::FromJsMulti,
    R: IntoJsMulti,
{
    let call: HostCall = Box::new(move |rt, args| {
        let parsed = A::from_js_multi(args, rt)?;
        func(rt, parsed)?.into_js_multi(rt)
    });

    let state = Box::new(HostState {
        rt: Rc::downgrade(&rt.inner),
        call,
    });
    let userdata = Box::into_raw(state) as *mut c_void;

    // Safety: trampoline/finalizer have the required ABI; userdata is a valid
    // Box pointer that the finalizer reclaims exactly once.
    let raw =
        unsafe { sys::loaf_function_new(rt.inner.raw, trampoline, userdata, Some(finalizer)) };
    if raw.is_null() {
        // Creation failed before the finalizer could be registered; reclaim.
        unsafe { drop(Box::from_raw(userdata as *mut HostState)) };
        return Err(Error::Runtime("failed to create function".into()));
    }
    Ok(Function(Ref::from_owned(&rt.inner, raw)))
}

/// Entry point JS calls into. Marshals arguments, runs the closure, and writes
/// back either a return value or a value to throw.
unsafe extern "C" fn trampoline(
    _rt_raw: *mut sys::LoafRuntime,
    userdata: *mut c_void,
    _this_val: *mut sys::LoafValue,
    argv: *const *mut sys::LoafValue,
    argc: usize,
    out_ret: *mut *mut sys::LoafValue,
) -> sys::LoafStatus {
    let state = &*(userdata as *const HostState);
    let inner = match state.rt.upgrade() {
        Some(inner) => inner,
        // Runtime is gone; nothing sensible to do.
        None => return sys::LoafStatus::Internal,
    };
    let rt = Runtime { inner };

    let mut args = Vec::with_capacity(argc);
    for i in 0..argc {
        let borrowed = *argv.add(i);
        let owned = sys::loaf_value_dup(rt.inner.raw, borrowed);
        args.push(Value::from_raw(&rt.inner, owned));
    }

    match (state.call)(&rt, args) {
        Ok(values) => {
            *out_ret = collapse_returns(&rt, values);
            sys::LoafStatus::Ok
        }
        Err(err) => {
            *out_ret = error_to_value(&rt, err);
            sys::LoafStatus::Exception
        }
    }
}

/// Reclaim the boxed closure when JS garbage-collects the function.
unsafe extern "C" fn finalizer(userdata: *mut c_void) {
    drop(Box::from_raw(userdata as *mut HostState));
}

/// A JS function returns a single value: zero results become `undefined`, one
/// result passes through, several are packed into an array.
unsafe fn collapse_returns(rt: &Runtime, values: Vec<Value>) -> *mut sys::LoafValue {
    match values.len() {
        0 => sys::loaf_undefined(rt.inner.raw),
        1 => values.into_iter().next().unwrap().into_raw_owned(&rt.inner),
        _ => {
            let arr = sys::loaf_array_new(rt.inner.raw);
            for v in values {
                let held = v.materialize(&rt.inner);
                let _ = sys::loaf_array_push(rt.inner.raw, arr, held.raw);
                drop(held);
            }
            arr
        }
    }
}

/// Turn a host error into a value to throw. For now that is the error's message
/// as a JS string; the embed layer may wrap it in an `Error`.
unsafe fn error_to_value(rt: &Runtime, err: Error) -> *mut sys::LoafValue {
    let msg = err.to_string();
    sys::loaf_string(rt.inner.raw, msg.as_ptr().cast(), msg.len())
}
