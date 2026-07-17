//! Move objects and arrays between Rust and JavaScript.
//!
//! Run with: `cargo run --example hostobject --features link` (needs libloaf).

use std::collections::HashMap;

use loaf::Runtime;

fn main() -> loaf::Result<()> {
    let rt = Runtime::new()?;

    // Build a JS object in Rust and expose it to script.
    let config = rt.create_object();
    config.set("name", "loaf")?;
    config.set("version", 1.0)?;
    rt.globals().set("config", config)?;

    let name: String = rt.eval("config.name")?;
    println!("config.name = {name}");

    // Read a whole JS object back into a Rust map.
    let scores: HashMap<String, f64> = rt.eval("({ alice: 10, bob: 7, carol: 9 })")?;
    println!("scores = {scores:?}");

    // Pass a Rust Vec in, get a transformed Vec back.
    let nums = vec![1.0f64, 2.0, 3.0];
    rt.globals().set("nums", nums)?;
    let squared: Vec<f64> = rt.eval("nums.map((n) => n * n)")?;
    println!("squared = {squared:?}");

    Ok(())
}
