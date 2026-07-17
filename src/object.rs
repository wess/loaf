//! JavaScript object handle.

use std::ptr;

use crate::convert::{FromJs, IntoJs};
use crate::error::Result;
use crate::handle::Ref;
use crate::runtime::Runtime;
use crate::sys;
use crate::value::Value;

/// An owned handle to a JavaScript object.
#[derive(Clone)]
pub struct Object(pub(crate) Ref);

impl Object {
    #[inline]
    fn rt(&self) -> Runtime {
        Runtime {
            inner: self.0.rt.clone(),
        }
    }

    /// Read property `key`, converting it to `V`.
    pub fn get<V: FromJs>(&self, key: &str) -> Result<V> {
        let rt = self.rt();
        let mut out = ptr::null_mut();
        let st = unsafe {
            sys::loaf_object_get(
                rt.inner.raw,
                self.0.raw,
                key.as_ptr().cast(),
                key.len(),
                &mut out,
            )
        };
        rt.check(st)?;
        let v = Value::from_raw(&rt.inner, out);
        V::from_js(v, &rt)
    }

    /// Set property `key` to `value`.
    pub fn set(&self, key: &str, value: impl IntoJs) -> Result<()> {
        let rt = self.rt();
        let v = value.into_js(&rt)?;
        let held = v.materialize(&rt.inner);
        let st = unsafe {
            sys::loaf_object_set(
                rt.inner.raw,
                self.0.raw,
                key.as_ptr().cast(),
                key.len(),
                held.raw,
            )
        };
        drop(held);
        rt.check(st)
    }

    /// Whether the object has an own or inherited property `key`.
    pub fn has(&self, key: &str) -> Result<bool> {
        let rt = self.rt();
        let mut out = 0i32;
        let st = unsafe {
            sys::loaf_object_has(
                rt.inner.raw,
                self.0.raw,
                key.as_ptr().cast(),
                key.len(),
                &mut out,
            )
        };
        rt.check(st)?;
        Ok(out != 0)
    }

    /// Delete property `key`.
    pub fn delete(&self, key: &str) -> Result<()> {
        let rt = self.rt();
        let st = unsafe {
            sys::loaf_object_delete(rt.inner.raw, self.0.raw, key.as_ptr().cast(), key.len())
        };
        rt.check(st)
    }

    /// The object's own enumerable string keys (`Object.keys`).
    pub fn keys(&self) -> Result<Vec<String>> {
        let rt = self.rt();
        let mut out = ptr::null_mut();
        let st = unsafe { sys::loaf_object_keys(rt.inner.raw, self.0.raw, &mut out) };
        rt.check(st)?;
        match Value::from_raw(&rt.inner, out) {
            Value::Array(a) => {
                let len = a.len()?;
                let mut keys = Vec::with_capacity(len as usize);
                for i in 0..len {
                    keys.push(a.get::<String>(i)?);
                }
                Ok(keys)
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Call method `name` on this object with `args`, returning `R`.
    pub fn call_method<R: FromJs>(
        &self,
        name: &str,
        args: impl crate::convert::IntoJsMulti,
    ) -> Result<R> {
        let method: crate::function::Function = self.get(name)?;
        method.call_with(Value::Object(self.clone()), args)
    }

    /// View this object as a generic [`Value`].
    pub fn into_value(self) -> Value {
        Value::Object(self)
    }
}

impl std::fmt::Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Object").finish_non_exhaustive()
    }
}
