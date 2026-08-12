# Parser #on 指令 + ?eval_json 内建计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 实现 Java FreeMarker 2.3.28+ 的 #on 指令和 ?eval_json 内建函数——P6 Stage 6 收官。

**Architecture:** #on 指令是 Java 2.3.28 引入的错误处理指令，语法 `[#on error_message]...[/#on]`，在模板渲染期间捕获指定错误并执行备选内容。?eval_json 内建函数将 JSON 字符串解析为 FreeMarker 数据模型（哈希/列表/标量）。

**Tech Stack:**
- 无新增依赖（手写递归下降解析器扩展 + serde_json 复用）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-parser-design.md`、`docs/superpowers/specs/2026-08-02-builtins-design.md`

---

## 全局约定

- **#on 指令**：Java 2.3.28+ 语法，解析器新增指令产生式
- **?eval_json**：JSON 字符串 -> TModel（哈希/列表/标量），复用 serde_json
- **错误对齐**：#on 指令的错误消息格式与 Java 基线一致

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | #on 指令实现 | 1 |
| 2 | ?eval_json 内建实现 | 1 |

---

## Stage 1 — #on 指令

### Task 1.1：#on 指令解析与执行

**Files:**
- Modify: `freemarker/src/parser/grammar.rs`
- Modify: `freemarker/src/core/eval.rs`

- [x] **Step 1:** 解析器新增 #on 指令产生式（`[#on error_message]...[/#on]`）
- [x] **Step 2:** 渲染引擎实现 #on 指令执行逻辑（错误捕获 + 备选内容渲染）
- [x] **Step 3:** Commit（与 Step 2 合并提交）

---

## Stage 2 — ?eval_json 内建

### Task 2.1：?eval_json 内建函数

**Files:**
- Modify: `freemarker/src/builtins/format.rs`
- Modify: `freemarker/src/builtins/mod.rs`

- [x] **Step 1:** 实现 ?eval_json 内建函数——JSON 字符串解析为 TModel
- [x] **Step 2:** 注册到 builtins 注册表
- [x] **Step 3:** Commit — `931b0de feat(parser): 实现 Java 2.3.28+ 的 #on 指令和 ?eval_json 内建函数`

---

## 实际完成状态

- **日期**：2026-08-05
- **提交**：`931b0de`
- **验收**：cargo test --workspace 全绿；#on 指令 + ?eval_json 语义对齐 Java 2.3.28+
- **状态**：全部 `- [x]` 完成
- **意义**：P6 打磨与对齐计划收官，freemarker-rust 功能对齐 Java FreeMarker 2.3.34
