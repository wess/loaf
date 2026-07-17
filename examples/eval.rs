//! Evaluate expressions and convert results to Rust types.
//!
//! Run with: `cargo run --example eval --features link` (needs libloaf).

use loaf::Runtime;

fn main() -> loaf::Result<()> {
    let rt = Runtime::new()?;

    let sum: f64 = rt.eval("1 + 2 + 3")?;
    println!("1 + 2 + 3 = {sum}");

    let msg: String = rt.eval("`hello from ${'bun'}`")?;
    println!("{msg}");

    let flag: bool = rt.eval("3 > 2")?;
    println!("3 > 2 = {flag}");

    Ok(())
}
