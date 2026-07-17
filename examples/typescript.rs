//! Run TypeScript directly — Bun transpiles it before evaluation.
//!
//! Run with: `cargo run --example typescript --features link` (needs libloaf).

use loaf::{Lang, Runtime};

fn main() -> loaf::Result<()> {
    let rt = Runtime::new()?;

    let src = r#"
        interface Point { x: number; y: number }
        const p: Point = { x: 3, y: 4 };
        Math.sqrt(p.x ** 2 + p.y ** 2)
    "#;

    let dist: f64 = rt.load(src).lang(Lang::Ts).name("point.ts").eval()?;
    println!("distance = {dist}");

    // The `.typescript()` shorthand is equivalent to `.lang(Lang::Ts)`.
    let doubled: Vec<f64> = rt
        .load("[1, 2, 3].map((n: number): number => n * 2)")
        .typescript()
        .eval()?;
    println!("doubled = {doubled:?}");

    Ok(())
}
