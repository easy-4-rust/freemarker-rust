//! Number formatting, boolean formatting, locale, and output format with auto-escaping (HTML).
//!
//! Run:  cargo run -p freemarker --example formatting
//!
//! Expected output:
//!
//! ```text
//! number (default): 1,234.56
//! number (pattern): 1234.56
//! currency: $1,234.56
//! percent: 123,456%
//! boolean: YES / NO
//! HTML auto-escaped: &lt;script&gt;alert(&amp;)&lt;/script&gt;
//! ```

use std::rc::Rc;

use freemarker::core::{AutoEscaping, OutputFormatKind};
use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = Configuration::new();
    cfg.settings.locale = "en_US".to_string();
    cfg.settings.boolean_format = "YES,NO".to_string();

    // Set output format to HTML so ${} auto-escapes by default.
    cfg.settings.output_format = OutputFormatKind::Html;
    cfg.settings.auto_escaping = AutoEscaping::Default;
    let cfg = Rc::new(cfg);

    let tpl = parse(
        &cfg,
        "formatting",
        r#"number (default): ${amount?string.number}
number (pattern): ${amount?string["0.##"]}
currency: ${amount?string.currency}
percent: ${amount?string.percent}
boolean: ${flag?string("YES", "NO")} / ${(!flag)?string("YES", "NO")}
HTML auto-escaped: ${xss}"#,
    )?;

    let mut root = IndexMap::new();
    root.insert(
        "amount".to_string(),
        TModel::from_number(TNumber::Double(1234.56)),
    );
    root.insert("flag".to_string(), TModel::from_boolean(true));
    root.insert(
        "xss".to_string(),
        TModel::from_scalar("<script>alert(&)</script>".to_string()),
    );

    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;
    println!("{}", String::from_utf8(out)?);
    Ok(())
}
