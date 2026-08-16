//! FreeMarker directives: <#if>, <#list> (with index/has_next), <#macro> + <#nested>,
//! <#include> (via StringLoader), and <#attempt>/<#recover>.
//!
//! Run:  cargo run -p freemarker --example directives
//!
//! Expected output:
//!
//! ```text
//! === if / list ===
//! [0] alpha (first)
//! [1] beta
//! [2] gamma (last)
//! === macro + nested ===
//! >> begin
//! |   body content
//! << end
//! === include ===
//! -- included: Hello from included.ftl --
//! === attempt / recover ===
//!   [RECOVERED] Template inclusion failed ...
//! ```

use std::rc::Rc;
use std::sync::Arc;

use freemarker::cache::StringLoader;
use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- if / list with loop variables ---
    println!("=== if / list ===");
    {
        let cfg = Rc::new(Configuration::new());
        let tpl = parse(
            &cfg,
            "if_list",
            r#"<#list items as item>[${item?index}] ${item}<#if !item_has_next> (last)<#elseif item?index == 0> (first)</#if>
</#list>"#,
        )?;
        let mut root = IndexMap::new();
        root.insert(
            "items".to_string(),
            TModel::from_sequence(vec![
                TModel::from_scalar("alpha".to_string()),
                TModel::from_scalar("beta".to_string()),
                TModel::from_scalar("gamma".to_string()),
            ]),
        );
        let mut out: Vec<u8> = Vec::new();
        tpl.process(TModel::from_hash(root), &mut out)?;
        print!("{}", String::from_utf8(out)?);
    }

    // --- macro + nested body ---
    println!("=== macro + nested ===");
    {
        let cfg = Rc::new(Configuration::new());
        let tpl = parse(
            &cfg,
            "macro_demo",
            r#"<#macro framed>
>> begin
|   <#nested>
<< end
</#macro>
<@framed>body content</@framed>"#,
        )?;
        let mut out: Vec<u8> = Vec::new();
        tpl.process(TModel::from_hash(IndexMap::new()), &mut out)?;
        print!("{}", String::from_utf8(out)?);
    }

    // --- include via StringLoader ---
    println!("=== include ===");
    {
        let mut cfg = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        loader.put("included.ftl", "-- included: ${message} --");
        cfg.template_loader = loader;
        let cfg = Rc::new(cfg);

        let tpl = parse(
            &cfg,
            "include_demo",
            r#"<#assign message = "Hello from included.ftl">
<#include "included.ftl">"#,
        )?;
        let mut out: Vec<u8> = Vec::new();
        tpl.process(TModel::from_hash(IndexMap::new()), &mut out)?;
        print!("{}", String::from_utf8(out)?);
    }

    // --- attempt / recover ---
    println!("=== attempt / recover ===");
    {
        let cfg = Rc::new(Configuration::new());
        let tpl = parse(
            &cfg,
            "attempt_demo",
            r#"<#attempt>
  <#include "does_not_exist.ftl">
<#recover>
  [RECOVERED] ${.error}
</#attempt>"#,
        )?;
        let mut out: Vec<u8> = Vec::new();
        tpl.process(TModel::from_hash(IndexMap::new()), &mut out)?;
        println!("{}", String::from_utf8(out)?);
    }

    Ok(())
}
