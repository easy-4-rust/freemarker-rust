//! Criterion performance benchmarks for the freemarker template engine.
//!
//! Each benchmark measures the isolated `Template::process()` call after
//! template parsing and data model construction (excluded from the timed region).

use criterion::{criterion_group, criterion_main, Criterion};
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helper: create a Configuration with a StringLoader, register one template,
// parse it, and return the parsed Template together with the loader (so the
// loader stays alive).
// ---------------------------------------------------------------------------

fn setup_config(template_name: &str, template_text: &str) -> (Configuration, Arc<StringLoader>) {
    let mut cfg = Configuration::new();
    let loader = Arc::new(StringLoader::default());
    loader.put(template_name, template_text);
    cfg.template_loader = loader.clone();
    (cfg, loader)
}

// ---------------------------------------------------------------------------
// Benchmark 1: simple_hello_world
// Render `${message}` with data model {"message": "Hello, World!"}
// ---------------------------------------------------------------------------

fn bench_simple_hello_world(c: &mut Criterion) {
    let (cfg, _loader) = setup_config("hello", "${message}");
    let t = cfg.get_template("hello").unwrap();

    let mut root_map = IndexMap::new();
    root_map.insert(
        "message".to_string(),
        TModel::from_scalar("Hello, World!".to_string()),
    );
    let root = TModel::from_hash(root_map);

    c.bench_function("simple_hello_world", |b| {
        b.iter(|| {
            t.process(root.clone(), &mut std::io::sink()).unwrap();
        })
    });
}

// ---------------------------------------------------------------------------
// Benchmark 2: simple_loop_100
// Render a template that loops 100 times outputting a number using the
// built-in range operator `1..100`.
// ---------------------------------------------------------------------------

fn bench_simple_loop_100(c: &mut Criterion) {
    let (cfg, _loader) = setup_config(
        "loop100",
        "<#list 1..100 as i>${i}</#list>",
    );
    let t = cfg.get_template("loop100").unwrap();
    let root = TModel::from_hash(IndexMap::new());

    c.bench_function("simple_loop_100", |b| {
        b.iter(|| {
            t.process(root.clone(), &mut std::io::sink()).unwrap();
        })
    });
}

// ---------------------------------------------------------------------------
// Benchmark 3: if_else_chain
// Render 10 if/elseif conditions. The data model sets `x = 5` so evaluation
// walks through the first 5 branches before hitting the match.
// ---------------------------------------------------------------------------

fn bench_if_else_chain(c: &mut Criterion) {
    let template_text = concat!(
        "<#if x == 1>A",
        "<#elseif x == 2>B",
        "<#elseif x == 3>C",
        "<#elseif x == 4>D",
        "<#elseif x == 5>E",
        "<#elseif x == 6>F",
        "<#elseif x == 7>G",
        "<#elseif x == 8>H",
        "<#elseif x == 9>I",
        "<#else>J</#if>",
    );
    let (cfg, _loader) = setup_config("ifchain", template_text);
    let t = cfg.get_template("ifchain").unwrap();

    let mut root_map = IndexMap::new();
    root_map.insert("x".to_string(), TModel::from_scalar("5".to_string()));
    let root = TModel::from_hash(root_map);

    c.bench_function("if_else_chain", |b| {
        b.iter(|| {
            t.process(root.clone(), &mut std::io::sink()).unwrap();
        })
    });
}

// ---------------------------------------------------------------------------
// Benchmark 4: macro_call_100
// Define a macro and call it 100 times using the range operator.
// ---------------------------------------------------------------------------

fn bench_macro_call_100(c: &mut Criterion) {
    let template_text =
        "<#macro m>hello</#macro><#list 1..100 as i><@m/></#list>";
    let (cfg, _loader) = setup_config("macro100", template_text);
    let t = cfg.get_template("macro100").unwrap();
    let root = TModel::from_hash(IndexMap::new());

    c.bench_function("macro_call_100", |b| {
        b.iter(|| {
            t.process(root.clone(), &mut std::io::sink()).unwrap();
        })
    });
}

// ---------------------------------------------------------------------------
// Benchmark 5: big_data_model
// Render a template that accesses a hash with 1000 keys.
// The template pulls out three keys: key_0, key_500, key_999.
// ---------------------------------------------------------------------------

fn bench_big_data_model(c: &mut Criterion) {
    let (cfg, _loader) = setup_config("bigdata", "${big.key_0}${big.key_500}${big.key_999}");
    let t = cfg.get_template("bigdata").unwrap();

    let mut big_hash = IndexMap::new();
    for i in 0..1000 {
        big_hash.insert(
            format!("key_{i}"),
            TModel::from_scalar(format!("value_{i}")),
        );
    }
    let mut root_map = IndexMap::new();
    root_map.insert("big".to_string(), TModel::from_hash(big_hash));
    let root = TModel::from_hash(root_map);

    c.bench_function("big_data_model", |b| {
        b.iter(|| {
            t.process(root.clone(), &mut std::io::sink()).unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_simple_hello_world,
    bench_simple_loop_100,
    bench_if_else_chain,
    bench_macro_call_100,
    bench_big_data_model,
);
criterion_main!(benches);
