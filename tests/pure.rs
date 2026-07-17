//! Tests that exercise loaf's pure logic — no runtime, so they run everywhere,
//! including where the native library cannot be linked.

use loaf::{Error, ValueType, Variadic};

#[test]
fn value_type_names() {
    assert_eq!(ValueType::Undefined.name(), "undefined");
    assert_eq!(ValueType::Null.name(), "null");
    assert_eq!(ValueType::Number.name(), "number");
    assert_eq!(ValueType::Function.name(), "function");
    assert_eq!(ValueType::Array.name(), "array");
}

#[test]
fn error_display() {
    let e = Error::Syntax("unexpected token".into());
    assert_eq!(e.to_string(), "syntax error: unexpected token");

    let e = Error::from_js("number", "String", "expected a string");
    assert_eq!(
        e.to_string(),
        "cannot convert JS number to Rust String: expected a string"
    );
}

#[test]
fn variadic_is_a_vec() {
    let mut v: Variadic<i32> = Variadic::default();
    v.push(1);
    v.push(2);
    assert_eq!(v.len(), 2);
    assert_eq!(v.iter().sum::<i32>(), 3);

    let from_vec: Variadic<i32> = vec![4, 5, 6].into();
    assert_eq!(from_vec.iter().copied().max(), Some(6));
}
