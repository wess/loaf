//! Internal reference-counted handles shared by every public wrapper.
//!
//! [`Inner`] owns the native runtime. [`Ref`] owns a single GC-protected value
//! handle and keeps its runtime alive. Both free their native resource on drop,
//! so the public types are just thin wrappers with no manual cleanup.

use std::rc::Rc;

use crate::sys;

/// Owns the native runtime pointer. Freed when the last handle drops.
pub(crate) struct Inner {
    pub(crate) raw: *mut sys::LoafRuntime,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Safety: `raw` came from loaf_runtime_new and is freed exactly once,
        // when the final Rc<Inner> drops.
        unsafe { sys::loaf_runtime_free(self.raw) }
    }
}

/// An owned, GC-protected handle to a JS value plus a strong ref to its runtime.
pub(crate) struct Ref {
    pub(crate) rt: Rc<Inner>,
    pub(crate) raw: *mut sys::LoafValue,
}

impl Ref {
    /// Take ownership of a handle returned by an FFI call.
    pub(crate) fn from_owned(rt: &Rc<Inner>, raw: *mut sys::LoafValue) -> Ref {
        Ref {
            rt: Rc::clone(rt),
            raw,
        }
    }

    #[inline]
    pub(crate) fn rt_raw(&self) -> *mut sys::LoafRuntime {
        self.rt.raw
    }
}

impl Clone for Ref {
    fn clone(&self) -> Ref {
        // Duplicating a handle roots a second reference to the same JS value.
        // Safety: both pointers are live for the duration of this call.
        let raw = unsafe { sys::loaf_value_dup(self.rt.raw, self.raw) };
        Ref {
            rt: Rc::clone(&self.rt),
            raw,
        }
    }
}

impl Drop for Ref {
    fn drop(&mut self) {
        // Safety: each Ref owns exactly one handle, released once here.
        unsafe { sys::loaf_value_free(self.raw) }
    }
}
