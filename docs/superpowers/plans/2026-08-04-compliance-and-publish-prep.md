# 合规审计与发布准备计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成 Rust 项目规范全仓合规审计、freemarker-pyo3 发布元数据修正、根 README 总览、coverage 审计报告。

**Architecture:** 治理层文档收口——合规审计报告（88% 合规）、freemarker-pyo3 Cargo.toml 元数据修正（pyproject.toml + LICENSE）、CI pyo3 发布认证双路径、根 README 中英双语总览。

**Tech Stack:**
- maturin（pyo3 打包）
- Trusted Publishing（PyPI 发布认证）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-pyo3-design.md`

---

## 全局约定

- **合规审计**：覆盖 cargo-deny/audit/public-api/Clippy/fmt/多 OS CI 全部门禁
- **pyo3 发布**：元数据完整，本轮不实际发布到 PyPI
- **覆盖率审计**：85.19% 行覆盖率，缺口分类与引擎缺口登记

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | 合规审计报告 | 2 |
| 2 | pyo3 发布元数据修正 + CI | 2 |
| 3 | 根 README + 覆盖率审计 | 2 |

---

## Stage 1 — 合规审计报告

### Task 1.1：合规审计报告编写

**Files:**
- Create: `docs/superpowers/specs/2026-08-04-compliance-audit-design.md`

- [x] **Step 1:** 编写 Rust 项目规范全仓合规审计报告（88% 合规）
- [x] **Step 2:** Commit — `docs(compliance): Rust 项目规范全仓合规审计报告（88% 合规）`

---

### Task 1.2：覆盖率审计报告

**Files:**
- Create: `docs/覆盖率审计报告.md`（后移至 `docs/` 下）

- [x] **Step 1:** 覆盖率审计报告（85.19% 行，缺口分类与引擎缺口登记）
- [x] **Step 2:** Commit — `docs: 覆盖率审计报告（85.19% 行，缺口分类与引擎缺口登记）`

---

## Stage 2 — pyo3 发布准备

### Task 2.1：freemarker-pyo3 元数据修正

**Files:**
- Modify: `freemarker-pyo3/Cargo.toml`

- [x] **Step 1:** freemarker crate 携带 README/LICENSE 进 .crate 包
- [x] **Step 2:** Commit — `fix(publish): freemarker crate 携带 README/LICENSE 进 .crate 包`

---

### Task 2.2：pyo3 CI 发布认证

**Files:**
- Modify: `.github/workflows/ci.yml`

- [x] **Step 1:** pyo3 CI 发布认证双路径——PYPI_API_TOKEN 优先 / Trusted Publishing 兜底
- [x] **Step 2:** Commit — `ci(pyo3): 发布认证双路径——PYPI_API_TOKEN 优先 / Trusted Publishing 兜底`

---

## Stage 3 — 根 README + 文档收口

### Task 3.1：根 README 总览

**Files:**
- Modify: `README.md`

- [x] **Step 1:** 中英双语 README + 完整代码示例——补全 crates.io 描述
- [x] **Step 2:** Commit — `docs(readme): 中英双语 README + 完整代码示例——补全 crates.io 描述`

---

### Task 3.2：交付总结

**Files:**
- Create: `docs/release/v0.1.0-alpha.0-summary.md`

- [x] **Step 1:** v0.1.0-alpha.0 交付总结（阶段 A-D 全部落档）
- [x] **Step 2:** Commit — `release: v0.1.0-alpha.0 交付总结`

---

## 实际完成状态

- **日期**：2026-08-03 ~ 2026-08-04
- **提交**：`9651d8a`、`1483947`、`2722544`、`f69d3b2`、`c49c2de`、`51a7d17`、`5711b3d`、`43c6722`
- **验收**：合规审计 88% 合规；pyo3 元数据完备；根 README 中英双语
- **状态**：全部 `- [x]` 完成
