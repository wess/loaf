//! End-to-end tests that actually execute JavaScript. They only build and run
//! when the native library is linked (`--features link` with `LOAF_LIB_DIR`
//! set, or `--features build-from-source`). Without `link` this file is empty,
//! so `cargo test` still passes on platforms where Bun's native build is not
//! yet available.
#![cfg(feature = "link")]

use std::collections::HashMap;

use loaf::{Lang, Runtime, Variadic};

#[test]
fn eval_arithmetic() {
    let rt = Runtime::new().unwrap();
    let n: f64 = rt.eval("1 + 2 * 3").unwrap();
    assert_eq!(n, 7.0);
}

#[test]
fn eval_string() {
    let rt = Runtime::new().unwrap();
    let s: String = rt.eval("`a${1 + 1}b`").unwrap();
    assert_eq!(s, "a2b");
}

#[test]
fn round_trip_globals() {
    let rt = Runtime::new().unwrap();
    rt.globals().set("x", 21.0).unwrap();
    let n: f64 = rt.eval("x * 2").unwrap();
    assert_eq!(n, 42.0);
}

#[test]
fn register_function() {
    let rt = Runtime::new().unwrap();
    let add = rt
        .create_function(|_, (a, b): (f64, f64)| Ok(a + b))
        .unwrap();
    rt.globals().set("add", add).unwrap();
    let n: f64 = rt.eval("add(20, 22)").unwrap();
    assert_eq!(n, 42.0);
}

#[test]
fn variadic_function() {
    let rt = Runtime::new().unwrap();
    let sum = rt
        .create_function(|_, xs: Variadic<f64>| Ok(xs.iter().sum::<f64>()))
        .unwrap();
    rt.globals().set("sum", sum).unwrap();
    let n: f64 = rt.eval("sum(1, 2, 3, 4)").unwrap();
    assert_eq!(n, 10.0);
}

#[test]
fn typescript_transpiles() {
    let rt = Runtime::new().unwrap();
    let n: f64 = rt
        .load("const x: number = 40; x + 2")
        .lang(Lang::Ts)
        .eval()
        .unwrap();
    assert_eq!(n, 42.0);
}

#[test]
fn object_into_hashmap() {
    let rt = Runtime::new().unwrap();
    let map: HashMap<String, f64> = rt.eval("({ a: 1, b: 2 })").unwrap();
    assert_eq!(map.get("a"), Some(&1.0));
    assert_eq!(map.get("b"), Some(&2.0));
}

#[test]
fn exceptions_surface_as_errors() {
    let rt = Runtime::new().unwrap();
    let err = rt.eval::<f64>("throw new Error('boom')").unwrap_err();
    assert!(err.to_string().contains("boom"));
}

#[test]
fn await_a_promise() {
    let rt = Runtime::new().unwrap();
    let promise = rt.eval::<loaf::Value>("Promise.resolve(123)").unwrap();
    let n: f64 = rt.await_value(promise).unwrap();
    assert_eq!(n, 123.0);
}
