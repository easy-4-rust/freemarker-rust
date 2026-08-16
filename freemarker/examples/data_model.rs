//! Demonstrates the TModel data model: scalars, numbers (with BigDecimal precision),
//! booleans, nested hashes, sequences, and ranges.
//!
//! Run:  cargo run -p freemarker --example data_model
//!
//! Expected output:
//!
//! ```text
//! User: Alice (age 30, active? yes)
//! Pi (BigDecimal): 3.142
//! Items: Rust, FreeMarker, FTL (3 total)
//! Nested: nested value
//! Range: 1, 2, 3, 4, 5
//! ```

use std::rc::Rc;

use bigdecimal::BigDecimal;
use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use indexmap::IndexMap;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Rc::new(Configuration::new());

    let tpl = parse(
        &cfg,
        "data_model",
        r#"User: ${user.name} (age ${user.age}, active? ${user.active?string("yes", "no")})
Pi (BigDecimal): ${pi}
Items: <#list items as item>${item}<#if item_has_next>, </#if></#list> (${items?size} total)
Nested: ${root.child.key}
Range: <#list 1..5 as r>${r}<#if r_has_next>, </#if></#list>"#,
    )?;

    // Scalar
    let mut user = IndexMap::new();
    user.insert("name".to_string(), TModel::from_scalar("Alice".to_string()));

    // Number: i32 via TNumber::Int
    user.insert("age".to_string(), TModel::from_number(TNumber::Int(30)));

    // Boolean
    user.insert("active".to_string(), TModel::from_boolean(true));

    // BigDecimal with high precision
    let pi: BigDecimal = BigDecimal::from_str("3.14159265358979323846").unwrap_or_default();

    // Sequence of scalars
    let items = TModel::from_sequence(vec![
        TModel::from_scalar("Rust".to_string()),
        TModel::from_scalar("FreeMarker".to_string()),
        TModel::from_scalar("FTL".to_string()),
    ]);

    // Nested hash
    let mut child = IndexMap::new();
    child.insert(
        "key".to_string(),
        TModel::from_scalar("nested value".to_string()),
    );
    let mut root_hash = IndexMap::new();
    root_hash.insert("child".to_string(), TModel::from_hash(child));

    // Assemble root data model
    let mut root = IndexMap::new();
    root.insert("user".to_string(), TModel::from_hash(user));
    root.insert("pi".to_string(), TModel::from_number(TNumber::Decimal(pi)));
    root.insert("items".to_string(), items);
    root.insert("root".to_string(), TModel::from_hash(root_hash));

    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;
    println!("{}", String::from_utf8(out)?);
    Ok(())
}
