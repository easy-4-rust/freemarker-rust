# 生产就绪计划（0.1.0-beta.0）— 执行记录

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 将 freemarker-rust 从 alpha.1 推进到 0.1.0-beta.0——补齐 pyo3 API 面、拆分超限文件、soak 稳定性验证、证据化复核 golden 处置、发版收口。

**Architecture:** 在布局对齐轮（472 文件 1:1）基础上收口：Stage 0 CI 门禁确认 → Stage 1 golden NA 证据化复核 → Stage 2 pyo3 桥接（纯绑定层，核心不动）→ Stage 3 grammar.rs 按产生式族拆分（零行为变化）→ Stage 4 soak/proptest/criterion → Stage 5 版本收口。

**Tech Stack:** 无新增依赖。

**Related Design Doc:** `docs/superpowers/VERSION-PLAN.md`（§2.1 beta 入口条件）、`docs/superpowers/specs/2026-08-03-versioning-design.md`

---

## 全局约定

- 行为零容忍变更：拆分/桥接不改引擎语义（995+ 测试 + public-api diff 守护）
- 诚实纪律：golden 目标修订须附证据（Stage 1）；性能差异归因须登记（Stage 4）
- 每 Stage 一次 conventional commit，不 push（统一确认后推送）

---

## 实施阶段总览

| Stage | 目标 | Task 数 | 状态 |
|-------|------|---------|------|
| 0 | CI 基线验证 | 1 | ✅ |
| 1 | golden 15 项 NA 复核 | 3 | ✅ |
| 2 | pyo3 配置桥接 7→35 | 4 | ✅ |
| 3 | grammar.rs 拆分 ≤800 | 2 | ✅ |
| 4 | soak + proptest 50000 + criterion | 3 | ✅ |
| 5 | beta.0 发版收口 | 4 | ✅ |

---

## Stage 0 — CI 基线验证

### Task 0.1：main CI 全绿确认

- [x] run 31862641542（e52ef3a）：**12 job 全绿**——MSRV 1.85 / Test&Lint 三 OS /
      Coverage / pyo3 六矩阵 / **Governance gates（deny/audit/public-api 4m57s）**
- [x] api-baseline 6054 行过 CI 门禁（本地 nightly-2026-07-28 生成与 CI 一致）
- [x] pyo3-publish 0s 失败定性：PyPI 环境未配置的发布路径噪音（0.1.0 阶段处理）

## Stage 1 — golden 15 项 NA 证据化复核

### Task 1.1-1.3：逐项复核 + 修订入口条件

- [x] 复核 golden.rs:104-123 全部 15 项：反射系 12（beans + overloaded-methods×11，
      security.md 决策 1 永久不支持）+ transforms 1（JythonRuntime Java 类）+
      套件过期断言 2（string-builtins3/date-type-builtins，与真实 Java 2.3.34 矛盾，
      jar 实测在案）→ **0 项可实施**
- [x] 结论：113 + 15 = 128 全处置 = **适用范围 100%**；原「≥115 差 2 项可实施」系误估
- [x] VERSION-PLAN §2.1 入口条件证据化修订（5 项全部勾选附证据）

## Stage 2 — pyo3 配置桥接

### Task 2.1-2.2：35 方法 + 翻转

- [x] FmConfiguration 7 → **35 方法**：格式化 12（locale/time_zone/8 格式串/
      output_format/auto_escaping/c_format/url_escaping_charset/output_encoding）+
      解析期 4（strict_syntax/whitespace_stripping/classic_compatible/input_encoding）+
      行为 6（fallback_on_null_loop_variable/localized_lookup/
      template_exception_handler/lazy_imports/lazy_auto_imports/delay）+
      模板查找 3（auto_import/auto_include/lookup_strategy）+ getter 3
- [x] test_smoke.py +14 配置桥接测试（断言全部探针实测）
- [x] golden Python 翻转 3 用例：**import / localization / number-literal**（34→37）
- [x] harness 默认 strict_syntax=True 对齐 Rust golden（common/mod.rs:65）；
      non-strict-syntax/strictinheader 经 manifest 用例级设置恢复（主智能体修复：
      探针实测 strict=True 才逐字节通过——引擎 strict 模式兼容 jython25 旧式标签）
- [x] 验证：workspace 995 全绿 + pytest **81 passed / 72 skipped** + clippy 0

## Stage 3 — grammar.rs 拆分

### Task 3.1-3.2：13 文件 + 零变化守护

- [x] 6,837 行 → **13 文件全部 ≤800**：grammar.rs 716（聚合入口/Parser struct）+
      grammar_block 464 / expression 640 / expression_args 316 / directive 722 /
      assign 516 / control 408 / stripping 408 / element_helpers 477 / helpers 682 +
      三个测试文件 586/666/445
- [x] 组织方式：独立 `impl Parser` 块 + `#[path]` 聚合（core/expression.rs 既有模式）
- [x] 守护证据：995 全绿 + fmt/clippy 干净 + **public-api diff = 0**（主智能体补验）
- [x] 其余 12 个 >800 行文件登记 §8 债务表（1.0 前清偿）

## Stage 4 — 稳定性验证

### Task 4.1-4.3：soak / proptest / criterion

- [x] soak_smoke.rs：8 线程 × 2000 迭代 × 4 模板 = **64000 次并发渲染全成功**，
      首末输出逐字节一致，6.51s，120s 死锁守卫通过
- [x] 内存探针：5000 轮输出恒定，首/末 500 轮均值比 0.44（无退化）
- [x] proptest **50000 用例 0 panic**（14.12s，PROP_CASES 环境变量）
- [x] criterion 5 指标落档 benchmarks.md（3 项 >20% 差异归因机器更换
      Apple M4 Pro vs 未记录上轮环境，如实登记；硬阈值 drift gate 属 1.0）

## Stage 5 — beta.0 发版收口

### Task 5.1-5.4

- [x] 版本 0.1.0-alpha.1 → **0.1.0-beta.0**（workspace.package 单一事实源）
- [x] CHANGELOG.md beta.0 条目（本轮全部交付）
- [x] 本计划文件落档 + VERSION-PLAN 快照更新 + README 索引
- [x] `cargo publish --dry-run -p freemarker` 演练通过
- [x] git tag `v0.1.0-beta.0`（annotated）

---

## 实际完成状态

- **日期**：2026-08-15
- **提交**：`ab5c755`（S0+S1）→ `33f057e`（S2）→ `75571ee`（S3）→ `e7d87a7`（S4）→ 本提交（S5）
- **验收**：997 workspace 测试 + pytest 81 全绿；beta 入口 5 条件全部证据化勾选；
  versioning.md §3.1 八门禁复核表见 VERSION-PLAN §3

## 未完成项（0.1.0 / 1.0 前置，已登记）

| 项 | 阶段 | 登记位置 |
|---|------|---------|
| crates.io / PyPI 实际发布 | 0.1.0 | VERSION-PLAN §2.2 |
| 用户文档 API 冻结（≥1 个 beta 无变更） | 0.1.0 | VERSION-PLAN §2.2 |
| 其余 12 个 >800 行文件拆分 | 1.0 前 | 结构对照 spec §8 债务表 |
| criterion drift gate 硬阈值 | 1.0 | VERSION-PLAN §2.3 |
| 社区反馈期（≥2 beta 间隔 ≥2 周） | 1.0 | VERSION-PLAN §2.3 |
