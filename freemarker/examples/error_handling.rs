//! Error handling: parse errors and runtime errors produce structured TemplateError
//! messages with template name, line, column, and FTL stack trace. Demonstrates
//! matching on Result and extracting error details.
//!
//! Run:  cargo run -p freemarker --example error_handling
//!
//! Expected output:
//!
//! ```text
//! Parse error (expected):
//!   Syntax error in template "bad_syntax" in line 1, column 2:
//! Runtime error (expected):
//!   The following has evaluated to null or missing:
//!   ==> missing_var  [in template "runtime_error" at line 1, column 3]
//! ```

use std::rc::Rc;

use freemarker::error::TemplateError;
use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() {
    let cfg = Rc::new(Configuration::new());

    // --- Parse error: malformed FTL syntax ---
    println!("Parse error (expected):");
    match parse(&cfg, "bad_syntax", r#"<#if>"#) {
        Ok(_) => println!("  UNEXPECTED: parse succeeded"),
        Err(e) => {
            // TemplateError::Debug output contains template name, line, column
            let msg = format!("{e}");
            // Show just the first line for brevity
            if let Some(first_line) = msg.lines().next() {
                println!("  {first_line}");
            }
        }
    }

    // --- Runtime error: missing variable ---
    println!("Runtime error (expected):");
    match parse(&cfg, "runtime_error", "${missing_var}") {
        Ok(tpl) => {
            let mut out: Vec<u8> = Vec::new();
            match tpl.process(TModel::from_hash(IndexMap::new()), &mut out) {
                Ok(_) => println!("  UNEXPECTED: render succeeded"),
                Err(e) => {
                    let msg = format!("{e}");
                    // Show first two lines: the "evaluated to null or missing" header
                    for line in msg.lines().take(2) {
                        println!("  {line}");
                    }
                }
            }
        }
        Err(e) => println!("  parse failed: {e}"),
    }

    // --- Pattern: match on error variant ---
    println!("\nPattern: match on TemplateError variant:");
    match parse(&cfg, "bad2", "<#list>") {
        Ok(_) => {}
        Err(e) => match &e {
            TemplateError::Parse {
                template,
                line,
                col,
                ..
            } => {
                println!(
                    "  Parse error in template {:?} at line {}, column {}",
                    template, line, col
                );
            }
            _ => println!("  Other error: {e}"),
        },
    }
}
