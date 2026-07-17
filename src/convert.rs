//! Conversions between Rust types and JavaScript [`Value`]s.
//!
//! Four traits, mirroring mlua:
//! - [`IntoJs`] / [`FromJs`] convert a single value.
//! - [`IntoJsMulti`] / [`FromJsMulti`] convert *lists* of values, which is how
//!   function arguments and multiple return values are handled. Every
//!   `IntoJs`/`FromJs` type is automatically a single-element multi, and tuples
//!   up to 16 elements spread positionally.

use std::collections::HashMap;

use crate::array::Array;
use crate::error::{Error, Result};
use crate::function::Function;
use crate::object::Object;
use crate::runtime::Runtime;
use crate::string::JsString;
use crate::value::Value;

/// Convert a Rust value into a JS [`Value`].
pub trait IntoJs {
    fn into_js(self, rt: &Runtime) -> Result<Value>;
}

/// Convert a JS [`Value`] into a Rust value.
pub trait FromJs: Sized {
    fn from_js(value: Value, rt: &Runtime) -> Result<Self>;
}

/// Convert a Rust value into a list of JS values (function arguments / returns).
pub trait IntoJsMulti {
    fn into_js_multi(self, rt: &Runtime) -> Result<Vec<Value>>;
}

/// Convert a list of JS values into a Rust value (function arguments / returns).
pub trait FromJsMulti: Sized {
    fn from_js_multi(values: Vec<Value>, rt: &Runtime) -> Result<Self>;
}

// Every single value is a one-element multi.
impl<T: IntoJs> IntoJsMulti for T {
    fn into_js_multi(self, rt: &Runtime) -> Result<Vec<Value>> {
        Ok(vec![self.into_js(rt)?])
    }
}

impl<T: FromJs> FromJsMulti for T {
    fn from_js_multi(values: Vec<Value>, rt: &Runtime) -> Result<Self> {
        let v = values.into_iter().next().unwrap_or(Value::Undefined);
        T::from_js(v, rt)
    }
}

fn expect_number(value: &Value, to: &'static str) -> Result<f64> {
    match value {
        Value::Number(n) => Ok(*n),
        other => Err(Error::from_js(
            other.type_of().name(),
            to,
            "expected a number",
        )),
    }
}

macro_rules! impl_int {
    ($($t:ty),* $(,)?) => {$(
        impl IntoJs for $t {
            fn into_js(self, _rt: &Runtime) -> Result<Value> {
                Ok(Value::Number(self as f64))
            }
        }
        impl FromJs for $t {
            fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
                let n = expect_number(&value, stringify!($t))?;
                if !n.is_finite() || n.fract() != 0.0 {
                    return Err(Error::from_js("number", stringify!($t), "expected an integer"));
                }
                if n < <$t>::MIN as f64 || n > <$t>::MAX as f64 {
                    return Err(Error::from_js("number", stringify!($t), "out of range"));
                }
                Ok(n as $t)
            }
        }
    )*};
}
impl_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl IntoJs for f64 {
    fn into_js(self, _rt: &Runtime) -> Result<Value> {
        Ok(Value::Number(self))
    }
}
impl FromJs for f64 {
    fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
        expect_number(&value, "f64")
    }
}

impl IntoJs for f32 {
    fn into_js(self, _rt: &Runtime) -> Result<Value> {
        Ok(Value::Number(self as f64))
    }
}
impl FromJs for f32 {
    fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
        Ok(expect_number(&value, "f32")? as f32)
    }
}

impl IntoJs for bool {
    fn into_js(self, _rt: &Runtime) -> Result<Value> {
        Ok(Value::Boolean(self))
    }
}
impl FromJs for bool {
    fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
        match value {
            Value::Boolean(b) => Ok(b),
            other => Err(Error::from_js(
                other.type_of().name(),
                "bool",
                "expected a boolean",
            )),
        }
    }
}

impl IntoJs for String {
    fn into_js(self, rt: &Runtime) -> Result<Value> {
        Ok(Value::String(rt.create_string(&self)))
    }
}
impl IntoJs for &str {
    fn into_js(self, rt: &Runtime) -> Result<Value> {
        Ok(Value::String(rt.create_string(self)))
    }
}
impl FromJs for String {
    fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
        match value {
            Value::String(s) => s.to_str(),
            other => Err(Error::from_js(
                other.type_of().name(),
                "String",
                "expected a string",
            )),
        }
    }
}

impl IntoJs for char {
    fn into_js(self, rt: &Runtime) -> Result<Value> {
        let mut buf = [0u8; 4];
        Ok(Value::String(rt.create_string(self.encode_utf8(&mut buf))))
    }
}
impl FromJs for char {
    fn from_js(value: Value, rt: &Runtime) -> Result<Self> {
        let s = String::from_js(value, rt)?;
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(Error::from_js(
                "string",
                "char",
                "expected a single character",
            )),
        }
    }
}

impl IntoJs for Value {
    fn into_js(self, _rt: &Runtime) -> Result<Value> {
        Ok(self)
    }
}
impl FromJs for Value {
    fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
        Ok(value)
    }
}

impl IntoJs for () {
    fn into_js(self, _rt: &Runtime) -> Result<Value> {
        Ok(Value::Undefined)
    }
}
impl FromJs for () {
    fn from_js(_value: Value, _rt: &Runtime) -> Result<Self> {
        Ok(())
    }
}

macro_rules! impl_handle {
    ($ty:ident, $variant:ident) => {
        impl IntoJs for $ty {
            fn into_js(self, _rt: &Runtime) -> Result<Value> {
                Ok(Value::$variant(self))
            }
        }
        impl FromJs for $ty {
            fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
                match value {
                    Value::$variant(v) => Ok(v),
                    other => Err(Error::from_js(
                        other.type_of().name(),
                        stringify!($ty),
                        concat!("expected a ", stringify!($variant)),
                    )),
                }
            }
        }
    };
}
impl_handle!(JsString, String);
impl_handle!(Object, Object);
impl_handle!(Array, Array);
impl_handle!(Function, Function);

impl<T: IntoJs> IntoJs for Option<T> {
    fn into_js(self, rt: &Runtime) -> Result<Value> {
        match self {
            Some(v) => v.into_js(rt),
            None => Ok(Value::Null),
        }
    }
}
impl<T: FromJs> FromJs for Option<T> {
    fn from_js(value: Value, rt: &Runtime) -> Result<Self> {
        if value.is_nullish() {
            Ok(None)
        } else {
            Ok(Some(T::from_js(value, rt)?))
        }
    }
}

impl<T: IntoJs> IntoJs for Vec<T> {
    fn into_js(self, rt: &Runtime) -> Result<Value> {
        let arr = rt.create_array();
        for item in self {
            let v = item.into_js(rt)?;
            arr.push(v)?;
        }
        Ok(Value::Array(arr))
    }
}
impl<T: FromJs> FromJs for Vec<T> {
    fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
        match value {
            Value::Array(a) => {
                let len = a.len()?;
                let mut out = Vec::with_capacity(len as usize);
                for i in 0..len {
                    out.push(a.get::<T>(i)?);
                }
                Ok(out)
            }
            other => Err(Error::from_js(
                other.type_of().name(),
                "Vec",
                "expected an array",
            )),
        }
    }
}

impl<V: IntoJs> IntoJs for HashMap<String, V> {
    fn into_js(self, rt: &Runtime) -> Result<Value> {
        let obj = rt.create_object();
        for (k, val) in self {
            obj.set(&k, val)?;
        }
        Ok(Value::Object(obj))
    }
}
impl<V: FromJs> FromJs for HashMap<String, V> {
    fn from_js(value: Value, _rt: &Runtime) -> Result<Self> {
        match value {
            Value::Object(o) => {
                let mut map = HashMap::new();
                for k in o.keys()? {
                    let val = o.get::<V>(&k)?;
                    map.insert(k, val);
                }
                Ok(map)
            }
            other => Err(Error::from_js(
                other.type_of().name(),
                "HashMap",
                "expected an object",
            )),
        }
    }
}

/// Wraps a `Vec<T>` so it spreads to/from any number of JS values (a trailing
/// `...args`). Use it as the argument type of a variadic host function.
#[derive(Debug, Clone)]
pub struct Variadic<T>(pub Vec<T>);

impl<T> Variadic<T> {
    pub fn new() -> Self {
        Variadic(Vec::new())
    }
}

impl<T> Default for Variadic<T> {
    fn default() -> Self {
        Variadic(Vec::new())
    }
}

impl<T> std::ops::Deref for Variadic<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}
impl<T> std::ops::DerefMut for Variadic<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}
impl<T> From<Vec<T>> for Variadic<T> {
    fn from(v: Vec<T>) -> Self {
        Variadic(v)
    }
}

impl<T: FromJs> FromJsMulti for Variadic<T> {
    fn from_js_multi(values: Vec<Value>, rt: &Runtime) -> Result<Self> {
        let mut out = Vec::with_capacity(values.len());
        for v in values {
            out.push(T::from_js(v, rt)?);
        }
        Ok(Variadic(out))
    }
}
impl<T: IntoJs> IntoJsMulti for Variadic<T> {
    fn into_js_multi(self, rt: &Runtime) -> Result<Vec<Value>> {
        let mut out = Vec::with_capacity(self.0.len());
        for v in self.0 {
            out.push(v.into_js(rt)?);
        }
        Ok(out)
    }
}

macro_rules! impl_tuple {
    ($($name:ident),+) => {
        impl<$($name: IntoJs),+> IntoJsMulti for ($($name,)+) {
            #[allow(non_snake_case)]
            fn into_js_multi(self, rt: &Runtime) -> Result<Vec<Value>> {
                let ($($name,)+) = self;
                Ok(vec![$($name.into_js(rt)?),+])
            }
        }
        impl<$($name: FromJs),+> FromJsMulti for ($($name,)+) {
            fn from_js_multi(values: Vec<Value>, rt: &Runtime) -> Result<Self> {
                let mut it = values.into_iter();
                Ok((
                    $( <$name as FromJs>::from_js(it.next().unwrap_or(Value::Undefined), rt)?, )+
                ))
            }
        }
    };
}

impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
