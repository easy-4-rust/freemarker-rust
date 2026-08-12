# P0 骨架与基线 — freemarker-rust 初始实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 建立 freemarker-rust 项目骨架——Cargo workspace、错误体系、基础类型、依赖锁定、D1-D5 决策评审落档、L3 对比 harness 骨架。

**Architecture:** Cargo workspace（`freemarker/` 核心引擎 crate + `freemarker-pyo3/` Python 绑定 crate），核心 crate 内按 `parser/`、`core/`、`template/`、`error/`、`builtins/`、`cache/`、`xml/` 模块组织。错误体系统一为 `TemplateError` enum + `ErrorCtx` 消息上下文。基础类型 `Span`/`TNumber`/`DateValue`/`Locale`/`TimeZone` 适配 Rust 生态。

**Tech Stack:**
- Rust edition 2024，MSRV 1.85
- regex / bigdecimal / chrono / indexmap / once_cell / thiserror
- pyo3 0.29（feature gate）
- roxmltree（XML 解析）
- fancy-regex（正则环视/反向引用）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-project-overview-design.md`、`docs/superpowers/specs/2026-08-01-architecture-design.md`

---

## 全局约定

- **版本基线**：Apache FreeMarker v2.3.34（commit `7926e97`，2.3 分支线）
- **incompatibleImprovements**：支持到 2.3.34
- **决策落档**：D1（serde 替代 BeansWrapper）、D2（fancy-regex 替代 Java 正则）、D3（ICI 锁定 2.3.34）、D4（Rust Result 替代 Java 异常传播）、D5（无日志框架，用 Result 传播）
- **提交约定**：conventional commits（feat/fix/docs/test/refactor）

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | Cargo workspace 初始化 | 2 |
| 2 | 错误体系骨架 | 2 |
| 3 | 基础类型（Span/TNumber/DateValue/Locale/TimeZone） | 2 |
| 4 | 依赖锁定 + D1-D5 决策落档 | 1 |
| 5 | L3 harness 骨架 | 1 |

---

## Stage 1 — Cargo Workspace 初始化

### Task 1.1：创建 workspace 结构

**Files:**
- Create: `Cargo.toml`（[workspace] members）
- Create: `freemarker/Cargo.toml`
- Create: `freemarker/src/lib.rs`
- Create: `freemarker-pyo3/Cargo.toml`
- Create: `freemarker-pyo3/src/lib.rs`
- Create: `.gitignore`
- Create: `rustfmt.toml`
- Create: `.clippy.toml`

- [x] **Step 1:** 创建根 `Cargo.toml` 定义 workspace members
- [x] **Step 2:** 创建 `freemarker/` 核心 crate 骨架（Cargo.toml + lib.rs）
- [x] **Step 3:** 创建 `freemarker-pyo3/` Python 绑定 crate 骨架
- [x] **Step 4:** 配置 rustfmt.toml + .clippy.toml
- [x] **Step 5:** 验证 `cargo build` 通过
- [x] **Step 6:** Commit — `feat: freemarker-rust 初始提交——Apache FreeMarker 语义兼容的 Rust 实现`

---

### Task 1.2：创建 freemarker-test 测试模块

**Files:**
- Create: `freemarker-test/Cargo.toml`
- Create: `freemarker-test/src/lib.rs`
- Create: `freemarker-test/tests/` 目录结构

- [x] **Step 1:** 创建 freemarker-test crate
- [x] **Step 2:** 初始化测试目录结构（java-tests/、tests/）
- [x] **Step 3:** Commit — `refactor: 新增 freemarker-test 整体功能测试模块`

---

## Stage 2 — 错误体系骨架

### Task 2.1：TemplateError enum + ErrorCtx

**Files:**
- Create: `freemarker/src/error/mod.rs`
- Create: `freemarker/src/error/template_error.rs`
- Create: `freemarker/src/error/error_ctx.rs`
- Create: `freemarker/src/error/flow_kind.rs`

- [x] **Step 1:** 设计 `TemplateError` enum（覆盖 Java 异常层级全族）
- [x] **Step 2:** 实现 `ErrorCtx` 消息上下文（模板名/行/列/期望值）
- [x] **Step 3:** 实现 `FlowKind`（RETURN/BREAK/CONTINUE/STOP 流控枚举）
- [x] **Step 4:** 编译通过
- [x] **Step 5:** Commit

---

### Task 2.2：异常类族镜像文件

**Files:**
- Create: `freemarker/src/error/template_exception.rs`
- Create: `freemarker/src/error/invalid_reference_exception.rs`
- Create: `freemarker/src/error/unexpected_type_exception.rs`
- Create: `freemarker/src/error/non_*_exception.rs`（8 个）
- Create: `freemarker/src/error/misc_template_exception.rs`
- Create: `freemarker/src/error/_misc_template_exception.rs`
- Create: `freemarker/src/error/parse_exception.rs`
- Create: `freemarker/src/error/stop_exception.rs`
- Create: `freemarker/src/error/return_exception.rs`
- Create: `freemarker/src/error/break_or_continue_exception.rs`
- Create: `freemarker/src/error/template_not_found_exception.rs`
- Create: `freemarker/src/error/template_model_exception.rs`

- [x] **Step 1:** 实现各异常结构体（委托 TemplateError 构造）
- [x] **Step 2:** 确保 460+ 调用点零改动（TemplateError 构造方法统一入口）
- [x] **Step 3:** Commit — `refactor: error 异常类拆分（Java 异常层级 17 文件）`

---

## Stage 3 — 基础类型

### Task 3.1：Span + value.rs

**Files:**
- Create: `freemarker/src/span.rs`
- Create: `freemarker/src/value.rs`

- [x] **Step 1:** 实现 `Span`（行/列/偏移量，源码位置追踪）
- [x] **Step 2:** 实现 `DynValue` / `TNumber` 值体系（对应 Java TemplateNumberModel）
- [x] **Step 3:** Commit

---

### Task 3.2：Locale / TimeZone 适配

**Files:**
- Modify: `freemarker/src/value.rs`（DateValue）
- Modify: `freemarker/src/template/mod.rs`（Locale/TimeZone 类型别名）

- [x] **Step 1:** 适配 chrono 时区语义（对应 Java TimeZone）
- [x] **Step 2:** 适配 locale 语义（Rust 无原生 Locale，用字符串标识）
- [x] **Step 3:** Commit

---

## Stage 4 — 依赖锁定 + 决策落档

### Task 4.1：锁定核心依赖 + D1-D5 文档化

**Files:**
- Modify: `freemarker/Cargo.toml`
- Create: `docs/superpowers/specs/2026-08-01-project-overview-design.md`（决策 D1-D5 落档）

- [x] **Step 1:** 锁定 regex / bigdecimal / chrono / indexmap / once_cell / thiserror
- [x] **Step 2:** D1-D5 决策落档到 01 文档
- [x] **Step 3:** Commit

---

## Stage 5 — L3 Harness 骨架

### Task 5.1：Java Probe + 对比脚本

**Files:**
- Create: `scripts/java_probe/`（Java 渲染任意模板 -> JSON）
- Create: `scripts/` 对比脚本

- [x] **Step 1:** 创建 Java probe 骨架（读模板 + 数据 -> 渲染 -> JSON 输出）
- [x] **Step 2:** 创建 Rust 对比脚本骨架
- [x] **Step 3:** Commit

---

## 验收标准

1. `cargo build` 通过
2. `cargo test` 空跑绿
3. D1-D5 有决议并落档
4. L3 harness 骨架可运行（双引擎输出 JSON diff）

## 实际完成状态

- **日期**：2026-08-01
- **Git 提交**：`feat: freemarker-rust 初始提交` + `refactor: 新增 freemarker-test`
- **验收**：全部通过（详见 git log 2026-08-01）
