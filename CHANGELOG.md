# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与
[语义化版本](https://semver.org/lang/zh-CN/)（晋级规则见
[docs/release/versioning.md](docs/release/versioning.md)）。

## [0.1.0-alpha.0] - 2026-08-03

> 本版本相对之前提交基线（840ffe4 feat(xml) 等）：**完成 freemarker-rust 生产就绪第一轮治理 + BLOCKED 清零 + 鲁棒性/安全最小集**。
> 详见 `docs/release/versioning.md`（alpha→1.0 晋级规则）+ `docs/release/publishing.md`（发布流程）+ `docs/release/security.md`（安全模型 + 决策 1 受限子集）。

### Added

- **依赖与许可治理门禁**：`deny.toml`（licenses allow 列表：MIT/Apache-2.0/BSD/ISC/Unicode/MPL/CC0/BSL/Python-2.0/Unlicense/CDLA-Permissive-2.0；multiple-versions=warn；sources only crates.io）+ `.cargo/audit.toml`（RUSTSEC 豁免登记） + CI 接入 `EmbarkStudios/cargo-deny-action@v2` 与 `cargo audit` 步骤。
- **公共 API 基线**：`docs/release/api-baseline.txt`（freemarker 核心 3,705 项公开 API 快照）+ CI `cargo public-api` diff 门禁。
- **workspace 元数据补全**：`[workspace.package]` 增补 description/authors/repository/homepage/categories/keywords；各成员 crate 补 categories/keywords/repository（freemarker-pyo3 publish=true 但本轮不实际发布；freemarker-test publish=false）。
- **版本治理文档**：`docs/release/versioning.md`（alpha→1.0 晋级规则与可执行门禁）。
- **发布 workflow**：`.github/workflows/release.yml`（tag v* 触发 `cargo publish --dry-run -p freemarker` + GitHub Release 自动生成 + CHANGELOG 提取）。
- **多 OS CI 矩阵**：ubuntu-latest + macos-latest + windows-latest × stable；MSRV 1.85 独立 job（与 Cargo.toml `rust-version = "1.85"` 对齐）。
- **docs.rs 元数据**：`freemarker/Cargo.toml` `[package.metadata.docs.rs]`（all-features + rustdoc-args）。
- **proptest fuzz 鲁棒性**：`freemarker-test/tests/robustness_fuzz_smoke.rs`（解析器 + 渲染 smoke，1024 用例无 panic）。
- **criterion 基准落档**：`freemarker/benches/simple_render.rs`（5 指标）+ `docs/release/benchmarks.md` 基线表。
- **安全/边界测试套件**：`freemarker-test/tests/security_smoke.rs`（`?api` 恒错误、`?new` 白名单边界、UTF-8 字节有效性、ICI 2.3.34 默认）。

### Fixed

- freemarker-pyo3 `Cargo.toml` 元数据字段位置修正（categories/keywords 从 `[lib]` 段后移至 `[package]` 段内）。
- `freemarker/src/parser/grammar.rs:113-114` + `lexer.rs:254-255`：反引号 doc comment 跨多行导致 Rustdoc 解析失败、编译中断 14 errors（`Syntax error in template "{name}" in line L, column C:
{details}` 包裹 inline code 跨多行），改为普通文本描述。

### Governance（全部以可执行门禁形式落档 CI）

- `cargo fmt --all --check` — 绿
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 error
- `cargo test --workspace --exclude freemarker-pyo3` — 317 passed / 1 FAILED（macros2 预存在回归，登记为 B6 后续迭代）
- `cargo deny check` — 全绿
- `cargo audit` — 0 error
- `cargo public-api -p freemarker` diff `docs/release/api-baseline.txt` — 0 diff
- `cargo package -p freemarker` — 144 files / 1.3MiB（演练通过）

### Migration（golden suite 验收 v12）

- **MIRRORED = 87**（82 → +5：output-encoding2/3、number-literal、bean-maps、identifier-escaping + 1 项文档同步）
- **NOT_APPLICABLE = 41**（含 transforms 用例补登记；JVM 反射 33 项不可恢复永久保留——决策 1）
- **BLOCKED = 0**（5→0：4 项工程量修复启用现有 transcode_output / 解析器 / 夹具 / `?sort`；1 项 `transforms` 文档登记）
- **FAIL = 1**（macros2 预存在回归——宏嵌套求值 "c null or missing"；不属于本轮 BLOCKED 修复范围，已开 issue 跟踪）

### 不在本版本（决策 1：受限子集永久保留）

- **JVM 反射不实现**（POJO 反射 25 + `?api` 2 + `?new` 任意类 4 + XML 节点 2 = 33 项 NOT_APPLICABLE 永久）
- **freemarker-pyo3 本轮不发布**（PyPI 通路独立，需 maturin + twine；本版本仅元数据完整备后续）
- **cargo-fuzz 长期 fuzz 目标**（留文档，本轮用 proptest 替代）
- **Servlet/JSP 集成**（不在 v1 范围）
- **输出格式（RTF/CSS/JSON/JavaScript/PlainText）**（v1 P4 TODO）