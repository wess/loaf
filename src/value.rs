//! The [`Value`] type: a Rust-side view of any JavaScript value.

use std::rc::Rc;

use crate::array::Array;
use crate::function::Function;
use crate::handle::{Inner, Ref};
use crate::object::Object;
use crate::string::JsString;
use crate::sys;

/// The runtime type of a JS value, mirroring `typeof` plus the distinctions
/// loaf cares about (arrays and functions are called out from plain objects).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Undefined,
    Null,
    Boolean,
    Number,
    String,
    Symbol,
    Object,
    Array,
    Function,
    Promise,
    BigInt,
    Other,
}

impl ValueType {
    pub(crate) fn from_sys(t: sys::LoafType) -> ValueType {
        match t {
            sys::LoafType::Undefined => ValueType::Undefined,
            sys::LoafType::Null => ValueType::Null,
            sys::LoafType::Boolean => ValueType::Boolean,
            sys::LoafType::Number => ValueType::Number,
            sys::LoafType::String => ValueType::String,
            sys::LoafType::Symbol => ValueType::Symbol,
            sys::LoafType::Object => ValueType::Object,
            sys::LoafType::Array => ValueType::Array,
            sys::LoafType::Function => ValueType::Function,
            sys::LoafType::Promise => ValueType::Promise,
            sys::LoafType::BigInt => ValueType::BigInt,
            sys::LoafType::Other => ValueType::Other,
        }
    }

    /// The name of this type as `typeof` would roughly report it.
    pub fn name(self) -> &'static str {
        match self {
            ValueType::Undefined => "undefined",
            ValueType::Null => "null",
            ValueType::Boolean => "boolean",
            ValueType::Number => "number",
            ValueType::String => "string",
            ValueType::Symbol => "symbol",
            ValueType::Object => "object",
            ValueType::Array => "array",
            ValueType::Function => "function",
            ValueType::Promise => "promise",
            ValueType::BigInt => "bigint",
            ValueType::Other => "value",
        }
    }
}

/// A JavaScript value held on the Rust side.
///
/// Primitives are stored inline. Heap values (strings, objects, arrays,
/// functions, and everything else) are owned, GC-rooted handles that free
/// themselves on drop and keep their runtime alive.
#[derive(Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(JsString),
    Object(Object),
    Array(Array),
    Function(Function),
    /// Symbols, BigInts, promises, dates, and other exotic values kept as an
    /// opaque handle.
    Other(Other),
}

/// An opaque handle to a JS value that is not one of the first-class variants.
#[derive(Clone)]
pub struct Other(pub(crate) Ref);

impl Other {
    /// The runtime type of this value.
    pub fn type_of(&self) -> ValueType {
        let t = unsafe { sys::loaf_value_type(self.0.rt_raw(), self.0.raw) };
        ValueType::from_sys(t)
    }

    /// Whether this value is a `Promise`.
    pub fn is_promise(&self) -> bool {
        unsafe { sys::loaf_is_promise(self.0.rt_raw(), self.0.raw) != 0 }
    }
}

impl Value {
    /// Consume an owned FFI handle and classify it into a [`Value`]. Primitive
    /// handles are read out and freed here; heap handles are retained.
    pub(crate) fn from_raw(rt: &Rc<Inner>, raw: *mut sys::LoafValue) -> Value {
        let ty = unsafe { sys::loaf_value_type(rt.raw, raw) };
        match ty {
            sys::LoafType::Undefined => {
                unsafe { sys::loaf_value_free(raw) };
                Value::Undefined
            }
            sys::LoafType::Null => {
                unsafe { sys::loaf_value_free(raw) };
                Value::Null
            }
            sys::LoafType::Boolean => {
                let b = unsafe { sys::loaf_truthy(rt.raw, raw) } != 0;
                unsafe { sys::loaf_value_free(raw) };
                Value::Boolean(b)
            }
            sys::LoafType::Number => {
                let mut n = 0.0f64;
                unsafe { sys::loaf_get_number(rt.raw, raw, &mut n) };
                unsafe { sys::loaf_value_free(raw) };
                Value::Number(n)
            }
            sys::LoafType::String => Value::String(JsString(Ref::from_owned(rt, raw))),
            sys::LoafType::Array => Value::Array(Array(Ref::from_owned(rt, raw))),
            sys::LoafType::Function => Value::Function(Function(Ref::from_owned(rt, raw))),
            sys::LoafType::Object | sys::LoafType::Promise => {
                Value::Object(Object(Ref::from_owned(rt, raw)))
            }
            _ => Value::Other(Other(Ref::from_owned(rt, raw))),
        }
    }

    /// Consume this value into an owned handle in `rt`. Heap values donate their
    /// existing handle (no duplication); primitives construct a fresh one.
    pub(crate) fn materialize(self, rt: &Rc<Inner>) -> Ref {
        match self {
            Value::Undefined => Ref::from_owned(rt, unsafe { sys::loaf_undefined(rt.raw) }),
            Value::Null => Ref::from_owned(rt, unsafe { sys::loaf_null(rt.raw) }),
            Value::Boolean(b) => {
                Ref::from_owned(rt, unsafe { sys::loaf_boolean(rt.raw, b as i32) })
            }
            Value::Number(n) => Ref::from_owned(rt, unsafe { sys::loaf_number(rt.raw, n) }),
            Value::String(s) => s.0,
            Value::Object(o) => o.0,
            Value::Array(a) => a.0,
            Value::Function(f) => f.0,
            Value::Other(o) => o.0,
        }
    }

    /// Consume this value into a raw owned handle, transferring ownership to the
    /// caller (used for FFI out-parameters and host-function returns).
    pub(crate) fn into_raw_owned(self, rt: &Rc<Inner>) -> *mut sys::LoafValue {
        let r = self.materialize(rt);
        let raw = r.raw;
        std::mem::forget(r);
        raw
    }

    /// The runtime type of this value.
    pub fn type_of(&self) -> ValueType {
        match self {
            Value::Undefined => ValueType::Undefined,
            Value::Null => ValueType::Null,
            Value::Boolean(_) => ValueType::Boolean,
            Value::Number(_) => ValueType::Number,
            Value::String(_) => ValueType::String,
            Value::Object(_) => ValueType::Object,
            Value::Array(_) => ValueType::Array,
            Value::Function(_) => ValueType::Function,
            Value::Other(o) => o.type_of(),
        }
    }

    pub fn is_undefined(&self) -> bool {
        matches!(self, Value::Undefined)
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    /// True for both `null` and `undefined` (JS "nullish").
    pub fn is_nullish(&self) -> bool {
        matches!(self, Value::Null | Value::Undefined)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_string(&self) -> Option<&JsString> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&Array> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_function(&self) -> Option<&Function> {
        match self {
            Value::Function(f) => Some(f),
            _ => None,
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Undefined => write!(f, "undefined"),
            Value::Null => write!(f, "null"),
            Value::Boolean(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(_) => write!(f, "String(..)"),
            Value::Object(_) => write!(f, "Object(..)"),
            Value::Array(_) => write!(f, "Array(..)"),
            Value::Function(_) => write!(f, "Function(..)"),
            Value::Other(o) => write!(f, "{}(..)", o.type_of().name()),
        }
    }
}

impl std::fmt::Debug for Other {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Other({})", self.type_of().name())
    }
}
