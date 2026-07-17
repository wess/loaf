//! JavaScript array handle.

use std::ptr;

use crate::convert::{FromJs, IntoJs};
use crate::error::Result;
use crate::handle::Ref;
use crate::runtime::Runtime;
use crate::sys;
use crate::value::Value;

/// An owned handle to a JavaScript array.
#[derive(Clone)]
pub struct Array(pub(crate) Ref);

impl Array {
    #[inline]
    fn rt(&self) -> Runtime {
        Runtime {
            inner: self.0.rt.clone(),
        }
    }

    /// The array's `length`.
    pub fn len(&self) -> Result<u32> {
        let rt = self.rt();
        let mut out = 0u32;
        let st = unsafe { sys::loaf_array_length(rt.inner.raw, self.0.raw, &mut out) };
        rt.check(st)?;
        Ok(out)
    }

    /// Whether `length` is zero.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Read element `index`, converting it to `V`.
    pub fn get<V: FromJs>(&self, index: u32) -> Result<V> {
        let rt = self.rt();
        let mut out = ptr::null_mut();
        let st = unsafe { sys::loaf_array_get(rt.inner.raw, self.0.raw, index, &mut out) };
        rt.check(st)?;
        let v = Value::from_raw(&rt.inner, out);
        V::from_js(v, &rt)
    }

    /// Set element `index` to `value`.
    pub fn set(&self, index: u32, value: impl IntoJs) -> Result<()> {
        let rt = self.rt();
        let held = value.into_js(&rt)?.materialize(&rt.inner);
        let st = unsafe { sys::loaf_array_set(rt.inner.raw, self.0.raw, index, held.raw) };
        drop(held);
        rt.check(st)
    }

    /// Append `value` to the end of the array.
    pub fn push(&self, value: impl IntoJs) -> Result<()> {
        let rt = self.rt();
        let held = value.into_js(&rt)?.materialize(&rt.inner);
        let st = unsafe { sys::loaf_array_push(rt.inner.raw, self.0.raw, held.raw) };
        drop(held);
        rt.check(st)
    }

    /// View this array as a generic [`Value`].
    pub fn into_value(self) -> Value {
        Value::Array(self)
    }
}

impl std::fmt::Debug for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Array").finish_non_exhaustive()
    }
}
