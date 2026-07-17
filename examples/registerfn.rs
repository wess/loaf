//! Expose Rust functions to JavaScript.
//!
//! Run with: `cargo run --example registerfn --features link` (needs libloaf).

use loaf::{Runtime, Variadic};

fn main() -> loaf::Result<()> {
    let rt = Runtime::new()?;

    // Two typed arguments in, one value out.
    let add = rt.create_function(|_, (a, b): (f64, f64)| Ok(a + b))?;
    rt.globals().set("add", add)?;
    let n: f64 = rt.eval("add(20, 22)")?;
    println!("add(20, 22) = {n}");

    // A variadic function: sum any number of arguments.
    let sum = rt.create_function(|_, nums: Variadic<f64>| Ok(nums.iter().sum::<f64>()))?;
    rt.globals().set("sum", sum)?;
    let total: f64 = rt.eval("sum(1, 2, 3, 4, 5)")?;
    println!("sum(1..5) = {total}");

    // A Rust callback that itself fails becomes a thrown JS error.
    let checked = rt.create_function(|_, n: f64| {
        if n < 0.0 {
            Err(loaf::Error::Runtime("no negatives".into()))
        } else {
            Ok(n.sqrt())
        }
    })?;
    rt.globals().set("checkedSqrt", checked)?;
    let caught: String = rt.eval("try { checkedSqrt(-1) } catch (e) { String(e) }")?;
    println!("caught: {caught}");

    Ok(())
}
