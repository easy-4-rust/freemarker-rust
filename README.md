# freemarker-rust

> **Apache FreeMarker 语义兼容的 Rust 模板引擎**（当前 0.1.0，alpha 早期）。
> 上游基线：Apache FreeMarker `2.3-gae` commit `7926e97`，improvements `2.3.34`。
> **受限子集**：JVM 反射不实现（33 项 NOT_APPLICABLE 永久保留——见 [security.md](docs/release/security.md)）。

## crates

| crate | 角色 | 状态 |
|---|---|---|
| [`freemarker`](freemarker/src/lib.rs) | 核心模板引擎 | 本轮可演练发布到 crates.io |
| [`freemarker-test`](freemarker-test/) | 整体功能测试模块（含 105 模块 java_ported + 128 用例 golden runner） | 不发布（publish=false） |
| [`freemarker-pyo3`](freemarker-pyo3/) | Python 绑定（pyo3 0.29）替代 jython25 | 本轮不发布到 crates.io/PyPI；元数据完整备后续 |

## 快速开始

```toml
# Cargo.toml
[dependencies]
freemarker = "0.1.0"
```

```rust
use freemarker::template::{Configuration, SimpleHash, TModel};
use freemarker::parser::parse;
use indexmap::IndexMap;

let cfg = Configuration::new();
let tpl = parse(&cfg, "hello", "Hello ${name}!").unwrap();

let mut root = IndexMap::default();
root.insert("name".to_owned(), TModel::from_scalar("World".to_owned()));
let root = SimpleHash(root);

let mut out = Vec::new();
tpl.process(TModel::from_hash_simplehash_or_direct(root), &mut out).unwrap();
assert_eq!(out, b"Hello World!");
```

## 治理门禁

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --exclude freemarker-pyo3`（**87 MIRRORED / 41 NOT_APPLICABLE / 0 BLOCKED** golden；509 java_ported 测试）
- `cargo deny check`（licenses/bans/sources/advisories）
- `cargo audit`（0 error）
- `cargo public-api -p freemarker` diff `docs/release/api-baseline.txt`（3,705 项基线）
- 多 OS 矩阵（ubuntu/macos/windows × stable + MSRV 1.85）
- `cargo package -p freemarker`（演练发布）

## 文档

- [docs/](docs/) — 设计文档（13 篇：项目概述、范围、迁移路线、模板引擎、解析器、缓存、格式化、安全等）
- [docs/release/versioning.md](docs/release/versioning.md) — 版本治理（alpha→1.0 晋级规则）
- [docs/release/publishing.md](docs/release/publishing.md) — 发布流程
- [docs/release/security.md](docs/release/security.md) — 安全模型 + 受限子集
- [docs/release/api-baseline.txt](docs/release/api-baseline.txt) — 公共 API 基线
- [docs/release/benchmarks.md](docs/release/benchmarks.md) — criterion 基准落档
- [docs/测试/验收报告.md](docs/测试/验收报告.md) — golden + java_ported 验收 v12
- [docs/测试/迁移测试对照表.md](docs/测试/迁移测试对照表.md) — 128 用例逐条 disposition

## 限制（决策 1）

- **POJO 反射不实现**：模板中 POJO 访问、?api、?new 任意类、Enum、Static class 不可用
- **freemarker-pyo3 本轮不发布**：需 maturin + PyPI 通路
- **macros2 预存在回归**：宏嵌套求值边界（`c null or missing`），已开 issue 跟踪

## 许可

Apache-2.0（与 Apache FreeMarker 上游一致）。