# freemarker-rust 版本规划

> 本文件汇总当前版本快照、路线图与晋级门禁映射，供团队与 agentic workers
> 快速判断项目成熟度与下一步优先级。

---

## 1. 当前快照（Snapshot）

| 指标 | 当前值 | 备注 |
|------|--------|------|
| 当前版本 | `0.1.0-alpha.1` | tag 于 2026-08-03 |
| golden MIRRORED | **113/128**（88%） | 0 FAIL / 0 BLOCKED，15 项永久 NA |
| builtins 覆盖 | **183/183**（100%） | Java 2.3.34 全集 |
| 测试数 | **1009** passing | cargo test --workspace |
| .rs 文件数 | **291**（freemarker/src） | 从 P0 的 ~10 增长到 291 |
| 结构对照 | 422 MAPPED / 4 MISSING / 115 NA-DESIGN | 见 `docs/JavaRust结构对照.md` |
| 公共 API 基线 | 0 diff（locked） | `docs/release/api-baseline.txt` |
| proptest fuzz | 10000 cases | expression + parser target |
| criterion 基准 | 5 metrics | 见 `docs/release/benchmarks.md` |
| pyo3 发布就绪 | 元数据完整，未实际发布 | pyproject.toml + LICENSE 完备 |

### 1.0 晋级条件 8 项状态

| # | 条件 | 状态 | 说明 |
|---|------|------|------|
| 1 | cargo-deny / cargo-audit 全绿 | ✅ 已落地 | CI 门禁已集成 |
| 2 | cargo public-api 基线 diff = 0 | ✅ 已落地 | `api-baseline.txt` 锁定 |
| 3 | Clippy + fmt + workspace 全测试 + golden >=86 MIRRORED 全绿 | ✅ 已落地 | 113/128 MIRRORED |
| 4 | `cargo package --verify -p freemarker` 通过 | ✅ 已落地 | alpha.0 阶段演练通过 |
| 5 | 多 OS CI 矩阵全绿 | ✅ 已落地 | ubuntu/macos/windows x stable + MSRV 1.85 |
| 6 | proptest fuzz 10000 用例无 panic | ✅ 已落地 | expression + parser |
| 7 | criterion 基准集落档 | ✅ 已落地 | `docs/release/benchmarks.md` |
| 8 | 安全模型文档评审通过 + "受限子集"边界明记 | ✅ 已落地 | `docs/release/security.md` 语义复查完成 |

**结论**：8/8 条件均已满足（alpha 级别），但尚未经历 beta 阶段的稳定性验证与社区反馈。

---

## 2. 版本路线图（Roadmap）

### 2.1 `0.1.0-beta.0`（下一个里程碑）

**目标**：从 alpha 进入 beta——功能冻结、稳定性验证、文档收口。

**入口条件**：
- [ ] 4 项 MISSING 功能块补齐（模板后处理钩子、DOCTYPE 节点——受 roxmltree 限制，需评估替代方案）
- [ ] golden MIRRORED >= 115/128（当前 113，差 2 项可实施）
- [ ] `docs/superpowers/plans/` 全部历史计划审计通过
- [ ] `docs/superpowers/specs/` 12 个 spec 映射完整
- [ ] P2 优先级未完成项（见 P6 计划）评估并排期

**预计时间**：2026-08 中旬

### 2.2 `0.1.0`（稳定化版本）

**目标**：首个功能完整、文档齐备的 0.x 稳定版。

**入口条件**：
- [ ] beta.0 无 blocker issue
- [ ] 公共 API 面经至少 1 个 beta 版本无变更
- [ ] 用户文档（README + API docs + 迁移指南）完整
- [ ] pyo3 绑定实际发布到 PyPI（首个版本）
- [ ] crates.io 发布 `freemarker` crate

**预计时间**：2026-08 下旬 ~ 2026-09

### 2.3 `1.0.0`（首个稳定版）

**目标**：SemVer 承诺生效，公共 API 稳定。

**入口条件**：
- [ ] versioning.md §3.1 全部 8 项条件在 beta 阶段重新验证通过
- [ ] 至少 2 个 beta 版本间隔 >= 2 周
- [ ] 社区反馈无重大 API 设计缺陷
- [ ] 性能基准设硬阈值（criterion drift gate）

**预计时间**：2026-09 ~ 2026-10

---

## 3. 晋级门禁映射（Gate Mapping）

| # | 1.0 条件（versioning.md §3.1） | 状态 | 证据文件 | 负责计划 |
|---|------|------|----------|---------|
| 1 | cargo-deny / cargo-audit 全绿 | ✅ | CI workflow `.github/workflows/ci.yml` | `2026-08-03-alpha0-production-readiness.md` Stage A |
| 2 | cargo public-api 基线 diff = 0 | ✅ | `docs/release/api-baseline.txt` | `2026-08-03-alpha0-production-readiness.md` Stage A |
| 3 | Clippy/fmt/workspace 测试 + golden >=86 MIRRORED | ✅ | `freemarker-test/` harness + CI | `2026-08-03-alpha1-governance-hardening.md` |
| 4 | `cargo package --verify` 通过 | ✅ | CI publish dry-run | `2026-08-03-alpha0-production-readiness.md` Stage C |
| 5 | 多 OS CI 矩阵全绿 | ✅ | `.github/workflows/ci.yml` matrix | `2026-08-03-alpha0-production-readiness.md` Stage A |
| 6 | proptest fuzz 10000 用例无 panic | ✅ | `freemarker/fuzz/` + CI | `2026-08-03-alpha0-production-readiness.md` Stage C |
| 7 | criterion 基准集落档 | ✅ | `docs/release/benchmarks.md` | `2026-08-03-alpha0-production-readiness.md` Stage C |
| 8 | 安全模型文档评审通过 | ✅ | `docs/release/security.md` | `2026-08-03-alpha1-governance-hardening.md` |

---

## 4. Unreleased / 待办索引

### 4.1 P6 计划未完成项（P2 优先级）

| # | 功能块 | 理由 | 状态 |
|---|--------|------|------|
| 6 | 模板后处理钩子 | 嵌入扩展点缺失（安全/审计集成） | [ ] 待实施 |
| 7 | 组合输出格式 | 已实现 CombinedMarkupOutputFormat | [x] 已完成 |
| 9 | DOCTYPE 节点 | roxmltree 无 Doctype 节点变体 | [ ] 受 crate 限制 |

### 4.2 合规审计 Top 5 必修项

> 来源：`docs/合规审计报告.md`（2026-08-04，88% 合规）

| # | 必修项 | 优先级 | 状态 |
|---|--------|--------|------|
| 1 | 模板后处理钩子扩展点 | P2 | 待实施 |
| 2 | DOCTYPE 节点支持（roxmltree 限制） | P2 | 评估中 |
| 3 | 公共 API 面文档补全（rustdoc 0 warning） | P2 | 进行中 |
| 4 | pyo3 实际发布到 PyPI | P1 | 元数据就绪，待发布 |
| 5 | 多 OS CI 稳定性持续观察 | P3 | 已落地，观察中 |

### 4.3 结构对照剩余缺口

| 分类 | 数量 | 说明 |
|------|------|------|
| MAPPED | 422 | 已建立 Rust 镜像 |
| MISSING | 4 | 3 功能块 + 1 待评估 |
| NA-DESIGN | 115 | 设计阶段确定不迁移 |

---

## 5. 变更记录

| 日期 | 变更 |
|------|------|
| 2026-08-04 | 初始版本——基于 alpha.1 快照 + 合规审计 + 8 项门禁状态 |
