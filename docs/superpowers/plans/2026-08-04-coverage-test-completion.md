# 覆盖率补齐 + 安全文档更新计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成测试覆盖率补齐（ToCanonical/TemplateSource/LoggingAttemptExceptionReporter 单元测试）、XML 节点模型深度覆盖、对照文档统计更新、security.md 语义复查。

**Architecture:** 在已有测试基础上补全单元测试覆盖：ToCanonical 转换测试、TemplateSource 缓存键测试、LoggingAttemptExceptionReporter 日志测试。XML 节点模型 VALUE_ADD 测试（11 个）。对照文档从 310→422 MAPPED 更新。security.md 新增语义复查节。

**Tech Stack:**
- 无新增依赖（纯测试补全 + 文档更新）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-testing-strategy-design.md`

---

## 全局约定

- **覆盖率目标**：核心模块 >= 85%（cargo llvm-cov）
- **测试命名**：`#[test] fn <module>_<scenario>_<expected>()`
- **文档同步**：测试补全后同步更新对照文档统计

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | XML 节点模型深度覆盖 | 1 |
| 2 | 单元测试覆盖率补齐 | 1 |
| 3 | 对照文档 + security.md 更新 | 1 |

---

## Stage 1 — XML 节点模型深度覆盖

### Task 1.1：XML VALUE_ADD 测试（11 个）

**Files:**
- Modify: `freemarker-test/java-tests/` 相关 XML 测试

- [x] **Step 1:** XML 节点模型 VALUE_ADD 测试——11 个测试覆盖 XML 节点访问、属性读取、XPath 查询
- [x] **Step 2:** Commit — `f4de1bf test: XML 节点模型深度覆盖（VALUE_ADD，11 测试）`

---

## Stage 2 — 单元测试覆盖率补齐

### Task 2.1：ToCanonical / TemplateSource / LoggingAttemptExceptionReporter 测试

**Files:**
- Create/Modify: `freemarker/src/` 下相关测试模块

- [x] **Step 1:** ToCanonical 转换测试——模板规范化输出验证
- [x] **Step 2:** TemplateSource 缓存键测试——等价性/hash 一致性
- [x] **Step 3:** LoggingAttemptExceptionReporter 日志测试——错误报告器行为验证
- [x] **Step 4:** 测试文件 clippy 修复
- [x] **Step 5:** Commit — `1c63c30 test: 覆盖率补齐（ToCanonical/TemplateSource/LoggingAttemptExceptionReporter 单元测试）+ 测试文件 clippy 修复`

---

## Stage 3 — 文档更新

### Task 3.1：对照文档统计更新

**Files:**
- Modify: `docs/superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md`

- [x] **Step 1:** 对照文档统计更新（310→422 MAPPED，新增 §5b 拆分执行记录）
- [x] **Step 2:** Commit — `754ea26 docs: 对照文档统计更新（310→422 MAPPED，新增 §5b 拆分执行记录）`

---

### Task 3.2：security.md 语义复查 + 覆盖率报告终值

**Files:**
- Modify: `docs/superpowers/specs/2026-08-03-security-model-design.md`

- [x] **Step 1:** security.md 新增语义复查节（1.0 晋级条件 8）
- [x] **Step 2:** 覆盖率报告终值更新
- [x] **Step 3:** Commit — `ddddce1 docs: security.md 新增语义复查 + 覆盖率报告终值更新（1.0 晋级条件 8）`

---

## 实际完成状态

- **日期**：2026-08-04
- **提交**：`f4de1bf`、`1c63c30`、`754ea26`、`ddddce1`
- **测试新增**：XML 11 + ToCanonical/TemplateSource/LoggingAttemptExceptionReporter 若干
- **文档更新**：对照文档 310→422 MAPPED；security.md 语义复查完成
- **验收**：cargo test --workspace 全绿；1.0 晋级条件 8 全部满足
- **状态**：全部 `- [x]` 完成
