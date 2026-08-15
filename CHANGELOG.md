# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与
[语义化版本](https://semver.org/lang/zh-CN/)（晋级规则见
[docs/superpowers/specs/2026-08-03-versioning-design.md](docs/superpowers/specs/2026-08-03-versioning-design.md)）。

## [0.1.0-beta.0] - 2026-08-15

> 本版本相对 v0.1.0-alpha.1：**布局对齐轮 + 生产就绪计划 Stage 0-5 全部收口**——
> Java↔Rust 目录/文件 1:1（472 文件，412 MAPPED / 0 MISSING）、pyo3 API 面补齐、
> 解析器拆分、soak/proptest 稳定性验证。详见
> `docs/superpowers/plans/2026-08-15-production-readiness.md`。

### Added

- **布局 1:1 对齐**（2026-08-15 布局对齐轮，13 提交）：`xml/`→`ext/dom/`、
  `error/`/`builtins/` Java 对应文件→`core/`、`core/expression/` 平铺；
  182 个镜像文件补齐（TemplatePostProcessor 完整实现 + DOCTYPE 降级实现 +
  CFormat 家族/BuiltIn 基类/惰性集合/输出模型锚点）；结构对照 412 MAPPED / 0 MISSING
- **pyo3 配置桥接**：`FmConfiguration` 7 → **35 方法**（格式化 12 + 解析期 4 + 行为 6 +
  模板查找 3 + getter 3 等）；Python golden 套件翻转 3 用例
  （import/localization/number-literal，34→37）
- **soak 稳定性套件**（`soak_smoke.rs`）：8 线程 × 64000 次并发渲染全成功 +
  首末输出逐字节一致 + 120s 死锁守卫；内存探针 5000 轮无退化
- **TEMPLATE 后处理钩子**：`TemplatePostProcessor` trait + 注册表 +
  `Configuration::add/remove_template_post_processor` + TemplateCache 集成（7 测试）
- **DOCTYPE 降级支持**：自扫 DOCTYPE 声明（roxmltree 0.21 无 API）+
  `@document_type$name` 语义对齐（7 测试）

### Changed

- **解析器拆分**：`grammar.rs` 6,837 行 → 13 文件（全部 ≤800 行），
  零行为变化（995 测试全绿 + public-api diff = 0）
- **public-api 基线**：6302 → 6054 项（路径重排所致，CI 门禁已验证绿）

### 验证（beta.0 门禁复核）

- workspace 测试 **997 passed / 0 failed**（含 soak）+ pytest 81 passed
- golden 113/128 MIRRORED + 15 有据永久 NA = **适用范围 100%**（0 FAIL / 0 BLOCKED）
- proptest **50000** 用例无 panic（单轮扩量实测）
- cargo-llvm-cov 85.02%；cargo fmt/clippy/deny/audit 全绿；多 OS CI 12 job 全绿

## [0.1.0-alpha.1] - 2026-08-03

> 本版本相对 v0.1.0-alpha.0：**生产就绪计划 v2 阶段 A/B/C 全部收口**——内建 183/183、
> golden 82 → **113/128（88%）定格**（0 FAIL / 0 BLOCKED，15 项永久 NA 分类确定化）、
> pyo3 一键可发布。详见 `docs/superpowers/specs/2026-08-03-acceptance-report-design.md`（v13）+ `docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md`。

### Added
- 内建函数补齐最后 5 个：`eval_json`/`is_date_like`/`next_sibling`/`previous_sibling`/
  `web_safe`（Java 2.3.34 内建名 183/183 全覆盖）
- XML visit 前缀宏分派（Java getNodeProcessor 语义）、`node[0]` 自身索引、
  XPath 子集 `./` 相对路径与 `true()` 函数
- `?api`/`?has_api` 支持：新增 `TemplateApiSupport` trait 与 `TModel.api` 槽位
  （对应 Java `TemplateModelWithAPISupport`；API 视图由包装方提供，引擎无反射）
- `?new` 四解析策略（unrestricted/safer/allows_nothing/opt-in + trusted_templates）
- ICI 版本化：`?html` <2.3.20 用 HTMLEnc、哈希字面量 <2.3.21 保留重复键、
  `?is_sequence` <2.3.24 / `?is_enumerable` <2.3.21 不排除方法模型
- pyo3 发布准备：pyproject readme/authors/classifiers/license-files、
  LICENSE 文件（Apache-2.0）、pyo3-publish workflow（Trusted Publishing）
- cargo-fuzz 启用：expression/parser target 声明（nightly 构建验证）

### Changed
- golden 套件 PASS 90 → **113/128（88%）定格**（B6 harness 收口 + B2/B3 ICI 版本化
  与 ?new 策略 + B5 XML 扩展 + B4 ?api；0 FAIL / 0 BLOCKED）
- 15 项 SKIP 全部登记**永久 NA**（分类确定化，golden.rs `permanent_na_reason`）：
  JVM 反射 12（beans + BeansWrapper 方法重载 11）+ transforms 1 + jython25 过期断言 2
- 公开 API 面新增 `TemplateApiSupport` trait + `TModel::api` 字段（api-baseline 已重生成）
- proptest fuzz 10000 用例（versioning.md 1.0 晋级条件 6）

## [0.1.0-alpha.0] - 2026-08-03

> 本版本相对之前提交基线（840ffe4 feat(xml) 等）：**完成 freemarker-rust 生产就绪第一轮治理 + BLOCKED 清零 + 鲁棒性/安全最小集**。
> 详见 `docs/superpowers/specs/2026-08-03-versioning-design.md`（alpha→1.0 晋级规则）+ `docs/superpowers/specs/2026-08-03-publishing-design.md`（发布流程）+ `docs/superpowers/specs/2026-08-03-security-model-design.md`（安全模型 + 决策 1 受限子集）。

### Added

- **依赖与许可治理门禁**：`deny.toml`（licenses allow 列表：MIT/Apache-2.0/BSD/ISC/Unicode/MPL/CC0/BSL/Python-2.0/Unlicense/CDLA-Permissive-2.0；multiple-versions=warn；sources only crates.io）+ `.cargo/audit.toml`（RUSTSEC 豁免登记） + CI 接入 `EmbarkStudios/cargo-deny-action@v2` 与 `cargo audit` 步骤。
- **公共 API 基线**：`docs/release/api-baseline.txt`（freemarker 核心 3,705 项公开 API 快照）+ CI `cargo public-api` diff 门禁。
- **workspace 元数据补全**：`[workspace.package]` 增补 description/authors/repository/homepage/categories/keywords；各成员 crate 补 categories/keywords/repository（freemarker-pyo3 publish=true 但本轮不实际发布；freemarker-test publish=false）。
- **版本治理文档**：`docs/superpowers/specs/2026-08-03-versioning-design.md`（alpha→1.0 晋级规则与可执行门禁）。
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