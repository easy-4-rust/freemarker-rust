# M5 错误对齐收尾计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成 M5 错误对齐收尾——ErrorCtx 装箱、宏默认值 Java 重试语义、数字键错误对齐、70 场景 parity 全过、词法层 Java 对齐、fuzz 防御。

**Architecture:** 在 P1-P4 错误体系基础上进行收口：ErrorCtx 从栈上内联改为 Box 堆分配（减少 TemplateError 枚举体大小），宏默认值参数增加 Java 重试语义（参数解析失败时 fallback 到默认值），数字键访问错误消息逐字对齐 Java 基线。词法层错误消息对齐 Java ParseException 格式。

**Tech Stack:**
- 无新增依赖（纯错误模型优化 + 消息对齐）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-error-handling-design.md`

---

## 全局约定

- **ErrorCtx 装箱**：`TemplateError` 枚举中的 `ErrorCtx` 字段改为 `Box<ErrorCtx>`，减少枚举体大小
- **70 场景 parity**：`error/expected_messages/` 下 70 个场景全部与 Java 输出逐字对齐
- **词法层对齐**：解析器 `ParseException` 错误消息格式（行/列/期望清单）与 Java 一致

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | ErrorCtx 装箱 + 宏默认值重试语义 | 2 |
| 2 | 数字键错误对齐 + 70 场景 parity | 2 |
| 3 | 词法层 Java 对齐 + fuzz 防御 | 2 |

---

## Stage 1 — ErrorCtx 装箱 + 宏默认值

### Task 1.1：ErrorCtx 装箱

**Files:**
- Modify: `freemarker/src/error/mod.rs`
- Modify: `freemarker/src/error/template_error.rs`

- [x] **Step 1:** `TemplateError` 枚举中 `ErrorCtx` 字段改为 `Box<ErrorCtx>`
- [x] **Step 2:** 更新所有 `TemplateError` 构造点（约 460 处）适配 Box
- [x] **Step 3:** 验证 `cargo test --workspace` 全绿
- [x] **Step 4:** Commit — `fix(engine): M5 错误对齐收尾——ErrorCtx 装箱`

---

### Task 1.2：宏默认值 Java 重试语义

**Files:**
- Modify: `freemarker/src/core/eval.rs`
- Modify: `freemarker/src/parser/grammar.rs`

- [x] **Step 1:** 宏默认值参数解析失败时 fallback 到默认值（Java 重试语义）
- [x] **Step 2:** 数字键访问错误消息对齐 Java `InvalidReferenceException` 格式
- [x] **Step 3:** Commit — `fix(engine): M5 错误对齐收尾——宏默认值 Java 重试语义 + 数字键错误对齐`

---

## Stage 2 — 70 场景 parity

### Task 2.1：错误消息 parity 全过

**Files:**
- Modify: `freemarker/src/error/expected_messages/`（70 个场景文件）
- Modify: `freemarker/src/core/eval.rs`（错误消息生成逻辑）

- [x] **Step 1:** 逐场景对比 Java 输出，修正 Rust 错误消息
- [x] **Step 2:** 验证 70/70 场景 parity 全过
- [x] **Step 3:** Commit

---

### Task 2.2：词法层 Java 对齐

**Files:**
- Modify: `freemarker/src/parser/lexer.rs`
- Modify: `freemarker/src/parser/grammar.rs`

- [x] **Step 1:** 解析器 `ParseException` 错误消息格式对齐 Java（行/列/期望清单）
- [x] **Step 2:** 词法状态切换错误消息逐字对齐
- [x] **Step 3:** Commit

---

## Stage 3 — fuzz 防御

### Task 3.1：fuzz 防御加固

**Files:**
- Modify: `freemarker/src/parser/grammar.rs`
- Modify: `freemarker/src/core/eval.rs`

- [x] **Step 1:** 解析器嵌套深度限制（防栈溢出）
- [x] **Step 2:** 表达式求值循环计数限制（防无限循环）
- [x] **Step 3:** Commit — `feat(m5): 错误对齐收尾——70 场景 parity 全过 + 词法层 Java 对齐 + fuzz 防御`

---

## 实际完成状态

- **日期**：2026-08-03
- **提交**：`d67d1c3`、`159f530`
- **验收**：70 场景 parity 全过；proptest fuzz 10000 用例无 panic
- **状态**：全部 `- [x]` 完成
