# P5 pyo3 集成计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成 freemarker-pyo3 Python 绑定——exec 签名同步（TemplateMethodModelEx 3 参）、pyo3 身份测试修正、workspace 全绿 1009 tests。

**Architecture:** freemarker-pyo3 crate 通过 PyO3 0.29 提供 Python 绑定：`FmConfiguration`（pyclass 包装 `Arc<Configuration>`）+ `FmTemplate`（pyclass 包装 `Arc<Template>`）+ `PyObjectWrapper`（JythonWrapper 等价，getattr/get_item 双通道）+ `TemplateModelAdapter`（反向适配：Python -> TModel）。

**Tech Stack:**
- PyO3 0.29.0
- maturin（打包）
- pytest（Python 测试）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-pyo3-design.md`

---

## 全局约定

- **GIL 策略**：单次持有 + allow_threads 分段
- **异常桥接**：PyErr <-> TemplateError + 自定义 FreeMarkerError
- **类型矩阵**：wrap（10 文档 S2 表）+ unwrap + TemplateModelAdapter

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | exec 签名同步（TemplateMethodModelEx 3 参） | 1 |
| 2 | pyo3 身份测试修正 | 1 |
| 3 | workspace 全绿验证 | 1 |

---

## Stage 1 — exec 签名同步

### Task 1.1：TemplateMethodModelEx::exec 加 env 参数

**Files:**
- Modify: `freemarker/src/template/template_method_model_ex.rs`
- Modify: `freemarker/src/core/eval.rs`（40 处 impl + 测试机械更新）

- [x] **Step 1:** `TemplateMethodModelEx::exec` 签名从 `exec(args: Vec<TModel>)` 改为 `exec(args: Vec<TModel>, env: &Environment)`
- [x] **Step 2:** Java 线程局部 -> Rust 显式传参（`Environment.getCurrentEnvironment()` 等价物）
- [x] **Step 3:** 40 处 impl + 测试机械更新
- [x] **Step 4:** Commit — `fix(pyo3): exec 签名同步（TemplateMethodModelEx 3 参）`

---

## Stage 2 — 身份测试修正

### Task 2.1：pyo3 身份测试

**Files:**
- Modify: `freemarker-pyo3/tests/`

- [x] **Step 1:** 修正 pyo3 身份测试（Python 对象身份与 Rust TModel 的映射验证）
- [x] **Step 2:** Commit

---

## Stage 3 — 验证

### Task 3.1：workspace 全绿验证

**Files:**
- 运行全量测试

- [x] **Step 1:** `cargo test --workspace` 全绿（1009 tests）
- [x] **Step 2:** Commit

---

## 实际完成状态

- **日期**：2026-08-04
- **Git 提交**：`fix(pyo3): exec 签名同步（TemplateMethodModelEx 3 参）——workspace 全绿 1009 tests`
- **验收**：1009 tests 全部通过
