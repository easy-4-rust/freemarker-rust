//! Rust FreeMarker probe binary for the L3 dual-engine comparison harness.
//!
//! Usage: probe <template.ftl> [data.json]
//!
//! Renders a FreeMarker template with optional JSON data model and writes
//! the result to stdout. Designed to be called from compare_outputs.py.

use freemarker::cache::FileLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use indexmap::IndexMap;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: probe <template.ftl> [data.json]");
        eprintln!("  Renders a FreeMarker template with optional JSON data.");
        eprintln!("  Output is written to stdout.");
        process::exit(1);
    }

    let template_path = Path::new(&args[1]);
    if !template_path.exists() {
        eprintln!(
            "Error: template file not found: {}",
            template_path.display()
        );
        process::exit(1);
    }

    // Build the data model from JSON if provided
    let root = if args.len() >= 3 {
        let data_path = Path::new(&args[2]);
        match load_json_data(data_path) {
            Ok(model) => model,
            Err(e) => {
                eprintln!("Error loading data file: {e}");
                process::exit(1);
            }
        }
    } else {
        TModel::from_hash(IndexMap::new())
    };

    // Create configuration with FileLoader pointing to template's parent directory
    let template_dir = template_path.parent().unwrap_or_else(|| Path::new("."));
    let template_name = template_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let loader = match FileLoader::new(template_dir) {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "Error creating file loader for {}: {e}",
                template_dir.display()
            );
            process::exit(1);
        }
    };

    let mut cfg = Configuration::new();
    cfg.template_loader = std::sync::Arc::new(loader);

    // Render
    match render(&cfg, &template_name, root) {
        Ok(out) => print!("{}", out),
        Err(e) => {
            eprintln!("Error rendering template: {e}");
            process::exit(2);
        }
    }
}

fn render(cfg: &Configuration, name: &str, root: TModel) -> freemarker::error::Result<String> {
    let t = cfg.get_template(name)?;
    let mut out = Vec::new();
    t.process(root, &mut out)?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

fn load_json_data(path: &Path) -> Result<TModel, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in {}: {e}", path.display()))?;
    Ok(json_to_model(&json))
}

fn json_to_model(v: &serde_json::Value) -> TModel {
    match v {
        serde_json::Value::Null => TModel::nothing(),
        serde_json::Value::Bool(b) => TModel::from_boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TModel::from_number(TNumber::from_i64(i))
            } else if let Some(f) = n.as_f64() {
                // Use Decimal for exact representation when possible
                if f.fract() == 0.0 && f.is_finite() {
                    TModel::from_number(TNumber::from_i64(f as i64))
                } else {
                    TModel::from_number(TNumber::Double(f))
                }
            } else {
                TModel::nothing()
            }
        }
        serde_json::Value::String(s) => TModel::from_scalar(s.clone()),
        serde_json::Value::Array(arr) => {
            let seq: Vec<TModel> = arr.iter().map(json_to_model).collect();
            TModel::from_sequence(seq)
        }
        serde_json::Value::Object(obj) => {
            let mut map = IndexMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_model(v));
            }
            TModel::from_hash(map)
        }
    }
}
