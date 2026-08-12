# 内建函数对齐 + 核心模块拆分 + java_ported 测试计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成 builtins 三批对齐、core AST/指令/异常/格式拆分（8+6+1+1 批次）、xml 模块拆分、?api/?has_api 支持、java_ported 测试新增（12 个）。

**Architecture:** 在 P6 Stage 3 基础上进行大规模文件级拆分与 builtins 对齐：core AST 拆分 8 批（26 个镜像文件）、指令类拆分 6 批（38 个镜像文件）、异常类拆分 17 文件、格式/输出模型拆分 14 文件。builtins 对齐 3 批（Hashes + MarkupOutputs + StringsMisc + Sequences）。xml 模块拆分 12 文件。java_ported 测试新增 12 个。

**Tech Stack:**
- 无新增依赖（纯结构重构 + 测试补全）

**Related Design Doc:** `docs/superpowers/specs/2026-08-02-builtins-design.md`、`docs/superpowers/specs/2026-08-01-rendering-engine-design.md`、`docs/superpowers/specs/2026-08-01-error-handling-design.md`

---

## 全局约定

- **一文件一对象**：每个 Java 类在 Rust 中建立独立 .rs 文件
- **聚合 API 保留**：ExprKind/ElementKind/TemplateError/OutputFormatKind 枚举不拆
- **java_ported 测试**：Java 测试 1:1 移植，NOT_APPLICABLE 标记保留

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | core AST 拆分 8 批 | 1 |
| 2 | core 指令类拆分 6 批 | 1 |
| 3 | error 异常类 + 格式/输出模型拆分 | 2 |
| 4 | builtins 对齐 3 批 | 1 |
| 5 | xml 模块拆分 + ?api 支持 | 2 |
| 6 | java_ported 测试新增 12 个 | 1 |

---

## Stage 1 — core AST 拆分

### Task 1.1：core AST 拆分 8 批（26 个镜像文件）

**Files:**
- Create: `freemarker/src/core/expression/` 下 26 个 .rs 文件

- [x] **Step 1:** 首批：AddConcatExpression/AndExpression/OrExpression — `ef72c3b`
- [x] **Step 2:** 第二批：ArithmeticExpression/ComparisonExpression — `e7f7535`
- [x] **Step 3:** 第三批：字面量类 7 文件 — `9d3f5dd`
- [x] **Step 4:** 第四批：控制流/调用类 6 文件 — `5baa83b`
- [x] **Step 5:** 第五批：Dot/DynamicKeyName — `ddb3e46`
- [x] **Step 6:** 第六批：Range 家族 4 文件 — `b0d3139`
- [x] **Step 7:** 第七批：BuiltinVariable — `6c4f3fa`
- [x] **Step 8:** 第八批：BuiltIn 聚合 — `a085ce0`

---

## Stage 2 — core 指令类拆分

### Task 2.1：core 指令类拆分 6 批（38 个镜像文件）

**Files:**
- Create: `freemarker/src/core/` 下 38 个指令镜像文件

- [x] **Step 1:** 首批：简单指令 10 文件 — `dbdfff5`
- [x] **Step 2:** 第二批：文本/插值 2 文件 — `dd07434`
- [x] **Step 3:** 第三批：块指令 8 文件 — `40effc0`
- [x] **Step 4:** 赋值指令实现 + 重构为独立模块 — `49db59e`、`39f4039`
- [x] **Step 5:** 第五批：流程类 5 文件 — `fe91484`
- [x] **Step 6:** 第六批：宏/调用/节点 8 文件（指令类收官） — `480d72e`

---

## Stage 3 — error + 格式拆分

### Task 3.1：error 异常类拆分（17 文件）

**Files:**
- Create: `freemarker/src/error/` 下 17 个异常镜像文件

- [x] **Step 1:** 拆分异常层级 — `d215e11`
- [x] **Step 2:** Commit — `refactor: error 异常类拆分（Java 异常层级 17 文件）`

---

### Task 3.2：core 格式/输出模型拆分（14 文件）

**Files:**
- Create: `freemarker/src/core/` 下 14 个格式/输出模型文件

- [x] **Step 1:** OutputFormat 家族 + 输出模型接口 — `a69876b`
- [x] **Step 2:** Commit — `refactor: core 输出格式类 + 输出模型拆分（14 文件）`

---

## Stage 4 — builtins 对齐

### Task 4.1：builtins 对齐 3 批

**Files:**
- Modify: `freemarker/src/builtins/` 目录

- [x] **Step 1:** 首批：BuiltInsForHashes + BuiltInsForMarkupOutputs — `a60c3bf`
- [x] **Step 2:** 第二批：BuiltInsForStringsMisc — `9ecbe10`
- [x] **Step 3:** 第三批：BuiltInsForSequences 补全 — `8658b63`（注：git log 显示 `893bae7` 为最终 183/183 达成）
- [x] **Step 4:** strings_misc 文档格式 clippy 修复 — `1823ec9`

---

## Stage 5 — xml + ?api

### Task 5.1：xml 模块拆分（12 文件）

**Files:**
- Create: `freemarker/src/xml/` 下 12 个文件

- [x] **Step 1:** 拆分 xml/mod.rs 为 ns_prefixes.rs + tree.rs + node.rs + 11 模型类 — `96275cb`
- [x] **Step 2:** Commit — `refactor: xml 模块拆分（Java ext.dom 17 文件对齐）`

---

### Task 5.2：?api/?has_api 支持

**Files:**
- Modify: `freemarker/src/core/eval.rs`
- Create: `freemarker/src/core/api_support.rs`

- [x] **Step 1:** 实现 ?api/?has_api 支持 + TemplateApiSupport trait — `254f885`
- [x] **Step 2:** Commit — `feat(api): B4 ?api/?has_api 支持——golden 111→113`

---

## Stage 6 — java_ported 测试

### Task 6.1：java_ported 测试新增 12 个

**Files:**
- Create: `freemarker-test/java-tests/` 下 12 个测试文件

- [x] **Step 1:** CallerTemplateNameTest — `20ae0ad`
- [x] **Step 2:** CombinedMarkupOutputFormatTest — `6c10470`
- [x] **Step 3:** DirectiveCallPlaceTest — `c9366f8`
- [x] **Step 4:** EnvironmentCustomStateTest — `70752d4`
- [x] **Step 5:** IncludeAndImportConfigurableLayersTest — `cac405c`
- [x] **Step 6:** LegacyFMParserConstructorsTest — `371aa28`
- [x] **Step 7:** OptInTemplateClassResolverTest — `38cddee`
- [x] **Step 8:** TemplateProcessingTracerTest — `18ba931`
- [x] **Step 9:** ThreadInterruptingSupportTest — `589b6c6`
- [x] **Step 10:** RuntimeEnvironmentReporterTest — `969e31f`
- [x] **Step 11:** ExamplesTest — `9b58943`
- [x] **Step 12:** 3 个 NOT_APPLICABLE 标记补全 — `2f757b2`、`ced9231`、`d6aa165`

---

## 实际完成状态

- **日期**：2026-08-04
- **提交**：见各 Step 标注的 commit SHA
- **文件数增长**：freemarker/src .rs 从 ~90 增长到 291
- **结构对照**：422 MAPPED / 4 MISSING / 115 NA-DESIGN
- **验收**：cargo test --workspace 全绿（1009 tests）；builtins 183/183；golden 113/128
- **状态**：全部 `- [x]` 完成
