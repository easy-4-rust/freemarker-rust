<a id="readme-top"></a>

<div align="center">

# freemarker-rust

**面向 Rust 应用的嵌入式模板引擎，按行为语义迁移自 Apache FreeMarker 2.3.34。**

[![Crates.io](https://img.shields.io/crates/v/freemarker)](https://crates.io/crates/freemarker)
[![docs.rs](https://img.shields.io/docsrs/freemarker)](https://docs.rs/freemarker)
[![CI](https://github.com/easy-4-rust/freemarker-rust/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/easy-4-rust/freemarker-rust/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-orange)](#环境要求)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

[English](README.md) | [简体中文](README.zh-CN.md)

[项目定位](#项目定位) · [为什么选择](#为什么选择-freemarker-rust) · [架构](#架构) ·
[能力矩阵](#能力矩阵) · [快速开始](#快速开始) · [配置](#配置) · [Java 兼容](#java-兼容) ·
[验证](#验证) · [文档导航](#文档导航)

</div>

---

> **当前版本**：`0.1.0-alpha.1`
> **成熟度**：Alpha 预览版；`1.0` 之前公共 API 仍可能调整
> **Java 基线**：Apache FreeMarker `2.3-gae` 分支 `7926e97`，`incompatibleImprovements = 2.3.34`
> **最后核验**：2026-08-03

freemarker-rust 在 Rust 进程内解析并渲染 `.ftl` 模板。它提供词法/解析器 + 模板渲染器、
与 Java 一致的数据模型和内置函数、Java 兼容的 XML 节点处理（基于 `roxmltree` 的子集）、
宿主数据模型包装协议、模板缓存以及带完整指令栈的错误模型。核心 crate 已发布到
[crates.io](https://crates.io/crates/freemarker)，用户在 [docs.rs](https://docs.rs/freemarker)
可查看 API 文档；Python 端通过 `freemarker-pyo3` 绑定使用（对应 Java 时代的
`freemarker-jython25`）。

仓库级本地验收和 CI 门禁已经通过。这些证据证明库和验收工具链的状态，但不等于
任意业务宿主已经达到生产可用。真实模板、真实数据、容量、监控、灰度、回滚仍需
在每个宿主环境中重新验收。

## 项目定位

freemarker-rust 在 Rust 应用内执行 FreeMarker 模板语言（`.ftl`）。它提供词法 + 递归
下降解析器、栈式渲染器、按 Java 角色接口家族对齐的强类型数据模型、Java 2.3.34 全集
183 个内置函数（`BuiltInsFor*`）、Java 兼容的 XML 节点处理（`roxmltree` 子集）、
宿主值构造的 `ObjectWrapper` 契约、数字/日期/区域/编码设置、模板缓存以及带源码位置
和指令栈的结构化错误。

本项目不是 Java ABI 或 JVM 的替代品。反射式 POJO 包装、BeansWrapper 方法重载、
Jython 阶段的自定义变换（`JythonRuntime`）都已明确判定为不做实现——见
[明确边界](#明确边界) 和 [`docs/superpowers/specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md) 中的
受限子集。

## 为什么选择 freemarker-rust？

- 与 Apache FreeMarker 2.3.34 的固定版本 commit 逐字节对齐：通过 `golden` 套件渲染
  官方 Java fixtures，113/128 PASS、0 FAIL、0 BLOCKED（`golden.rs::permanent_na_reason`
  固定的 15 项永久 NA 详见 [Java 兼容](#java-兼容)）。
- 用 864 个 Rust 测试覆盖 15,000+ 的 Java 测试语义：183 内建、128 黄金模板、502
  java_ported 测试，加上单元测试、`proptest` 10000 用例模糊测试与安全 smoke 套件。
- 提供与 Java 角色接口家族对齐的 `TModel` 槽位结构（`TemplateScalarModel`、
  `TemplateNumberModel`、`TemplateSequenceModel`、`TemplateHashModel`、
  `TemplateMethodModelEx`、`TemplateNodeModel`、`TemplateApiSupport` …），不强制宿主
  依赖 JVM 反射。
- 工程层零 `unsafe`，通过 workspace lints 强制 `#[forbid(unsafe_code)]`。
- 复用现有 FreeMarker 模板、宏、配置和评审心智模型。
- 提供第一类 Python 绑定 `freemarker-pyo3`：Python 宿主 `pip install` 后 `import freemarker`
  即可驱动同一 Rust 引擎，无需启动 JVM。

### 适用场景

- Rust 服务渲染 HTML / 邮件 / SVG / 代码生成模板。
- 需要与现有 Java 工具链产出**逐字节一致**的 Rust 应用嵌入 FreeMarker 模板。
- Python 应用需要 FreeMarker 引擎而不想依赖 JVM。
- 把 Java FreeMarker 逻辑迁移到 Rust 时需要一个固定的行为基线。

### 明确边界

- **不实现 JVM 反射**。`BeansWrapper` / `ClassIntrospector` / 方法重载 / POJO 包装
  被永久划为 `NOT_APPLICABLE`（15 项永久 NA 中占 12 项）。需要将宿主对象自行包装
  为 `TModel`，或通过 `SimpleObjectWrapper` 注入。
- `?api` 已实现，但 API 视图由**模型所有者**提供（引擎无反射）。详见
  [`TemplateApiSupport`](https://docs.rs/freemarker) 与 [能力矩阵](#能力矩阵)。
- `Configuration` 基于 `Rc` 持有内部状态，**不是 `Send`/`Sync`**。长生命周期缓存
  应在每个工作线程克隆一份独立的 `Configuration`；同一份跨线程渲染**不支持**。
- 部分 Java 端变换（如 `JythonRuntime`）无法映射，对应 `transforms` 黄金用例为
  永久 `NOT_APPLICABLE`。
- `0.1.0-alpha.1` 是 Alpha 版本，不代表稳定的 `1.0` 兼容承诺。

## 架构

```text
.ftl 模板文本 + 宿主上下文（TModel） + Configuration
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│ freemarker                                                      │
│  lexer.rs → parser.rs  →  AST（Element 树 + 宏表）               │
│                                       │                          │
│                                       ▼                          │
│  core::Environment  →  eval / exec  →  指令分发              │
│          │                                                         │
│          ├─ builtins/   183 个内置函数（BuiltInsFor*）            │
│          ├─ core/       Settings / 时区 / ICI                    │
│          ├─ template/   TModel + ObjectWrapper + SimpleSequence  │
│          ├─ utility/    ObjectWrapper 辅助 / 变换                │
│          ├─ xml/        NodeModel + xpath_subset（roxmltree）    │
│          └─ cache/      TemplateCache + TemplateLoader             │
│                                       │                          │
│                                       ▼                          │
│                       输出（字节流）+ 结构化错误                   │
└──────────────────────────────────────────────────────────────┘
```

核心执行链：

```text
Configuration::get_template(name)
  → TemplateLoader::fetch
  → cache::TemplateCache  （按 (name, locale, encoding) 去重）
  → parser::parse          （Template AST）
  → Template::process(root, out)
  → core::render
  → env exec / eval   （每个 Element → TModel）
  → builtins::*
  → out.write
```

| Crate | 是否发布 | 职责 |
|:---|:---:|:---|
| [`freemarker`](freemarker/) | 是 | 词法、解析器、渲染器、内建、数据模型、XML、缓存、错误模型 |
| [`freemarker-test`](freemarker-test/) | 否 | `golden` 套件、`java_ported` 套件、fuzz、security smoke、pyo3 smoke |
| [`freemarker-pyo3`](freemarker-pyo3/) | 是（构建） | Python 绑定（`pip install`）；详见 [Python 绑定](#python-绑定) |

组件边界、运行流程、安全模型和架构决策详见
[`docs/superpowers/specs/2026-08-01-architecture-design.md`](docs/superpowers/specs/2026-08-01-architecture-design.md) 和
[`docs/superpowers/specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md)。

## 能力矩阵

| 能力 | 状态 | 证据或限制 |
|:---|:---:|:---|
| FreeMarker 模板语言（`.ftl`）解析 | 已实现 | 128 个官方 Java fixtures（`golden`）——113/113 逐字节一致 |
| 183 个内置函数（`?api`、`?has_api`、`?new`、`?lower_abc`、`?eval_json` …） | 已实现 | `docs/superpowers/specs/2026-08-02-builtins-design.md` 中的兼容矩阵 |
| ICI（`incompatible_improvements`）版本化 | 已实现 | `?html <2.3.20` 用 HTMLEnc、`<2.3.21` 哈希字面量保留重复键、`<2.3.24` `?is_sequence` |
| `?new` 类解析策略 | 已实现 | `unrestricted` / `safer` / `allows_nothing` / opt-in `allowed_classes` + `trusted_templates` |
| XML 节点模型子集 | 已实现 | `roxmltree`；visit 命名空间前缀宏分派、`node[0]`、`./`、`true()`、索引 |
| Auto-import / `<#include>` / `<#import>` | 已实现 | `Configuration.addAutoImport` / `addAutoInclude` |
| 共享变量（`.globals`、`.data_model`） | 已实现 | `Configuration.set_shared_variable` |
| `Configuration` 克隆 + 缓存重置 | 已实现 | 每次 clone 重建 `TemplateCache`（对应 `Configuration.clone()`） |
| Locale / 编码 / 时区 / 输出格式 | 已实现 | `Settings.locale`、`url_escaping_charset`、`incompatible_improvements` |
| 动态模型的 `?api` / `?has_api` | 已实现 | `TemplateApiSupport` trait + `TModel.api` 槽位 |
| POJO 反射 / `BeansWrapper` | 永久 `NOT_APPLICABLE` | 12 项锁定在 `golden.rs::permanent_na_reason` |
| BeansWrapper 方法重载 | 永久 `NOT_APPLICABLE` | 11 项锁定在 `golden.rs::permanent_na_reason` |
| `JythonRuntime` 变换 | 永久 `NOT_APPLICABLE` | 1 项锁定在 `golden.rs::permanent_na_reason` |
| 同一 `Configuration` 跨线程渲染 | 不支持 | `Configuration` 基于 `Rc`——每个 worker 线程 clone 一份 |
| WASM target | 尚未声明 | 解析器与二进制解码器对 `no_std` 友好，但 workspace 还未配置 |

## 快速开始

### 环境要求

- Rust `1.85` 或更高版本
- 支持 Rust Edition 2021 的 Cargo
- Linux / macOS / Windows（CI 在三种平台运行矩阵）

添加依赖：

```bash
cargo add freemarker@0.1.0-alpha.1
```

### 最小示例

```rust
use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 构造配置（通常在同一 worker 线程内跨渲染复用）
    let cfg = Rc::new(Configuration::default());

    // 2. 解析模板——name 是缓存键，text 是 FTL 源码
    let tpl = parse(&cfg, "hello", "Hello ${name}!")?;

    // 3. 构造数据模型；每个角色对应一个 TModel::from_* 构造器
    let mut root = IndexMap::new();
    root.insert("name".to_string(), TModel::from_scalar("World".to_string()));

    // 4. 渲染到任意 Write
    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;

    assert_eq!(out, b"Hello World!");
    Ok(())
}
```

预期输出：

```text
Hello World!
```

### 模板特性一瞥

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

预期输出：

```text
# 3 items

- RUST
- FTL
```

### 从 Git 或 workspace 本地使用

```toml
[dependencies]
freemarker = { git = "https://github.com/easy-4-rust/freemarker-rust", rev = "dev" }
```

本地 `path` 仅限开发使用，不要发布。

## 配置

`Configuration` 对应 Java 端的 `freemarker.template.Configuration`。默认固定 Java
2.3.34（`incompatible_improvements = 2.3.34`、`locale = en_US`、宽松版模板名格式）。
可变字段通过 `set_*` 方法修改（`apply_settings` 解析 Java `.properties` 的
camelCase 字符串）。

| 字段 | 默认值 | Java key | 说明 |
|---|---|---|---|
| `settings.incompatible_improvements` | `2.3.34` | `incompatibleImprovements` | 按 ICI 生命周期递增 |
| `settings.locale` | `en_US` | `locale` | 用于日期 / 数字格式化 |
| `settings.time_zone` | GMT | `time_zone` | 接受偏移（`"GMT+01:00"`）或 IANA 名称 |
| `settings.number_format` | `"number"` | `number_format` | 模式或已注册名称 |
| `settings.date_format` / `datetime_format` / `time_format` | 派生 | `date_format` / `datetime_format` / `time_format` | FreeMarker 格式串 |
| `settings.url_escaping_charset` | UTF-8 | `url_escaping_charset` | `?url` 使用 |
| `settings.output_encoding` | UTF-8 | `output_encoding` | 支持 `UTF-16`、`ISO-8859-1` |
| `settings.boolean_format` | `"true,false"` | `boolean_format` | 逗号分隔的 `true, false` |
| `settings.new_builtin_class_resolver` | `Unrestricted` | `new_builtin_class_resolver` | `unrestricted` / `safer` / `allows_nothing` / opt-in |
| `template_loader` | `StringLoader` | — | 注入 `Arc<dyn TemplateLoader>` 接文件/网络 |
| `auto_imports` | `[]` | `auto_import` | `Vec<(namespace, path)>` |
| `shared_vars` | `compress`、`html_escape`、… | `shared_variable` | `Configuration.set_shared_variable` |

设置通过 `apply_settings` 应用（对应 Java `Configuration.setSettings`）：

```rust
use freemarker::template::Configuration;

let mut cfg = Configuration::default();
apply_settings(&mut cfg, &[
    ("locale".to_string(), "en_US".to_string()),
    ("incompatible_improvements".to_string(), "2.3.34".to_string()),
    ("new_builtin_class_resolver".to_string(), "unrestricted".to_string()),
]);
```

### 构造宿主数据模型

`TModel` 槽位结构为每个角色暴露一个 `Option`（`scalar`、`number`、`boolean`、`date`、
`sequence`、`collection`、`hash`、`method`、`directive`、`transform`、`node`、`node_hash`、
`api`）。构造器是 Java 兼容的角色赋值方式：

| 构造器 | 角色 | 包装类型 |
|---|---|---|
| `TModel::from_scalar(s)` | `TemplateScalarModel` | `String` |
| `TModel::from_number(n)` | `TemplateNumberModel` | `TNumber`（`Int`/`Long`/`BigInt`/`Float`/`Double`/`Decimal`） |
| `TModel::from_boolean(b)` | `TemplateBooleanModel` | `bool` |
| `TModel::from_date(d)` | `TemplateDateModel` | `DateValue` |
| `TModel::from_sequence(v)` | `TemplateSequenceModel` | `Vec<TModel>` |
| `TModel::from_collection(v)` | `TemplateCollectionModel` | `Vec<TModel>` |
| `TModel::from_hash(v)` | `TemplateHashModel` + `TemplateHashModelEx` | `IndexMap<String, TModel>` |
| `TModel::from_method(m)` | `TemplateMethodModelEx` | 任何实现 `exec(Vec<TModel>) -> Result<TModel>` 的对象 |
| `TModel::from_directive(d)` | `TemplateDirectiveModel` | 任何 `impl TemplateDirectiveModel` |
| `TModel::from_transform(t)` | `TemplateTransformModel` | 任何 `impl TemplateTransformModel` |
| `TModel::from_node_model(...)` | `TemplateNodeModel` | XML 节点适配器 |

常规数据（字符串、数字、布尔、日期、HashMap、Vec）可直接用 `SimpleObjectWrapper::wrap`
包装 `DynValue`。

### 注册自定义方法（宿主函数）

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

### 通过 `DynValue` 接入动态数据

从请求体或外部服务流入的行数据，可用 `DynValue` 作为扁平表示，渲染前一次性转换：

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

## Java 兼容

行为权威为 Apache FreeMarker `2.3-gae@7926e97`（`incompatible_improvements = 2.3.34`）。
经由官方 freemarker-jython25 parity 套件（`freemarker-test/tests/suite/cases/…`）、Java
端口测试类、以及 `expected/*.txt` 文件的固定回归验证：

| Java 设计 | Rust 设计 | 兼容目标 |
|:---|:---|:---|
| `Configuration` | `Configuration` | 设置、缓存、locale、auto-import、共享变量 |
| `Template` / `Template.process` | `Template` / `Template::process` | 同名同语义渲染入口 |
| FML 解析器（`fmpp` + `FMParser`） | 递归下降解析器（`lexer.rs` + `parser/grammar.rs`） | AST 行为一致，解析器实现不要求相同 |
| `TemplateModel` 角色接口家族 | `TModel` 槽位结构 + `Option` 角色 | 角色语义一致，无 JVM 反射 |
| `BuiltInsFor*`（183 个静态类） | `builtins/mod.rs` 注册表 | 名称 → handler 查询 + 参数/arity 校验 |
| `simplemap` / `SimpleHash` / `SimpleSequence` | `Template::from_hash` / `from_sequence` | 语义一致，包括 Ex hash（`entrySet()`） |
| `ObjectWrapper` / `SimpleObjectWrapper` / `DefaultObjectWrapper` | `ObjectWrapper` trait + `SimpleObjectWrapper` | API 表面一致；反射式 `DefaultObjectWrapper` 未实现 |
| `BeansWrapper` / `ClassIntrospector` | **不实现** | 永久 `NOT_APPLICABLE`——宿主对象自行包装为 `TModel` |
| `TemplateClassResolver` × 4 策略 | `NewBuiltinClassResolver`（`template_class_resolver.rs`） | `unrestricted` / `safer` / `allows_nothing` / opt-in |
| `TemplateModelWithAPISupport`（`?api`） | `TemplateApiSupport` trait + `TModel.api` | 引擎无反射；模型所有者提供 API 视图 |
| `NodeModel` + Jaxen XPath | `xml/mod.rs`（子集） + `roxmltree` | visit 命名空间前缀宏分派、`node[0]`、`./`、`true()`、索引 |
| `Configuration` 克隆 | `Configuration::clone` | 每次 clone 重建空缓存（对应 `Configuration.clone()`） |
| `Multimap` / `ArrayList` 重载分派 | `TemplateMethodModelEx::exec` | 单一分派，无重载解析 |

详细对照表：

- [`docs/superpowers/specs/2026-08-03-migration-parity-ledger-design.md`](docs/superpowers/specs/2026-08-03-migration-parity-ledger-design.md) —— 128 fixture 一一 disposition
- [`docs/superpowers/specs/2026-08-02-builtins-design.md`](docs/superpowers/specs/2026-08-02-builtins-design.md) —— 183 内建兼容矩阵
- [`docs/superpowers/specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md) —— 受限子集 + 15 项永久 NA 决策记录

## 验证

仓库当前基础门禁：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace --all-features
cargo test --workspace
cargo test --workspace --no-default-features
cargo doc --workspace --all-features --no-deps
```

CI 还会执行 `cargo deny check`、`cargo audit`（0 漏洞）、`cargo public-api` 基线
diff 校验（0 漂移）、`proptest` 模糊测试（10000 用例）、以及 Ubuntu / macOS / Windows
跨 stable 与 `rust-version = 1.85` 的多 OS 矩阵。

2026-08-03 审计结果：

- **golden**：113/128 PASS（88%）、0 FAIL、0 BLOCKED、15 项永久 `NOT_APPLICABLE`
- **java_ported**：502/502 PASS、7 ignored（引擎缺口已记录）
- **fuzz**：10000 proptest 用例，0 panic
- **CI**：12/12 jobs success（治理、MSRV、3 OS、pyo3 × 6）
- **public-api**：相对 `docs/release/api-baseline.txt`（3,804 项） 0 漂移

详细的 parity 指标与生产就绪核查清单见
[`docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md`](docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md)。同口径的兼容报告由
`scripts/gen_compat_report.py` 生成。

## Python 绑定

`freemarker-pyo3` 通过 `pyo3` + `maturin` 将 Rust 引擎暴露给 Python。构建产物是
Python 扩展模块 `freemarker_pyo3`：

```bash
# 本地开发构建
cd freemarker-pyo3
maturin build --release --sdist --out ../dist
pip install ../dist/freemarker_pyo3-*.whl

# 发布到 PyPI（手动）
git tag pyo3-v0.1.0-alpha.1
git push origin pyo3-v0.1.0-alpha.1
# .github/workflows/pyo3-publish.yml 触发 Trusted Publishing
```

Python 面 API 以 `freemarker-pyo3/src/lib.rs` 为准，包含 `FmConfiguration`、`FmTemplate`、
`Process`、`DataModel` 与同样的 `?has_api` / `?api` 扩展点。Java 用户会注意到：
本项目本质是 Java `freemarker-jython25` 的 Rust 后续形态。

## 文档导航

| 文档 | English | 简体中文 |
|:---|:---|:---|
| 架构 | [`specs/2026-08-01-architecture-design.md`](docs/superpowers/specs/2026-08-01-architecture-design.md) | 同左 |
| 解析器 | [`specs/2026-08-01-parser-design.md`](docs/superpowers/specs/2026-08-01-parser-design.md) | 同左 |
| 渲染引擎 | [`specs/2026-08-01-rendering-engine-design.md`](docs/superpowers/specs/2026-08-01-rendering-engine-design.md) | 同左 |
| 内建函数 | [`specs/2026-08-02-builtins-design.md`](docs/superpowers/specs/2026-08-02-builtins-design.md) | 同左 |
| 数据模型 | [`specs/2026-08-01-data-model-design.md`](docs/superpowers/specs/2026-08-01-data-model-design.md) | 同左 |
| 配置缓存 | [`specs/2026-08-01-config-cache-design.md`](docs/superpowers/specs/2026-08-01-config-cache-design.md) | 同左 |
| 格式化与转义 | [`specs/2026-08-01-formatting-design.md`](docs/superpowers/specs/2026-08-01-formatting-design.md) | 同左 |
| 错误处理 | [`specs/2026-08-01-error-handling-design.md`](docs/superpowers/specs/2026-08-01-error-handling-design.md) | 同左 |
| pyo3 设计 | [`specs/2026-08-01-pyo3-design.md`](docs/superpowers/specs/2026-08-01-pyo3-design.md) | 同左 |
| 测试与验证 | [`specs/2026-08-01-testing-strategy-design.md`](docs/superpowers/specs/2026-08-01-testing-strategy-design.md) | 同左 |
| 迁移路线 | [`specs/2026-08-01-migration-roadmap-design.md`](docs/superpowers/specs/2026-08-01-migration-roadmap-design.md) | 同左 |
| 版本治理 | [`specs/2026-08-03-versioning-design.md`](docs/superpowers/specs/2026-08-03-versioning-design.md) | 同左 |
| 发布流程 | [`specs/2026-08-03-publishing-design.md`](docs/superpowers/specs/2026-08-03-publishing-design.md) | 同左 |
| 安全模型 | [`specs/2026-08-03-security-model-design.md`](docs/superpowers/specs/2026-08-03-security-model-design.md) | 同左 |
| 基准落档 | [`docs/release/benchmarks.md`](docs/release/benchmarks.md) | 同左 |
| 迁移测试台账 | [`specs/2026-08-03-migration-parity-ledger-design.md`](docs/superpowers/specs/2026-08-03-migration-parity-ledger-design.md) | 同左 |
| 验收报告 | [`specs/2026-08-03-acceptance-report-design.md`](docs/superpowers/specs/2026-08-03-acceptance-report-design.md) | 同左 |
| 生产就绪审计 | [`specs/2026-08-03-production-readiness-audit-design.md`](docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md) | 同左 |
| 可运行示例 | [`freemarker/examples/`](freemarker/examples/) | 7 个示例——`cargo run --example <name>` |
| 用户迁移指南 | [`docs/user-guide.md`](docs/user-guide.md) | Java → Rust 迁移指南（含代码对照） |
| API 稳定性承诺 | [`docs/api-stability.md`](docs/api-stability.md) | 版本策略、SemVer 承诺、API 基线 |
| API 参考 | [docs.rs](https://docs.rs/freemarker) | 源码 rustdoc 内含中英文注释 |

## 开发与发布

日常开发在 `dev` 分支进行，`main` 是发布分支。`main` 上的 `v*` tag 触发
`.github/workflows/release.yml`：执行 `cargo publish --dry-run` 并创建 GitHub Release，
附带 CHANGELOG 对应版本条目。

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo publish -p freemarker --dry-run
# freemarker-pyo3 的发布通过 pyo3-v* tag 与 .github/workflows/pyo3-publish.yml 协调
```

crates.io 上 `freemarker` 版本可被 PyPI 引用之前，请勿提前发布 PyPI package。

## 许可证

项目采用 [Apache License 2.0](LICENSE)。Apache FreeMarker 是 Apache Software Foundation
项目；本 Rust 迁移项目由 `easy-4-rust` 组织独立维护。

---

<div align="center">

[返回顶部](#readme-top) · [crates.io](https://crates.io/crates/freemarker) ·
[docs.rs](https://docs.rs/freemarker) ·
[Issues](https://github.com/easy-4-rust/freemarker-rust/issues)

</div>
