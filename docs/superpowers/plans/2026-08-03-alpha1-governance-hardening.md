# v0.1.0-alpha.1 治理收口计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 生产就绪计划 v2 阶段 A/B/C 全部收口——内建 183/183 全覆盖、golden 113/128（88%）定格、15 项永久 NA 分类确定化、pyo3 一键可发布。

**Architecture:** 在 alpha.0 治理层基础上继续收口：补齐最后 5 个内建函数（eval_json/is_date_like/next_sibling/previous_sibling/web_safe）、XML visit 前缀宏分派、?api/?has_api 支持、?new 四解析策略、ICI 版本化、pyo3 发布准备。

**Tech Stack:**
- cargo-fuzz（expression/parser target）
- maturin（pyo3 打包）
- Trusted Publishing（PyPI 发布）

**Related Design Doc:** `docs/superpowers/specs/2026-08-02-builtins-design.md`、`docs/superpowers/specs/2026-08-01-testing-strategy-design.md`、`docs/superpowers/specs/2026-08-03-versioning-design.md`

---

## 全局约定

- **内建函数**：183/183 全覆盖（Java 2.3.34 全集）
- **golden 套件**：113/128 MIRRORED（88%），0 FAIL / 0 BLOCKED，15 项永久 NA
- **永久 NA 分类**：JVM 反射 12（beans + BeansWrapper 方法重载 11）+ transforms 1 + jython25 过期断言 2
- **pyo3 发布**：元数据完整，本轮不实际发布到 PyPI

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| B | 内建函数补齐 + ICI 版本化 + ?new 策略 + XML 扩展 + ?api 支持 | 6 |
| C | pyo3 发布准备 + cargo-fuzz + CHANGELOG | 3 |

---

## Stage B — 功能收口

### Task B1：内建函数补齐最后 5 个

**Files:**
- Modify: `freemarker/src/builtins/mod.rs`
- Modify: `freemarker/src/builtins/*.rs`

- [x] **Step 1:** 实现 `eval_json`（JSON 字符串解析为哈希/列表/标量）
- [x] **Step 2:** 实现 `is_date_like`（日期类型判断）
- [x] **Step 3:** 实现 `next_sibling` / `previous_sibling`（XML 节点导航）
- [x] **Step 4:** 实现 `web_safe`（HTML 转义别名，等价 `?html`）
- [x] **Step 5:** 验证 183/183 全注册
- [x] **Step 6:** Commit

---

### Task B2：ICI 版本化

**Files:**
- Modify: `freemarker/src/core/eval.rs`

- [x] **Step 1:** `?html` <2.3.20 用 HTMLEnc
- [x] **Step 2:** 哈希字面量 <2.3.21 保留重复键
- [x] **Step 3:** `?is_sequence` <2.3.24 / `?is_enumerable` <2.3.21 不排除方法模型
- [x] **Step 4:** Commit

---

### Task B3：?new 四解析策略

**Files:**
- Modify: `freemarker/src/builtins/callables.rs`

- [x] **Step 1:** 实现 unrestricted/safer/allows_nothing/opt-in + trusted_templates 四策略
- [x] **Step 2:** Commit

---

### Task B4：?api/?has_api 支持

**Files:**
- Create: `freemarker/src/template/template_model_with_api_support.rs`
- Modify: `freemarker/src/template/t_model.rs`（api 槽位）

- [x] **Step 1:** 新增 `TemplateApiSupport` trait + `TModel.api` 字段
- [x] **Step 2:** API 视图由包装方提供，引擎无反射
- [x] **Step 3:** Commit

---

### Task B5：XML visit 前缀宏分派

**Files:**
- Modify: `freemarker/src/xml/node.rs`

- [x] **Step 1:** 实现 Java getNodeProcessor 语义（visit 前缀宏分派）
- [x] **Step 2:** `node[0]` 自身索引 + XPath 子集 `./` 相对路径 + `true()` 函数
- [x] **Step 3:** Commit — `feat(xml): XML 节点支持 + 性能 5/5 达标`

---

### Task B6：golden 套件收口（87 -> 113 MIRRORED）

**Files:**
- Modify: `freemarker-test/tests/golden.rs`

- [x] **Step 1:** B6 harness 收口 + 错误对齐进展
- [x] **Step 2:** 15 项永久 NA 分类确定化（golden.rs permanent_na_reason）
- [x] **Step 3:** 0 FAIL / 0 BLOCKED
- [x] **Step 4:** Commit

---

## Stage C — 发布准备

### Task C1：pyo3 发布准备

**Files:**
- Modify: `freemarker-pyo3/pyproject.toml`（readme/authors/classifiers/license-files）
- Create: `freemarker-pyo3/LICENSE`（Apache-2.0）
- Create: `.github/workflows/pyo3-publish.yml`

- [x] **Step 1:** pyproject.toml 补全元数据
- [x] **Step 2:** LICENSE 文件
- [x] **Step 3:** pyo3-publish workflow（Trusted Publishing）
- [x] **Step 4:** Commit — `feat(pyo3): 发布准备`

---

### Task C2：cargo-fuzz 启用

**Files:**
- Modify: `fuzz/` 目录

- [x] **Step 1:** expression/parser target 声明（nightly 构建验证）
- [x] **Step 2:** Commit

---

### Task C3：CHANGELOG + tag

**Files:**
- Modify: `CHANGELOG.md`

- [x] **Step 1:** 编写 0.1.0-alpha.1 条目
- [x] **Step 2:** git tag v0.1.0-alpha.1
- [x] **Step 3:** Commit — `docs(changelog): 0.1.0-alpha.1`

---

## 验收结果

| 维度 | 数值 |
|------|------|
| 内建函数 | 183/183 全覆盖 |
| golden MIRRORED | 113/128（88%） |
| NOT_APPLICABLE | 15 项永久 NA |
| BLOCKED | 0 |
| FAIL | 0 |

## 实际完成状态

- **日期**：2026-08-03
- **Git tag**：v0.1.0-alpha.1
- **验收**：全部通过（详见 CHANGELOG.md 0.1.0-alpha.1 条目）
