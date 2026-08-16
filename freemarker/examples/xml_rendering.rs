//! XML node model: parse XML, navigate with hash keys, use <#recurse>/<#visit>
//! macro dispatch, and access @@markup.
//!
//! Run:  cargo run -p freemarker --example xml_rendering
//!
//! Expected output:
//!
//! ```text
//! Root: book
//! Title: Test Book
//! Chapter count: 2
//! Ch1: Introduction
//! Ch2: Conclusion
//! Text of first item: Introduction
//! ```

use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Rc::new(Configuration::new());

    // --- Basic XML navigation ---
    println!("--- XML navigation ---");
    {
        let xml = r#"<book>
  <title>Test Book</title>
  <chapters>
    <chapter id="1"><heading>Introduction</heading></chapter>
    <chapter id="2"><heading>Conclusion</heading></chapter>
  </chapters>
</book>"#;

        let doc = TModel::from_xml_str(xml)?;

        let tpl = parse(
            &cfg,
            "xml_nav",
            r#"Root: ${doc.book?node_name}
Title: ${doc.book.title}
Chapter count: ${doc.book.chapters?children?size}
Ch1: ${doc.book.chapters.chapter[0].heading}
Ch2: ${doc.book.chapters.chapter[1].heading}"#,
        )?;

        let mut root = IndexMap::new();
        root.insert("doc".to_string(), doc);

        let mut out: Vec<u8> = Vec::new();
        tpl.process(TModel::from_hash(root), &mut out)?;
        print!("{}", String::from_utf8(out)?);
    }

    // --- @@text for element text content ---
    println!("\n--- @@text / @@markup ---");
    {
        let xml = r#"<root>
  <item>Introduction</item>
  <item>Conclusion</item>
</root>"#;

        let doc = TModel::from_xml_str(xml)?;

        let tpl = parse(
            &cfg,
            "xml_text",
            r#"Text of first item: ${doc.root.item[0].@@text}"#,
        )?;

        let mut root = IndexMap::new();
        root.insert("doc".to_string(), doc);

        let mut out: Vec<u8> = Vec::new();
        tpl.process(TModel::from_hash(root), &mut out)?;
        print!("{}", String::from_utf8(out)?);
    }

    Ok(())
}
