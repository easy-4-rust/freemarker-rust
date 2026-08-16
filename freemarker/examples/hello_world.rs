//! Minimal FreeMarker template rendering: a single variable interpolation.
//!
//! Run:  cargo run -p freemarker --example hello_world
//!
//! Expected output:
//!
//! ```text
//! Hello, FreeMarker on Rust!
//! Today's engine version: 2.3.34
//! ```

use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a Configuration (reuse across renders in the same thread).
    let cfg = Rc::new(Configuration::new());

    // 2. Parse a template from inline FTL source.
    //    The name "hello" is the cache key; the text is the FTL source.
    let tpl = parse(
        &cfg,
        "hello",
        "Hello, ${name}!\nToday's engine version: ${.version}",
    )?;

    // 3. Build the data model — a hash with one key "name".
    let mut root = IndexMap::new();
    root.insert(
        "name".to_string(),
        TModel::from_scalar("FreeMarker on Rust".to_string()),
    );

    // 4. Render to a byte buffer, then print as UTF-8.
    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;

    println!("{}", String::from_utf8(out)?);
    Ok(())
}
