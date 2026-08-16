//! Common FreeMarker built-in functions: string, number, sequence, existence checks
//! (`!`, `??`), and UTF-16 semantics (emoji ?length counts code units).
//!
//! Run:  cargo run -p freemarker --example builtins
//!
//! Expected output:
//!
//! ```text
//! upper: HELLO WORLD
//! lower: hello world
//! length of "abc": 3
//! substring: ello World
//! replace: Hexxo Worxd
//! number round: 4
//! number floor: 3
//! number ceiling: 4
//! seq size: 4
//! seq first: a
//! seq last: d
//! seq join: a-b-c-d
//! seq reversed: d c b a
//! fallback
//! present
//! emoji length (code points): 4
//! pad: ------hello
//! ```

use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Rc::new(Configuration::new());

    let tpl = parse(
        &cfg,
        "builtins",
        r#"upper: ${s?upper_case}
lower: ${s?lower_case}
length of "abc": ${"abc"?length}
substring: ${s?substring(1)}
replace: ${s?replace("l","x")}
number round: ${3.7?round}
number floor: ${3.7?floor}
number ceiling: ${3.2?ceiling}
seq size: ${items?size}
seq first: ${items?first}
seq last: ${items?last}
seq join: ${items?join("-")}
seq reversed: <#list items?reverse as v>${v} </#list>
${missing!"fallback"}
${present!"exists"}
emoji length (code points): ${emoji?length}
pad: ${"hello"?left_pad(11, "-")}"#,
    )?;

    let mut root = IndexMap::new();
    root.insert(
        "s".to_string(),
        TModel::from_scalar("Hello World".to_string()),
    );
    root.insert(
        "items".to_string(),
        TModel::from_sequence(vec![
            TModel::from_scalar("a".to_string()),
            TModel::from_scalar("b".to_string()),
            TModel::from_scalar("c".to_string()),
            TModel::from_scalar("d".to_string()),
        ]),
    );
    // missing: intentionally absent from data model — "!default" provides fallback
    root.insert(
        "present".to_string(),
        TModel::from_scalar("present".to_string()),
    );
    // emoji: two Unicode code points (U+1F600 U+1F680)
    root.insert(
        "emoji".to_string(),
        TModel::from_scalar("\u{1F600}\u{1F680}".to_string()),
    );

    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;
    println!("{}", String::from_utf8(out)?);
    Ok(())
}
