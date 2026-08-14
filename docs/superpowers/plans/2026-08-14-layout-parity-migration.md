# 布局对齐轮——目录重排 + 镜像补齐 + 收尾

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成 freemarker-rust 源码目录与 Java freemarker-core 包结构的最终对齐——4 阶段：目录重排（Agent A）、镜像补齐（Agent B1/B2）、真实现（Agent C）、收尾审计（Agent D）。

**Architecture:** 在 P6 polish-alignment + builtins-coverage-rounds 基础上，执行 4 类目录重排（xml→ext/dom、error→core、builtins→core 重命名、expression 平铺）、182 个镜像文件补齐（锚点 ~140 + 语义 ~35 + 真实现 4 + 跳过 8）、PostProcessor/DOCTYPE 真实现、测试盘点 + 覆盖率复核 + public-api 基线重生成。

**Tech Stack:**
- 无新增依赖（纯结构重构 + 少量真实现）

**Related Design Doc:** `docs/superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md`、`docs/superpowers/specs/2026-08-04-coverage-audit-design.md`

---

## 全局约定

- **一文件一对象**：每个 Java 类在 Rust 中建立独立 .rs 文件
- **聚合 API 保留**：ExprKind/ElementKind/TemplateError/OutputFormatKind 枚举不拆
- **缩写感知命名**：jsonc/java_script/xsc/x_path 等按 Clippy snake_case 缩写规则修正
- **前导下划线例外**：Java `_Xxx` 内部类忠实映射，审计脚本 non-snake 启发式误报

---

## 实施阶段总览

| Stage | 目标 | Agent | Task 数 |
|-------|------|-------|---------|
| 1 | 目录重排（xml→ext/dom、error→core、builtins→core、expression 平铺） | A | 4 |
| 2 | 镜像补齐（182 项：core 88 + 格式/输出 ~80 + builtins 15 + ext.dom 8 + xml 11） | B1/B2 | 2 |
| 3 | 真实现（PostProcessor + DOCTYPE） | C | 2 |
| 4 | 收尾审计（覆盖率 + api-baseline + spec 回填） | D | 4 |

---

## Stage 1 — 目录重排（Agent A）

### Task 1.1：xml → ext/dom 归位（freemarker.ext.dom 包对齐）

**Files:**
- Move: `freemarker/src/xml/` 下 5 个已有文件 → `freemarker/src/ext/dom/`
- Create: `freemarker/src/ext/dom/` 下 8 个新增镜像文件

- [x] **Step 1:** xml→ext/dom 归位（5 文件迁移 + 8 镜像新增） — `8560ac6`

### Task 1.2：error → core 归位（freemarker.core 包对齐）

**Files:**
- Move: `freemarker/src/error/` 下 19 个异常镜像文件 → `freemarker/src/core/`

- [x] **Step 1:** error→core 归位（19 文件迁移） — `94f85d9`

### Task 1.3：builtins BuiltInsFor* → core 归位 + snake_case 重命名

**Files:**
- Move/Rename: `freemarker/src/builtins/` 下 BuiltInsFor* 文件按 Java snake_case 全名重命名

- [x] **Step 1:** builtins→core 归位 + snake_case 重命名 — `ef5730b`
- [x] **Step 2:** 4 个 range_model 命名尾巴对齐 + range 模块链修复 — `ebcbca8`

### Task 1.4：core/expression 平铺归位

**Files:**
- Move: `freemarker/src/core/expression/` 下 27 个表达式类平铺

- [x] **Step 1:** core/expression 平铺归位 — `9036c05`

---

## Stage 2 — 镜像补齐（Agent B1/B2）

### Task 2.1：core 88 个 Java 对象镜像文件

**Files:**
- Create: `freemarker/src/core/` 下 88 个 .rs 文件（异常/AST/内部工具，一一对应）

- [x] **Step 1:** 88 个 core 镜像文件 — `7ab0735`

### Task 2.2：格式化/CFormat/BuiltIn 基类/惰性集合/输出模型镜像（~80 项）

**Files:**
- Create: `freemarker/src/core/` 下 ~80 个 .rs 文件（格式化 + 输出模型 + BuiltIn 基类 + 惰性集合）

- [x] **Step 1:** ~80 项镜像文件 — `7ea2ae3`

---

## Stage 3 — 真实现（Agent C）

### Task 3.1：TemplatePostProcessor 完整实现

**Files:**
- Create: `freemarker/src/core/template_post_processor.rs`
- Create: `freemarker/src/core/template_post_processor_exception.rs`
- Create: `freemarker/src/core/thread_interruption_support_template_post_processor.rs`

- [x] **Step 1:** trait + 注册表 + Configuration 集成 — `7416048`

### Task 3.2：DOCTYPE 降级真实现

**Files:**
- Create: `freemarker/src/ext/dom/document_type_model.rs`

- [x] **Step 1:** 自扫声明 + DocumentTypeModel 语义对齐 — `e57dcfd`

---

## Stage 4 — 收尾审计（Agent D）

### Task 4.1：cargo-llvm-cov 覆盖率复核

- [x] **Step 1:** 运行 cargo-llvm-cov，记录原始值 85.02% + 排除锚点对照值 84.96%
- [x] **Step 2:** 追加到 `docs/superpowers/specs/2026-08-04-coverage-audit-design.md`

### Task 4.2：public-api 基线重生成

- [x] **Step 1:** nightly-2026-07-28 + cargo-public-api 重生成基线（6054 行）
- [x] **Step 2:** 更新 `docs/release/api-baseline.txt`

### Task 4.3：superpowers spec 回填

- [x] **Step 3a:** 更新 `docs/superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md`（§1 目录表 + §2 统计表 + §3 MISSING 清单 + §4b NA_FINAL + §8 新章节）
- [x] **Step 3b:** 更新 `docs/superpowers/AUDIT-SUMMARY.md`（§4 未完成项 + §5 结论 + §7 布局合规复审）
- [x] **Step 3c:** 新增 `docs/superpowers/plans/2026-08-14-layout-parity-migration.md`（本文件）

### Task 4.4：最终验证 + 提交

- [x] **Step 1:** cargo fmt + clippy + test 全量验证
- [x] **Step 2:** 提交 docs 变更

---

## 相关 commits

```
8560ac6 refactor(layout): xml→ext/dom 归位（freemarker.ext.dom 包对齐）
94f85d9 refactor(layout): error 异常镜像→core 归位（freemarker.core 包对齐）
9036c05 refactor(layout): core/expression 平铺归位
ef5730b refactor(layout): builtins BuiltInsFor*→core 归位并按 Java snake_case 全名重命名
2fa1551 fix: clippy 修复与外部可见性调整
7ab0735 feat(core): 补齐 88 个 core Java 对象镜像文件（异常/AST/内部工具，一一对应）
7416048 feat(core): TemplatePostProcessor 完整实现（trait+注册表+Configuration 集成）
7ea2ae3 feat(core): 补齐格式化/CFormat/BuiltIn基类/惰性集合/输出模型镜像（~80 项一一对应）
e57dcfd feat(ext-dom): DOCTYPE 降级真实现（自扫声明 + DocumentTypeModel 语义对齐）
47c4c03 refactor(layout): 缩写感知 snake_case 命名对齐（jsonc/java_script/xsc/x_path）+ 补锚点
ebcbca8 refactor(layout): 4 个 built_in/range_model 命名尾巴对齐 + range 模块链修复
22052bc test(java-ported): 补齐 Java core 测试缺口盘点（SOURCE_PARITY 补充轮）
```
