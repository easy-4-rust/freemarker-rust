# 版本治理与发布门禁设计

- **日期**：2026-08-03
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97，2.3 分支线）
- **依赖**：无外部依赖

---

> 本文件定义 freemarker-rust 从 `0.1.0` 到稳定版的版本晋级规则、发布门禁与
> 语义化版本承诺。核心 crate `freemarker` 与 15 个整合 crate 共享 `[workspace.package]`
> 版本号（`Cargo.toml`），保持同批发布。

## 1. 当前版本与目标

- 当前：`0.1.0-alpha.1`（生产就绪计划 v2 收口；golden 113/128 MIRRORED（88%），15 项永久 NA 分类确定化）
- 阶段目标：维持 `0.1.x` 直至治理、鲁棒性与发布流程全部就绪，再按 §3 晋级
  `1.0.0`（首个稳定版），此后按 SemVer 承诺演化。
- 与 thymeleaf-rust 同口径：deny/audit/public-api 三类治理门禁齐备 + 多 OS CI + 发布演练。

## 2. SemVer 承诺（1.0 前）

`0.1.x` 阶段不承诺稳定 API；但以下红线始终有效：

- **破坏性变更必须记录在 CHANGELOG.md 的对应 Unreleased 条目**；
- **`freemarker` 核心 crate 的公开 API 面**（由 `cargo public-api` 基线锁定）不得在
  无评审的情况下扩大/收缩；
- 迁移语义（Java FreeMarker `2.3-gae` commit `7926e97`，improvements `2.3.34`）
  不因版本迭代而改变：128 套件用例差分、golden MIRRORED 数值、
  `migration-test` 账本为不可回归门禁。

## 3. 晋级规则

### 3.1 0.1.x → 1.0（首个稳定版）

满足全部条件后发起：

1. cargo-deny / cargo-audit 全绿（已落地）；
2. cargo public-api 基线 diff 门禁 0（已落地）；
3. 严格 Clippy、fmt、workspace 全测试、128 套件用例（≥86 MIRRORED，当前 **113/128**）全绿；
4. `cargo package --verify -p freemarker` 发布演练通过；
5. 多 OS CI 矩阵（ubuntu/macos/windows × stable + MSRV 1.85）全绿；
6. proptest fuzz（解析器 + 表达式）10000 用例无 panic；
7. criterion 基准集落档 `docs/release/benchmarks.md`；
8. 安全模型文档（specs/2026-08-03-security-model-design.md）评审通过 + "受限子集"边界明记。

### 3.2 晋级动作

- 更新 `Cargo.toml` 的 `[workspace.package] version`
- CHANGELOG.md 移动 Unreleased → 版本条目
- git tag：`v<version>`（annotated），GitHub Release 自动生成
- 重新生成 coverage/审计/语料台账快照并落档

## 4. 版本号纪律

- 版本号单一事实来源：`Cargo.toml` `[workspace.package].version`（三个成员 crate 已全部 `*.workspace = true` 继承，禁止手写版本号）；
- `rust-version = "1.85"` 是 MSRV 红线，任何新依赖若强制更高 MSRV，须在
  specs/2026-08-03-versioning-design.md 登记批准例外并单独标注该 crate 的 MSRV；
- 根 `Cargo.lock` 跟踪策略评估中（库 crate 惯例不跟踪，CI 使用 `--locked` 时以
  `xtask/Cargo.lock` 为准——本仓库无 xtask，暂不使用 `--locked`）。

## 5. 评审与责任

- 本文件变更需随版本晋级提交一并评审；
- 每个晋级条件对应一个可执行门禁（见 §3 引用），不允许"口头确认"。

---

## 对应计划

- `docs/superpowers/plans/2026-08-03-alpha0-production-readiness.md`（生产就绪）
