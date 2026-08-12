# P6 打磨与对齐计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成 freemarker-rust 的打磨与对齐——严格一文件一对象拆分、Java <-> Rust 目录结构对齐、core 语义补全、parser #on 指令 + ?eval_json 内建。

**Architecture:** 在 P0-P5 基础上进行结构优化：聚合枚举（ExprKind/ElementKind/TemplateError/OutputFormatKind）保留为聚合 API，各 Java 类建立镜像文件（struct + new + exec/eval 方法），dispatch 切换为 struct 方法调用。同时补齐 4 个功能块缺口（per-template 配置、c_format 变体、自动转义禁令、get_optional_template 等）。

**Tech Stack:**
- 无新增依赖（纯结构重构 + 语义补全）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-architecture-design.md`、`docs/superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md`

---

## 全局约定

- **一文件一对象**：每个 Java 类在 Rust 中建立独立 .rs 文件
- **聚合 API 保留**：ExprKind/ElementKind/TemplateError/OutputFormatKind 枚举不拆
- **dispatch 切换**：从 enum match 切换为 struct 方法调用
- **460 处调用点零改动**：TemplateError 构造方法全部委托镜像文件

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | 文件级拆分——cache 家族 + template.utility | 3 |
| 2 | 文件级拆分——template trait 家族 + 简单对象 + 剩余 | 3 |
| 3 | 文件级拆分——core AST + 指令类 + 异常 + 格式 | 6 |
| 4 | 功能块缺口补齐 | 4 |
| 5 | core 语义补全 | 4 |
| 6 | parser #on 指令 + ?eval_json 内建 | 1 |

---

## Stage 1 — 文件级拆分批次 1

### Task 1.1：cache 家族一文件一对象

**Files:**
- Create: `freemarker/src/cache/` 下 37 个独立文件（matcher 8 + factory 4 + exception 1 + loader 家族 + storage 家族）

- [x] **Step 1:** 拆分 cache 模块为一文件一对象
- [x] **Step 2:** Commit — `refactor: 严格一文件一对象拆分批次 1`

---

### Task 1.2：template.utility 目录拆分

**Files:**
- Create: `freemarker/src/template/utility/` 目录
- Create: `freemarker/src/template/utility/string_util.rs` 等

- [x] **Step 1:** 拆分 utility_transforms.rs 为独立文件
- [x] **Step 2:** Commit

---

### Task 1.3：TemplateSource 拆分

**Files:**
- Create: `freemarker/src/cache/template_source.rs`
- Modify: `freemarker/src/cache/template_loader.rs`

- [x] **Step 1:** 从 template_loader.rs 拆出 TemplateSource
- [x] **Step 2:** Commit

---

## Stage 2 — 文件级拆分批次 2-3

### Task 2.1：template trait 家族

**Files:**
- Create: `freemarker/src/template/` 下 17 个 trait 文件（template_scalar_model.rs 等）

- [x] **Step 1:** 拆分 template 模块为一文件一对象
- [x] **Step 2:** Commit — `refactor: 严格一文件一对象拆分批次 2a`

---

### Task 2.2：template 简单对象

**Files:**
- Create: 14 个简单对象文件（adapter/wrapping/iterator 等）

- [x] **Step 1:** 拆分
- [x] **Step 2:** Commit — `refactor: 严格一文件一对象拆分批次 2b`

---

### Task 2.3：template 剩余 + utility

**Files:**
- Create: 剩余 17 文件 + utility 19 文件

- [x] **Step 1:** 拆分
- [x] **Step 2:** Commit — `refactor: 严格一文件一对象拆分批次 2c + 3b`

---

## Stage 3 — 文件级拆分批次 4-8

### Task 3.1：core AST 拆分（8 批）

**Files:**
- Create: `freemarker/src/core/expression/` 下 26 个镜像文件

- [x] **Step 1:** 首批：AddConcatExpression/AndExpression/OrExpression
- [x] **Step 2:** 第二批：ArithmeticExpression/ComparisonExpression
- [x] **Step 3:** 第三批：字面量类 7 文件
- [x] **Step 4:** 第四批：控制流/调用类 6 文件
- [x] **Step 5:** 第五批：Dot/DynamicKeyName
- [x] **Step 6:** 第六批：Range 家族 4 文件
- [x] **Step 7:** 第七批：BuiltinVariable
- [x] **Step 8:** 第八批：BuiltIn 聚合
- [x] **Step 9:** Commit — `refactor: core AST 拆分`

---

### Task 3.2：core 指令类拆分（6 批）

**Files:**
- Create: `freemarker/src/core/` 下 38 个指令镜像文件

- [x] **Step 1:** 首批：简单指令 10 文件
- [x] **Step 2:** 第二批：文本/插值 2 文件
- [x] **Step 3:** 第三批：块指令 8 文件
- [x] **Step 4:** 第四批：赋值指令
- [x] **Step 5:** 第五批：流程类 5 文件
- [x] **Step 6:** 第六批：宏/调用/节点 8 文件（指令类收官）
- [x] **Step 7:** Commit — `refactor: core 指令类拆分`

---

### Task 3.3：error 异常类拆分

**Files:**
- Create: `freemarker/src/error/` 下 17 个异常镜像文件

- [x] **Step 1:** 拆分异常层级
- [x] **Step 2:** Commit — `refactor: error 异常类拆分（Java 异常层级 17 文件）`

---

### Task 3.4：core 格式/输出模型拆分

**Files:**
- Create: `freemarker/src/core/` 下 14 个格式/输出模型文件

- [x] **Step 1:** OutputFormat 家族 + 输出模型接口
- [x] **Step 2:** Commit — `refactor: core 输出格式类 + 输出模型拆分（14 文件）`

---

### Task 3.5：builtins 对齐

**Files:**
- Modify: `freemarker/src/builtins/` 目录

- [x] **Step 1:** 首批：BuiltInsForHashes + BuiltInsForMarkupOutputs
- [x] **Step 2:** 第二批：BuiltInsForStringsMisc
- [x] **Step 3:** 第三批：BuiltInsForSequences 补全
- [x] **Step 4:** Commit — `refactor: builtins 对齐`

---

### Task 3.6：xml 模块拆分

**Files:**
- Create: `freemarker/src/xml/` 下 12 个文件（xml_dom_string_util + 11 模型类锚点）

- [x] **Step 1:** 拆分 xml/mod.rs 为 ns_prefixes.rs + tree.rs + node.rs + 11 模型类
- [x] **Step 2:** Commit — `refactor: xml 模块拆分（Java ext.dom 17 文件对齐）`

---

## Stage 4 — 功能块缺口补齐

### Task 4.1：per-template 配置体系

**Files:**
- Create: `freemarker/src/core/template_configuration.rs`
- Create: `freemarker/src/cache/` matcher 8 + factory 4 + exception 1

- [x] **Step 1:** 实现 TemplateConfiguration（渲染期设置 Option + apply_to/merge）
- [x] **Step 2:** 实现 Configuration.set_template_configurations + 加载路径应用
- [x] **Step 3:** Commit

---

### Task 4.2：c_format 变体

**Files:**
- Modify: `freemarker/src/core/cformat.rs`（CFormatKind 枚举）

- [x] **Step 1:** 实现 CFormatKind + Settings.c_format + 设置解析 + ?c/?cn 变体分派
- [x] **Step 2:** Commit

---

### Task 4.3：自动转义禁令

**Files:**
- Modify: `freemarker/src/core/eval.rs`
- Modify: `freemarker/src/builtins/strings_encoding.rs`

- [x] **Step 1:** 实现 check_legacy_escaping_ban（?html/?xml/?rtf/?web_safe）
- [x] **Step 2:** FORCE 禁令不适用（Rust 无 force 策略，文档化）
- [x] **Step 3:** Commit

---

### Task 4.4：get_optional_template + StatefulTemplateLoader

**Files:**
- Create: `freemarker/src/core/get_optional_template_method.rs`
- Modify: `freemarker/src/cache/stateful_template_loader.rs`

- [x] **Step 1:** 实现 GetOptionalTemplateMethod（一文件一对象）
- [x] **Step 2:** 实现 TemplateLoader::reset_state 默认钩子
- [x] **Step 3:** Commit — `feat(align): Java <-> Rust 目录结构对齐 + 缺口补齐`

---

## Stage 5 — core 语义补全

### Task 5.1：Environment 三层 auto import/include 执行

**Files:**
- Modify: `freemarker/src/core/environment.rs`

- [x] **Step 1:** 实现 auto_import/include 三层执行（custom state 实现）
- [x] **Step 2:** Commit — `feat: Environment 三层 auto import/include 执行与 custom state 实现`

---

### Task 5.2：CombinedMarkupOutputFormat

**Files:**
- Create: `freemarker/src/core/combined_markup_output_format.rs`

- [x] **Step 1:** 实现组合输出格式（HTML+XML 转义联合）
- [x] **Step 2:** Commit — `feat: CombinedMarkupOutputFormat`

---

### Task 5.3：?absolute_template_name/.caller_template_name/markup

**Files:**
- Modify: `freemarker/src/core/environment.rs`

- [x] **Step 1:** 实现 ?absolute_template_name / .caller_template_name / markup 语义
- [x] **Step 2:** 实现 ?with_args 宏函数 / 设置解析剥引号
- [x] **Step 3:** Commit — `feat(core): ?absolute_template_name/.caller_template_name/markup 语义`

---

### Task 5.4：@@nodeName/XPath 默认命名空间/capture markup

**Files:**
- Modify: `freemarker/src/xml/node.rs`
- Modify: `freemarker/src/core/environment.rs`

- [x] **Step 1:** 实现 @@nodeName / XPath 默认命名空间 / capture markup 语义
- [x] **Step 2:** Commit — `feat(core): 补齐 @@nodeName/XPath 默认命名空间/capture markup 语义`

---

## Stage 6 — parser #on 指令 + ?eval_json 内建

### Task 6.1：#on 指令 + ?eval_json

**Files:**
- Modify: `freemarker/src/parser/grammar.rs`
- Modify: `freemarker/src/builtins/format.rs`

- [x] **Step 1:** 实现 Java 2.3.28+ 的 #on 指令
- [x] **Step 2:** 实现 ?eval_json 内建函数
- [x] **Step 3:** Commit — `feat(parser): 实现 Java 2.3.28+ 的 #on 指令和 ?eval_json 内建函数`

---

## 实际完成状态

- **日期**：2026-08-04 ~ 2026-08-05
- **文件数**：freemarker/src 从 90 .rs -> 291 .rs（2026-08-04 第二轮核对）
- **结构对照**：422 MAPPED / 4 MISSING（3 功能块）/ 115 NA-DESIGN
- **验收**：cargo test --workspace 全绿（1009 tests）；golden 113/128 MIRRORED

## 未完成项（P2 优先级）

| # | 功能块 | 理由 | 状态 |
|---|--------|------|------|
| 6 | 模板后处理钩子 | 嵌入扩展点缺失（安全/审计集成） | [ ] 待实施 |
| 7 | 组合输出格式 | 已实现 CombinedMarkupOutputFormat | [x] 已完成 |
| 9 | DOCTYPE 节点 | roxmltree 无 Doctype 节点变体 | [ ] 受 crate 限制 |
