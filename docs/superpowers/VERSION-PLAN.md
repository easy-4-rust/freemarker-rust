# freemarker-rust 版本规划

> 本文件汇总当前版本快照、路线图与晋级门禁映射，供团队与 agentic workers
> 快速判断项目成熟度与下一步优先级。

---

## 1. 当前快照（Snapshot）

| 指标 | 当前值 | 备注 |
|------|--------|------|
| 当前版本 | `0.1.0` | 2026-09-03（0.x 收官；三渠道发布齐备） |
| golden MIRRORED | **113/128**（88%） | 0 FAIL / 0 BLOCKED，15 项永久 NA |
| builtins 覆盖 | **183/183**（100%） | Java 2.3.34 全集 |
| 测试数 | **997** workspace + pytest 81 | cargo test / pytest（2026-08-15 实测） |
| .rs 文件数 | **472**（freemarker/src） | 布局对齐轮 291→472（412 MAPPED / 0 MISSING） |
| 结构对照 | 422 MAPPED / 4 MISSING / 115 NA-DESIGN | 见 `docs/superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md` |
| 公共 API 基线 | 0 diff（locked） | `docs/release/api-baseline.txt` |
| proptest fuzz | 10000 cases | expression + parser target |
| criterion 基准 | 5 metrics | 见 `docs/release/benchmarks.md` |
| pyo3 API 面 | **35 方法**（原 7） | PyPI 已上线 0.1.0b0（5 平台 abi3 wheel） |
| crates.io | freemarker **0.1.0** | 2026-09-03 发布（release.yml 流水线） |

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
| 8 | 安全模型文档评审通过 + "受限子集"边界明记 | ✅ 已落地 | `docs/superpowers/specs/2026-08-03-security-model-design.md` 语义复查完成 |

**结论**：8/8 条件均已满足（alpha 级别），但尚未经历 beta 阶段的稳定性验证与社区反馈。

---

## 2. 版本路线图（Roadmap）

### 2.1 `0.1.0-beta.0`（下一个里程碑）

**目标**：从 alpha 进入 beta——功能冻结、稳定性验证、文档收口。

**入口条件**：
- [x] 4 项 MISSING 功能块补齐（✅ 2026-08-15 布局对齐轮：TemplatePostProcessor 三件套 + DOCTYPE 降级实现 + CombinedMarkupOutputFormat；结构对照 412 MAPPED / 0 MISSING）
- [x] golden 适用范围 100%（✅ 2026-08-15 复核：113 MIRRORED + 15 有据永久 NA = 128 全处置，0 FAIL / 0 BLOCKED。原「≥115，差 2 项可实施」系误估——15 项 NA 逐项复核无一可实施：反射系 12 项引擎永久不支持（security.md 决策 1）、transforms 1 项依赖 JythonRuntime、2 项套件 expected 与真实 Java 2.3.34 矛盾（jar 实测在案，golden.rs:104-108），实施即错判或须改源 fixture（迁移红线禁止））
- [x] `docs/superpowers/plans/` 全部历史计划审计通过（✅ AUDIT-SUMMARY §4）
- [x] `docs/superpowers/specs/` spec 映射完整（✅ 12 → 23 个，2026-08-14 全量迁移）
- [x] P2 优先级未完成项评估并排期（✅ 本计划 Stage 1-5 即排期）

**预计时间**：2026-08 中旬

### 2.2 `0.1.0`（稳定化版本）

**目标**：首个功能完整、文档齐备的 0.x 稳定版。

**入口条件**：
- [x] beta.0 无 blocker issue（✅ 2026-08-15 发版；后续发现的 3 项边缘缺陷均已当日修复：非 dict 根统一拒绝 086cca5、currency/percent 预定义格式 9a10174、PyPI 发布链路 4 项修复）
- [x] 公共 API 面经至少 1 个 beta 版本无变更（✅ 冻结窗口 2026-08-15 起，api-baseline CI 门禁守护，beta.0→今 diff=0）
- [x] 用户文档（README + API docs + 迁移指南）完整（✅ 2026-08-16：docs/user-guide.md 508 行差异矩阵 + docs/api-stability.md + examples/×7 可运行示例，c71af40）
- [x] pyo3 绑定实际发布到 PyPI（✅ 2026-08-16 提前完成：freemarker-pyo3 0.1.0b0 上线，5 平台 abi3 wheel + sdist，端到端验证）
- [x] crates.io 发布 `freemarker` crate（✅ 2026-09-03：`freemarker 0.1.0-beta.0` 经 release.yml 新流水线上线——wxrust 模板适配的五段流水线（validate-tag → 全门禁 → dry-run → publish → create-release）5/5 job 成功，[crates.io](https://crates.io/crates/freemarker) max_version=0.1.0-beta.0 实测验证。发布矩阵至此完整：crates.io + PyPI + GitHub Release 三渠道齐备）

> **0.1.0 入口条件 5/5 全部达成（2026-09-03）**。后续按 §2.3 推进 1.0：beta 间隔期、
> 12 个 >800 行文件拆分（结构对照 spec §8 债务表）、drift gate 硬阈值、社区反馈。

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
| 8 | 安全模型文档评审通过 | ✅ | `docs/superpowers/specs/2026-08-03-security-model-design.md` | `2026-08-03-alpha1-governance-hardening.md` |

---

## 4. Unreleased / 待办索引

### 4.1 P6 计划未完成项（P2 优先级）

| # | 功能块 | 理由 | 状态 |
|---|--------|------|------|
| 6 | 模板后处理钩子 | 嵌入扩展点缺失（安全/审计集成） | [ ] 待实施 |
| 7 | 组合输出格式 | 已实现 CombinedMarkupOutputFormat | [x] 已完成 |
| 9 | DOCTYPE 节点 | roxmltree 无 Doctype 节点变体 | [ ] 受 crate 限制 |

### 4.2 合规审计 Top 5 必修项

> 来源：`docs/superpowers/specs/2026-08-04-compliance-audit-design.md`（2026-08-04，88% 合规）

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
