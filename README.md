<a id="readme-top"></a>

<div align="center">

# freemarker-rust

**An embeddable Rust template engine, behaviorally ported from Apache FreeMarker 2.3.34.**

[![Crates.io](https://img.shields.io/crates/v/freemarker)](https://crates.io/crates/freemarker)
[![docs.rs](https://img.shields.io/docsrs/freemarker)](https://docs.rs/freemarker)
[![CI](https://github.com/easy-4-rust/freemarker-rust/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/easy-4-rust/freemarker-rust/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-orange)](#requirements)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

[English](README.md) | [简体中文](README.zh-CN.md)

[Overview](#overview) · [Why](#why-freemarker-rust) · [Architecture](#architecture) ·
[Capabilities](#capabilities) · [Quick start](#quick-start) · [Configuration](#configuration) ·
[Compatibility](#java-compatibility) · [Verification](#verification) · [References](#references)

</div>

---

> **Release:** `0.1.0-alpha.1`
> **Maturity:** alpha preview; APIs may change before `1.0`
> **Java baseline:** Apache FreeMarker `2.3-gae`, commit `7926e97`, `incompatible_improvements = 2.3.34`
> **Last verified:** 2026-08-03

freemarker-rust works inside a Rust process. It parses `.ftl` templates, evaluates expressions,
runs directives, builds outputs, and exposes the same data model and built-in surface as Apache
FreeMarker 2.3.34. The crate is published on [crates.io](https://crates.io/crates/freemarker),
the API is documented on [docs.rs](https://docs.rs/freemarker), and a Python binding
(`freemarker-pyo3`) is provided for the same audience that the Java `freemarker-jython25`
package served.

The repository has passed its repeatable local and CI readiness gates. That is evidence for the
library and its harness — not proof that an arbitrary host deployment is production-ready. Real
templates, data, capacity limits, monitoring, rollout, and rollback must still be accepted in
each host environment.

## Overview

freemarker-rust evaluates FreeMarker template language (`.ftl`) inside a Rust application. It
provides a lexer + recursive-descent parser, a stack-based renderer, a typed data model
matching the FreeMarker role interface family, all 183 Java 2.3.34 built-ins (`BuiltInsFor*`),
Java-compatible XML node handling (subset via `roxmltree`), an `ObjectWrapper` contract for
host-side value construction, integer / number / date / locale / encoding settings, template
caching, and structured errors with full instruction stack.

It is not a Java ABI or JVM replacement. Reflection-driven POJO wrapping, BeansWrapper
method overloading, and Jython-time transforms are intentionally out of scope — see
[Boundaries](#boundaries) and the security model in [`docs/superpowers/specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md).

## Why freemarker-rust?

- Parity-tested against Apache FreeMarker 2.3.34 at a pinned commit, with a `golden` suite that
  renders the official Java fixtures byte-for-byte (113/128 PASS, 0 FAIL, 0 BLOCKED).
- Cover 15,000+ Java test functions worth of behavior in 864 Rust tests: 183 built-ins, 128
  golden templates, 502 java_ported tests, plus unit tests, fuzz (`proptest` 10000 cases),
  and a security smoke suite.
- A typed Rust model (`TModel` slot struct) that mirrors Java's role interface family
  (`TemplateScalarModel`, `TemplateNumberModel`, `TemplateSequenceModel`, `TemplateHashModel`,
  `TemplateMethodModelEx`, `TemplateNodeModel`, `TemplateApiSupport`, …) without forcing
  the host to depend on JVM reflection.
- Zero `unsafe` in the workspace; `#[forbid(unsafe_code)]` enforced via workspace lints.
- Reuse existing FreeMarker templates, macros, configuration, and reviewer muscle memory.
- First-class Python binding (`freemarker-pyo3`) so a Python host can `pip install` and
  `import freemarker` to drive the same Rust engine.

### Good fit

- HTML / email / SVG / code-gen templates rendered by a Rust service.
- Embedding FreeMarker syntax into a Rust application that needs byte-identical output
  with the existing Java tooling.
- Python applications that need a FreeMarker engine without spinning up a JVM.
- Migration of Java FreeMarker logic to Rust with a pinned behavior baseline.

### Boundaries

- JVM reflection is **not** replicated. `BeansWrapper` / `ClassIntrospector` / method
  overloading / POJO wrapping are permanent `NOT_APPLICABLE` — 12 of the 15 parity skips
  (see [Compatibility](#java-compatibility)). Wrap host objects in `TModel` values yourself
  or via `SimpleObjectWrapper`.
- `?api` is supported, but the API view is supplied by the model owner (no JVM reflection).
  See `TemplateApiSupport` in the [Capabilities](#capabilities) table.
- `Configuration` is `Rc`-based and is not `Send`/`Sync`. Use one configuration per worker
  thread for long-lived caching. Cross-thread rendering of the same `Configuration` is not
  supported.
- Some third-party Java transforms (`JythonRuntime`) are unreachable; the corresponding
  `transforms` golden case is a permanent `NOT_APPLICABLE`.
- `0.1.0-alpha.1` is an alpha release; it does not carry a stable `1.0` compatibility promise.

## Architecture

```text
.ftl template text + host context (TModel) + Configuration
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│ freemarker                                                      │
│  lexer.rs → parser.rs  →  AST (Element tree + macros)            │
│                                       │                          │
│                                       ▼                          │
│  core::Environment  →  eval / exec  →  Instruction dispatch    │
│          │                                                         │
│          ├─ builtins/  183 built-ins (BuiltInsFor*)                │
│          ├─ core/      Settings, time_zone, incompatible           │
│          ├─ template/  TModel + ObjectWrapper + SimpleSequence     │
│          ├─ utility/   ObjectWrapper helpers, transforms           │
│          ├─ xml/       NodeModel + xpath_subset (roxmltree)        │
│          └─ cache/     TemplateCache + TemplateLoader                │
│                                       │                          │
│                                       ▼                          │
│                       Output (bytes) + structured error stack      │
└──────────────────────────────────────────────────────────────┘
```

The execution path is:

```text
Configuration::get_template(name)
  → TemplateLoader::fetch
  → cache::TemplateCache  (deduped by (name, locale, encoding))
  → parser::parse          (Template AST)
  → Template::process(root, out)
  → core::render
  → env exec / eval   (each Element → TModel)
  → builtins::*
  → out.write
```

| Crate | Published | Responsibility |
|:---|:---:|:---|
| [`freemarker`](freemarker/) | Yes | Lexer, parser, renderer, built-ins, data model, XML, cache, error model |
| [`freemarker-test`](freemarker-test/) | No | `golden` suite, `java_ported` suite, fuzz, security smoke, pyo3 smoke |
| [`freemarker-pyo3`](freemarker-pyo3/) | Yes (build) | Python bindings (`pip install`); see [Python bindings](#python-bindings) |

Full component boundaries, runtime flows, security model, and design decisions are in
[`docs/superpowers/specs/2026-08-01-architecture-design.md`](docs/superpowers/specs/2026-08-01-architecture-design.md) and
[`docs/superpowers/specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md).

## Capabilities

| Capability | Status | Evidence / limit |
|:---|:---:|:---|
| Freemarker template language (`.ftl`) parsing | Implemented | 128 official Java fixtures (`golden`) — 113/113 byte-exact matches |
| 183 built-ins (`?api`, `?has_api`, `?new`, `?lower_abc`, `?eval_json`, …) | Implemented | `compatibility` matrix in `docs/superpowers/specs/2026-08-02-builtins-design.md` |
| ICI (`incompatible_improvements`) versioning | Implemented | `?html <2.3.20` HTMLEnc, hash-literal duplicate keys `<2.3.21`, `?is_sequence` `<2.3.24` |
| `?new` class-resolution strategies | Implemented | `unrestricted` / `safer` / `allows_nothing` / opt-in `allowed_classes` + `trusted_templates` |
| XML node model subset | Implemented | `roxmltree`; visit macro with namespace prefix dispatch, `node[0]`, `./`, `true()`, index |
| Auto-imports / `<#include>` / `<#import>` | Implemented | `Configuration.addAutoImport` / `addAutoInclude` |
| Shared variables (`.globals`, `.data_model`) | Implemented | `Configuration.setSharedVariable` |
| `Configuration` cloning + cache reset | Implemented | New `TemplateCache` per clone (matches `Configuration.clone()`) |
| Locale, encoding, time-zone, output format | Implemented | `Settings.locale`, `url_escaping_charset`, `incompatible_improvements` |
| `?api` / `?has_api` on dynamic models | Implemented | `TemplateApiSupport` trait + `TModel.api` slot |
| POJO reflection / `BeansWrapper` | Permanent `NOT_APPLICABLE` | 12 cases locked in `golden.rs::permanent_na_reason` |
| BeansWrapper method overloading | Permanent `NOT_APPLICABLE` | 11 cases locked in `golden.rs::permanent_na_reason` |
| `JythonRuntime` transform | Permanent `NOT_APPLICABLE` | 1 case locked in `golden.rs::permanent_na_reason` |
| Multi-thread rendering with one `Configuration` | Not supported | `Configuration` is `Rc`-based — clone per worker thread |
| WASM target | Not yet claimed | XML parser and binary decoders are no_std-friendly, but the workspace is not yet configured |

## Quick start

### Requirements

- Rust `1.85` or newer
- Cargo with Rust Edition 2021 support
- Linux / macOS / Windows (CI matrix on all three)

Add the crate:

```bash
cargo add freemarker@0.1.0-alpha.1
```

### Minimal example

```rust
use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Build a configuration (typically shared across renders in a worker thread).
    let cfg = Rc::new(Configuration::default());

    // 2. Parse a template — the name is the cache key, the text is the FTL source.
    let tpl = parse(&cfg, "hello", "Hello ${name}!")?;

    // 3. Build the data model. Use `TModel::from_*` constructors for each role.
    let mut root = IndexMap::new();
    root.insert("name".to_string(), TModel::from_scalar("World".to_string()));

    // 4. Render to any `Write`.
    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;

    assert_eq!(out, b"Hello World!");
    Ok(())
}
```

Expected output:

```text
Hello World!
```

### Templating features in one snippet

```rust
use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Rc::new(Configuration::default());
    let tpl = parse(
        &cfg,
        "catalog",
        r#"
<#assign items = ["Rust", "FreeMarker", "FTL"]>
# ${items?size} items

<#list items as item>
<#if item != "FreeMarker">- ${item?upper_case}
</#if></#list>
"#,
    )?;

    let mut root = IndexMap::new();
    root.insert("user".to_string(), TModel::from_scalar("alice".to_string()));
    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;
    print!("{}", String::from_utf8(out)?);
    Ok(())
}
```

Expected output:

```text
# 3 items

- RUST
- FTL
```

### From Git or local workspace

```toml
[dependencies]
freemarker = { git = "https://github.com/easy-4-rust/freemarker-rust", rev = "dev" }
```

Local path is acceptable for development only; do not publish it.

## Configuration

`Configuration` is the Rust equivalent of Java `freemarker.template.Configuration`. The defaults
pin to Java 2.3.34 (`incompatible_improvements = 2.3.34`, locale `en_US`, lenient template name
format). Mutable fields are exercised through `set_*` methods (`apply_settings` parses the
camelCase strings used in Java `.properties` files).

| Setting | Default | Java key | Notes |
|---|---|---|---|
| `settings.incompatible_improvements` | `2.3.34` | `incompatibleImprovements` | Bump per ICI lifecycle |
| `settings.locale` | `en_US` | `locale` | Locale used for date / number formatting |
| `settings.time_zone` | GMT | `time_zone` | Accepts offset (`"GMT+01:00"`) or IANA name |
| `settings.number_format` | `"number"` | `number_format` | Pattern or registered name |
| `settings.date_format` / `datetime_format` / `time_format` | derived | `date_format` / `datetime_format` / `time_format` | FreeMarker format strings |
| `settings.url_escaping_charset` | UTF-8 | `url_escaping_charset` | Used by `?url` |
| `settings.output_encoding` | UTF-8 | `output_encoding` | `UTF-16` and `ISO-8859-1` are supported |
| `settings.boolean_format` | `"true,false"` | `boolean_format` | Comma-separated `true, false` tokens |
| `settings.new_builtin_class_resolver` | `Unrestricted` | `new_builtin_class_resolver` | `unrestricted` / `safer` / `allows_nothing` / `opt_in` |
| `settings.locale` | `en_US` | `locale` | FreeMarker locale logic |
| `template_loader` | `StringLoader` | — | Inject `Arc<dyn TemplateLoader>` for files/network |
| `auto_imports` | `[]` | `auto_import` | `Vec<(namespace, path)>` |
| `shared_vars` | `compress`, `html_escape`, `normalize_newlines`, `capture_output`, `xml_escape` | `shared_variable` | `Configuration.set_shared_variable` |

Settings are applied through `apply_settings` (Java `Configuration.setSettings` parity):

```rust
use freemarker::template::Configuration;

let mut cfg = Configuration::default();
apply_settings(&mut cfg, &[
    ("locale".to_string(), "en_US".to_string()),
    ("incompatible_improvements".to_string(), "2.3.34".to_string()),
    ("new_builtin_class_resolver".to_string(), "unrestricted".to_string()),
]);
```

### Building host data models

The `TModel` slot struct exposes one `Option` per role (`scalar`, `number`, `boolean`,
`date`, `sequence`, `collection`, `hash`, `method`, `directive`, `transform`, `node`, `node_hash`,
`api`). Constructors are the Java-compatible way to assign roles:

| Constructor | Role | Wraps |
|---|---|---|
| `TModel::from_scalar(s)` | `TemplateScalarModel` | `String` |
| `TModel::from_number(n)` | `TemplateNumberModel` | `TNumber` (`Int`/`Long`/`BigInt`/`Float`/`Double`/`Decimal`) |
| `TModel::from_boolean(b)` | `TemplateBooleanModel` | `bool` |
| `TModel::from_date(d)` | `TemplateDateModel` | `DateValue` |
| `TModel::from_sequence(v)` | `TemplateSequenceModel` | `Vec<TModel>` |
| `TModel::from_collection(v)` | `TemplateCollectionModel` | `Vec<TModel>` |
| `TModel::from_hash(v)` | `TemplateHashModel` + `TemplateHashModelEx` | `IndexMap<String, TModel>` |
| `TModel::from_method(m)` | `TemplateMethodModelEx` | any object implementing `exec(Vec<TModel>) -> Result<TModel>` |
| `TModel::from_directive(d)` | `TemplateDirectiveModel` | any `impl TemplateDirectiveModel` |
| `TModel::from_transform(t)` | `TemplateTransformModel` | any `impl TemplateTransformModel` |
| `TModel::from_node_model(...)` | `TemplateNodeModel` | XML node adapters |

Or wrap a Rust value with `SimpleObjectWrapper` for the most common case (`String`, `i64`,
`bool`, `DateValue`, `HashMap<String, DynValue>`/`Vec<(String, DynValue)>`, `Vec<DynValue>`).

### Registering a custom method (host function)

```rust
use freemarker::template::{Configuration, TModel, TemplateMethodModelEx};
use freemarker::parser::parse;
use std::rc::Rc;

struct Greet;

impl TemplateMethodModelEx for Greet {
    fn exec(&self, args: Vec<TModel>) -> freemarker::Result<TModel> {
        let who = args.first().and_then(|m| m.get_scalar().ok()).unwrap_or_default();
        Ok(TModel::from_scalar(format!("hi, {who}")))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Rc::new(Configuration::default());
    let tpl = parse(&cfg, "greet", "${greetMethod('World')}")?;

    let mut root = freemarker::value::IndexMap::new();
    root.insert("greetMethod".to_string(), TModel::from_method(Greet));
    let mut out = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;
    assert_eq!(out, b"hi, World");
    Ok(())
}
```

### Dynamic data via `DynValue`

For rows that come from request bodies or external services, use `DynValue` as a flat
representation and convert once before rendering:

```rust
use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel, DynValue, ObjectWrapper, SimpleObjectWrapper};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Rc::new(Configuration::default());
    let tpl = parse(&cfg, "user", "Hello ${user.name}!")?;

    let payload = DynValue::Map(vec![(
        "user".to_string(),
        DynValue::Map(vec![("name".to_string(), DynValue::Str("Bob".to_string()))]),
    )]);
    let root = SimpleObjectWrapper
        .wrap(&payload)?
        .unwrap_or_else(TModel::nothing);

    let mut out = Vec::new();
    tpl.process(root, &mut out)?;
    assert_eq!(out, b"Hello Bob!");
    Ok(())
}
```

## Java compatibility

The behavioral authority is Apache FreeMarker `2.3-gae@7926e97` at `incompatibleImprovements
= 2.3.34`. Compatibility is verified through the official freemarker-jython25 parity suite
(`freemarker-test/tests/suite/cases/…`), Java-ported test classes, and a pinned replay of
the bundle's `expected/*.txt` files.

| Java design | Rust design | Compatibility intent |
|:---|:---|:---|
| `Configuration` | `Configuration` | Settings, caching, locale, auto-imports, shared variables |
| `Template` / `Template.process` | `Template` / `Template::process` | Identical signature and rendering semantics |
| FML parser (`fmpp` + `FMParser`) | Recursive-descent parser (`lexer.rs` + `parser/grammar.rs`) | Parity of AST, not of parser implementation |
| `TemplateModel` role interface family | `TModel` slot struct + `Option` per role | Identical role semantics, no JVM reflection |
| `BuiltInsFor*` (183 static classes) | `builtins/mod.rs` registry | Name → handler lookup with arity/argument checking |
| `simplemap` / `SimpleHash` / `SimpleSequence` | `Template::from_hash` / `from_sequence` | Same semantics, including Ex hash (`entrySet()`) |
| `ObjectWrapper` / `SimpleObjectWrapper` / `DefaultObjectWrapper` | `ObjectWrapper` trait + `SimpleObjectWrapper` | API surface aligned; reflection-equipped `DefaultObjectWrapper` is out of scope |
| `BeansWrapper` / `ClassIntrospector` | **Not implemented** | Permanent `NOT_APPLICABLE` — wrap host objects in `TModel` |
| `TemplateClassResolver` × 4 strategies | `NewBuiltinClassResolver` (`template_class_resolver.rs`) | `unrestricted` / `safer` / `allows_nothing` / opt-in |
| `TemplateModelWithAPISupport` (`?api`) | `TemplateApiSupport` trait + `TModel.api` | Engine has no reflection; model owners provide API views |
| `NodeModel` + Jaxen XPath | `xml/mod.rs` (subset) + `roxmltree` | Visit-macro namespace dispatch, `node[0]`, `./`, `true()`, index |
| `Configuration` cloning | `Configuration::clone` | New empty cache per clone (matches `Configuration.clone()`) |
| `Multimap` / `ArrayList` overload picking | `TemplateMethodModelEx::exec` | Single dispatch, no overload resolution |

Detailed matrices:

- [`docs/superpowers/specs/2026-08-03-migration-parity-ledger-design.md`](docs/superpowers/specs/2026-08-03-migration-parity-ledger-design.md) — 128 fixture dispositions
- [`docs/superpowers/specs/2026-08-02-builtins-design.md`](docs/superpowers/specs/2026-08-02-builtins-design.md) — 183 built-in parity
- [`docs/superpowers/specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md) — restricted subset, 15 permanent NA

## Verification

The current repository gates include:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace --all-features
cargo test --workspace
cargo test --workspace --no-default-features
cargo doc --workspace --all-features --no-deps
```

CI also runs `cargo deny check`, `cargo audit` (0 vulnerabilities), `cargo public-api`
baseline diff (0 drift), proptest fuzz (10,000 cases), and a multi-OS matrix across
Ubuntu / macOS / Windows on both `stable` and `rust-version = 1.85`.

The 2026-08-03 audit:

- **golden**: 113/128 PASS (88%), 0 FAIL, 0 BLOCKED, 15 permanent `NOT_APPLICABLE`
- **java_ported**: 502/502 PASS, 7 ignored (engine gaps documented)
- **fuzz**: 10,000 proptest cases, 0 panic
- **CI**: 12/12 jobs success (governance, MSRV, 3 OS, pyo3 × 6)
- **public-api**: 0 drift against `docs/release/api-baseline.txt` (3,804 items)

Detailed parity metrics and the production audit checklist are in
[`docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md`](docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md). The compatibility report
(generated by `scripts/gen_compat_report.py`) summarizes the same numbers.

## Python bindings

`freemarker-pyo3` exposes the Rust engine to Python through `pyo3` + `maturin`. Build with
maturin and install the wheel; the result is a Python extension module named `freemarker_pyo3`.

```bash
# Local development build
cd freemarker-pyo3
maturin build --release --sdist --out ../dist
pip install ../dist/freemarker_pyo3-*.whl

# Publish to PyPI (manual)
git tag pyo3-v0.1.0-alpha.1
git push origin pyo3-v0.1.0-alpha.1
# .github/workflows/pyo3-publish.yml runs Trusted Publishing
```

The Python surface is the source of truth in `freemarker-pyo3/src/lib.rs` and includes
`FmConfiguration`, `FmTemplate`, `Process`, `DataModel`, and the same `?has_api` / `?api`
extension point as the Rust engine. Java users will recognize the shape as the successor to
`freemarker-jython25`.

## References

| Document | English | 简体中文 |
|:---|:---|:---|
| Architecture | [`specs/2026-08-01-architecture-design.md`](docs/superpowers/specs/2026-08-01-architecture-design.md) | 同左 |
| Parser | [`specs/2026-08-01-parser-design.md`](docs/superpowers/specs/2026-08-01-parser-design.md) | 同左 |
| Renderer | [`specs/2026-08-01-rendering-engine-design.md`](docs/superpowers/specs/2026-08-01-rendering-engine-design.md) | 同左 |
| Built-ins | [`specs/2026-08-02-builtins-design.md`](docs/superpowers/specs/2026-08-02-builtins-design.md) | 同左 |
| Data model | [`specs/2026-08-01-data-model-design.md`](docs/superpowers/specs/2026-08-01-data-model-design.md) | 同左 |
| Configuration & cache | [`specs/2026-08-01-config-cache-design.md`](docs/superpowers/specs/2026-08-01-config-cache-design.md) | 同左 |
| Format & escape | [`specs/2026-08-01-formatting-design.md`](docs/superpowers/specs/2026-08-01-formatting-design.md) | 同左 |
| Errors | [`specs/2026-08-01-error-handling-design.md`](docs/superpowers/specs/2026-08-01-error-handling-design.md) | 同左 |
| pyo3 design | [`specs/2026-08-01-pyo3-design.md`](docs/superpowers/specs/2026-08-01-pyo3-design.md) | 同左 |
| Testing | [`specs/2026-08-01-testing-strategy-design.md`](docs/superpowers/specs/2026-08-01-testing-strategy-design.md) | 同左 |
| Roadmap | [`specs/2026-08-01-migration-roadmap-design.md`](docs/superpowers/specs/2026-08-01-migration-roadmap-design.md) | 同左 |
| Versioning | [`specs/2026-08-03-versioning-design.md`](docs/superpowers/specs/2026-08-03-versioning-design.md) | 同左 |
| Publishing | [`specs/2026-08-03-publishing-design.md`](docs/superpowers/specs/2026-08-03-publishing-design.md) | 同左 |
| Security | [`specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md) | 同左 |
| Benchmarks | [`docs/release/benchmarks.md`](docs/release/benchmarks.md) | 同左 |
| Migration ledger | [`specs/2026-08-03-migration-parity-ledger-design.md`](docs/superpowers/specs/2026-08-03-migration-parity-ledger-design.md) | 同左 |
| Acceptance report | [`specs/2026-08-03-acceptance-report-design.md`](docs/superpowers/specs/2026-08-03-acceptance-report-design.md) | 同左 |
| Production audit | [`specs/2026-08-03-production-readiness-audit-design.md`](docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md) | 同左 |
| API reference | [docs.rs](https://docs.rs/freemarker) | Source rustdoc includes bilingual notes |

## Development and release

Development happens on `dev`; `main` is the release branch. A `v*` tag contained in `main`
triggers `.github/workflows/release.yml`: `cargo publish --dry-run` plus a GitHub Release
with the matching CHANGELOG section.

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo publish -p freemarker --dry-run
cargo publishes of freemarker-pyo3 are coordinated through the pyo3-v* tag and
.github/workflows/pyo3-publish.yml.
```

Do not publish to PyPI before the matching exact version of `freemarker` is on crates.io.

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Apache FreeMarker is an Apache
Software Foundation project; this Rust port is maintained independently by the
`easy-4-rust` organization.

---

<div align="center">

[Back to top](#readme-top) · [crates.io](https://crates.io/crates/freemarker) ·
[docs.rs](https://docs.rs/freemarker) ·
[Issues](https://github.com/easy-4-rust/freemarker-rust/issues)

</div>
