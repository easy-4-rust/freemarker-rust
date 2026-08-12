# 文件级拆分批次 2c + 3a + 3b 与功能块补齐计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成严格一文件一对象拆分批次 2c（template 剩余 17 文件）、3a（template/utility 变换家族 9 文件）、3b（utility 剩余 19 文件）、功能块缺口补齐（4 项）、Java<->Rust 目录结构对齐。

**Architecture:** 在 P6 Stage 1-2 基础上继续拆分：每个 Java 类在 Rust 中建立独立 .rs 文件，聚合枚举（ExprKind/ElementKind/TemplateError/OutputFormatKind）保留为聚合 API。同时补齐 4 个功能块缺口：per-template 配置、c_format 变体、自动转义禁令、get_optional_template。

**Tech Stack:**
- 无新增依赖（纯结构重构 + 语义补全）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-architecture-design.md`、`docs/superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md`

---

## 全局约定

- **一文件一对象**：每个 Java 类在 Rust 中建立独立 .rs 文件
- **聚合 API 保留**：ExprKind/ElementKind/TemplateError/OutputFormatKind 枚举不拆
- **dispatch 切换**：从 enum match 切换为 struct 方法调用

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | 批次 2c：template 剩余 17 文件 | 1 |
| 2 | 批次 3a + 3b：utility 变换家族 + 剩余 | 2 |
| 3 | 功能块缺口补齐（4 项） | 2 |
| 4 | 目录结构对齐 + 规范核对 | 1 |

---

## Stage 1 — 批次 2c

### Task 1.1：template 剩余 17 文件拆分

**Files:**
- Create: `freemarker/src/template/` 下 17 个独立 .rs 文件

- [x] **Step 1:** 拆分 template 模块剩余 17 个文件
- [x] **Step 2:** Commit — `477d30f refactor: 严格一文件一对象拆分批次 2c（template 剩余 17 文件）`

---

## Stage 2 — 批次 3a + 3b

### Task 2.1：template/utility 变换家族 9 文件

**Files:**
- Create: `freemarker/src/template/utility/` 下 9 个变换家族文件

- [x] **Step 1:** 拆分 utility 变换家族
- [x] **Step 2:** Commit — `06c6d69 refactor: 严格一文件一对象拆分批次 3a（template/utility 变换家族 9 文件）`

---

### Task 2.2：utility 剩余 19 文件

**Files:**
- Create: `freemarker/src/template/utility/` 下 19 个文件

- [x] **Step 1:** 拆分 utility 剩余文件
- [x] **Step 2:** Commit — `5e73163 refactor: 严格一文件一对象拆分批次 3b（utility 剩余 19 文件）`

---

## Stage 3 — 功能块缺口补齐

### Task 3.1：per-template 配置 + c_format 变体 + 自动转义禁令

**Files:**
- Create: `freemarker/src/core/template_configuration.rs`
- Modify: `freemarker/src/core/cformat.rs`
- Modify: `freemarker/src/core/eval.rs`
- Modify: `freemarker/src/builtins/strings_encoding.rs`

- [x] **Step 1:** 实现 TemplateConfiguration（per-template 配置体系）
- [x] **Step 2:** 实现 CFormatKind + Settings.c_format + ?c/?cn 变体分派
- [x] **Step 3:** 实现 check_legacy_escaping_ban（?html/?xml/?rtf/?web_safe）
- [x] **Step 4:** Commit — `6e997ed feat(align): Java<->Rust 目录结构对齐 + 缺口补齐`

---

### Task 3.2：get_optional_template + StatefulTemplateLoader

**Files:**
- Create: `freemarker/src/core/get_optional_template_method.rs`
- Modify: `freemarker/src/cache/stateful_template_loader.rs`

- [x] **Step 1:** 实现 GetOptionalTemplateMethod
- [x] **Step 2:** 实现 TemplateLoader::reset_state 默认钩子
- [x] **Step 3:** Commit — `240f997 feat: 补齐 4 个功能块缺口（Java<->Rust 一一对应核对驱动）`

---

## Stage 4 — 目录结构对齐

### Task 4.1：规范核对修正路径差异

**Files:**
- Modify: 多个模块路径调整

- [x] **Step 1:** 规范核对修正路径差异（/goal 命名与目录映射规则）
- [x] **Step 2:** Commit — `4427b29 refactor: 规范核对修正路径差异（/goal 命名与目录映射规则）`
- [x] **Step 3:** Commit — `1d2cde5 noop: 收尾对齐`

---

## 实际完成状态

- **日期**：2026-08-04
- **提交**：`477d30f`、`06c6d69`、`5e73163`、`6e997ed`、`240f997`、`4427b29`、`1d2cde5`、`ab31b06`
- **文件数增长**：freemarker/src .rs 文件从 ~90 增长到 ~200+
- **验收**：cargo test --workspace 全绿
- **状态**：全部 `- [x]` 完成
