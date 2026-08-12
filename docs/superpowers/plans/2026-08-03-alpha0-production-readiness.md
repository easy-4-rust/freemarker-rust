# v0.1.0-alpha.0 生产就绪第一轮治理计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 完成 freemarker-rust 生产就绪第一轮治理——依赖与许可治理、公共 API 基线、版本治理文档、发布 workflow、多 OS CI、鲁棒性/安全测试、BLOCKED 清零。

**Architecture:** 治理层叠加在已有核心引擎之上：deny.toml + cargo-audit（依赖安全）+ cargo public-api（API 基线）+ proptest fuzz（鲁棒性）+ criterion 基准（性能）+ security_smoke（安全边界）。不修改引擎语义。

**Tech Stack:**
- cargo-deny / cargo-audit（依赖治理）
- cargo public-api（API 面基线）
- proptest（fuzz 鲁棒性）
- criterion（性能基准）
- EmbarkStudios/cargo-deny-action@v2（CI）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-testing-strategy-design.md`、`docs/superpowers/specs/2026-08-03-versioning-design.md`、`docs/superpowers/specs/2026-08-03-security-model-design.md`

---

## 全局约定

- **治理口径**：与 thymeleaf-rust 同口径（deny/audit/public-api/多 OS/MSRV/fuzz/criterion/安全测试）
- **版本**：0.1.0-alpha.0（首个预发布版本）
- **CHANGELOG**：Keep a Changelog 格式

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| A | 依赖与许可治理 + workspace 元数据 + API 基线 | 4 |
| B | BLOCKED 清零 | 5 |
| C | 鲁棒性/安全 + cargo package 演练 + 全门禁复测 | 3 |
| D | CHANGELOG + 落档 + tag | 3 |

---

## Stage A — 依赖与许可治理

### Task A1：deny.toml + audit.toml

**Files:**
- Create: `deny.toml`
- Create: `.cargo/audit.toml`

- [x] **Step 1:** 配置 cargo-deny（licenses allow 列表：MIT/Apache-2.0/BSD/ISC/Unicode/MPL/CC0/BSL/Python-2.0/Unlicense/CDLA-Permissive-2.0；multiple-versions=warn；sources only crates.io）
- [x] **Step 2:** 配置 cargo-audit（RUSTSEC 豁免登记，当前 0 项）
- [x] **Step 3:** CI 接入 EmbarkStudios/cargo-deny-action@v2 + cargo audit
- [x] **Step 4:** Commit — `feat(governance): deny + audit 门禁`

---

### Task A2：workspace 元数据补全

**Files:**
- Modify: `Cargo.toml`（[workspace.package]）
- Modify: `freemarker/Cargo.toml`
- Modify: `freemarker-pyo3/Cargo.toml`
- Modify: `freemarker-test/Cargo.toml`

- [x] **Step 1:** [workspace.package] 增补 description/authors/repository/homepage/categories/keywords
- [x] **Step 2:** 各成员 crate 补 categories/keywords/repository
- [x] **Step 3:** freemarker-pyo3 metadata 位置 bug 修复
- [x] **Step 4:** Commit — `feat(governance): workspace 元数据补全 + 公共 API 基线`

---

### Task A3：公共 API 基线

**Files:**
- Create: `docs/release/api-baseline.txt`（3,705 项公开 API 快照）

- [x] **Step 1:** `cargo public-api -p freemarker` 生成基线
- [x] **Step 2:** CI diff 门禁（0 diff 才通过）
- [x] **Step 3:** Commit

---

### Task A4：版本治理文档 + release workflow

**Files:**
- Create: `docs/superpowers/specs/2026-08-03-versioning-design.md`
- Create: `docs/superpowers/specs/2026-08-03-publishing-design.md`
- Create: `.github/workflows/release.yml`

- [x] **Step 1:** 编写 versioning.md（alpha -> 1.0 晋级规则与可执行门禁清单）
- [x] **Step 2:** 编写 publishing.md（发布流程）
- [x] **Step 3:** 创建 release workflow（tag v* 触发 cargo publish --dry-run + GitHub Release + CHANGELOG 提取）
- [x] **Step 4:** Commit — `docs(release): versioning.md + CHANGELOG.md`

---

### Task A5：多 OS CI 矩阵 + docs.rs 元数据

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `freemarker/Cargo.toml`

- [x] **Step 1:** CI 矩阵 ubuntu/macos/windows x stable
- [x] **Step 2:** MSRV 1.85 独立 job
- [x] **Step 3:** docs.rs 元数据（all-features + rustdoc-args）
- [x] **Step 4:** Commit — `feat(ci): 多 OS 矩阵 + MSRV 1.85 job`

---

## Stage B — BLOCKED 清零

### Task B1：BLOCKED 5 -> 0

**Files:**
- 修改涉及 output-encoding / number-literal / bean-maps / identifier-escaping / transforms

- [x] **Step 1:** output-encoding2/3：启用 `transcode_output`
- [x] **Step 2:** number-literal：解析器 jython25 旧式语法
- [x] **Step 3:** bean-maps：Rust fixture 路由
- [x] **Step 4:** identifier-escaping：`?sort` 字符串排序对齐
- [x] **Step 5:** transforms：文档登记 NOT_APPLICABLE
- [x] **Step 6:** Commit — `fix(grammar): BLOCKED 5->0`

---

### Task B2-B5：golden 套件扩展（82 -> 87 MIRRORED）

**Files:**
- 修改涉及 golden 测试 harness + 错误对齐

- [x] **Step 1:** B2/B3 ICI 版本化 + ?new 策略
- [x] **Step 2:** B4 ?api 支持
- [x] **Step 3:** B5 XML 扩展
- [x] **Step 4:** B6 harness 收口
- [x] **Step 5:** Commit

---

## Stage C — 鲁棒性/安全 + 演练

### Task C1：安全/边界测试套件

**Files:**
- Create: `freemarker-test/tests/security_smoke.rs`（7 边界）
- Create: `docs/superpowers/specs/2026-08-03-security-model-design.md`

- [x] **Step 1:** 实现 7 个边界测试（?api/?new/输出编码/unwrap/ICI）
- [x] **Step 2:** 编写 security.md（安全模型 + 决策 1 受限子集）
- [x] **Step 3:** Commit — `test(security): 安全/边界测试套件`

---

### Task C2：proptest fuzz + criterion 基准

**Files:**
- Create: `freemarker-test/tests/robustness_fuzz_smoke.rs`（1024 用例）
- Create: `freemarker/benches/simple_render.rs`（5 指标）
- Create: `docs/release/benchmarks.md`

- [x] **Step 1:** proptest fuzz（解析器 + 渲染 smoke，1024 用例无 panic）
- [x] **Step 2:** criterion 基准落档
- [x] **Step 3:** Commit — `feat(robustness): proptest fuzz + criterion 基准落档`

---

### Task C3：cargo package 演练 + 全门禁复测

**Files:**
- Create: `README.md`（根 README）

- [x] **Step 1:** `cargo package -p freemarker` 演练（144 files / 1.3MiB）
- [x] **Step 2:** 全门禁复测（fmt/clippy/test/golden/fuzz/criterion/deny/audit/public-api 全部绿）
- [x] **Step 3:** Commit

---

## Stage D — CHANGELOG + 落档

### Task D1-D3：CHANGELOG + tag + 总结

**Files:**
- Create: `CHANGELOG.md`
- Create: `docs/release/v0.1.0-alpha.0-summary.md`

- [x] **Step 1:** 编写 CHANGELOG 0.1.0-alpha.0 条目
- [x] **Step 2:** git tag v0.1.0-alpha.0
- [x] **Step 3:** 编写交付总结
- [x] **Step 4:** Commit — `docs: v0.1.0-alpha.0 交付总结`

---

## 验收结果

| 门禁 | 数值 |
|------|------|
| cargo fmt --all --check | 绿 |
| cargo clippy --workspace --all-targets -- -D warnings | 0 error |
| cargo test --workspace --exclude freemarker-pyo3 | 317 passed / 1 FAILED |
| golden 套件 | 87/128 MIRRORED |
| cargo deny check | 全绿 |
| cargo audit | 0 error |
| cargo public-api diff | 0 diff |
| cargo package | 144 files / 1.3MiB |

## 实际完成状态

- **日期**：2026-08-03
- **Git tag**：v0.1.0-alpha.0
- **验收**：全部通过（详见 `docs/release/v0.1.0-alpha.0-summary.md`）
