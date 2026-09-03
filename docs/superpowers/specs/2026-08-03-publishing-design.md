# 发布流程设计

- **日期**：2026-08-03
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97，2.3 分支线）
- **依赖**：无外部依赖

---

> 本文件定义 freemarker-rust 从 `git tag vX.Y.Z` 到 crates.io / GitHub Release 的端到端
> 发布流程、阻断点与演练规范。

## 1. 发布对象

| crate | publish | 本轮策略 | 备注 |
|---|---|---|---|
| `freemarker` | true | **本轮演练 dry-run**（`cargo publish --dry-run`） | 核心引擎；语义基线 1.0 候选 |
| `freemarker-test` | false | 不发布 | 整体功能测试模块，含 harness + golden runner + java_ported；本地使用 |
| `freemarker-pyo3` | true | **本轮不发布**（元数据已完整登记备后续） | Python 绑定；PyPI 通路独立，需 maturin + PyPI 发布，不在 cargo publish 范围 |

## 2. 端到端流程

```
git tag v0.1.0-alpha.1
        │
        ▼
GitHub Actions: .github/workflows/release.yml 触发
        │
        ├─ Job: publish-core
        │   ├─ checkout + Rust toolchain
        │   ├─ cargo publish --dry-run -p freemarker  → 演练打 .crate + 自检
        │   ├─ awk 提取 CHANGELOG.md 对应版本条目
        │   └─ softprops/action-gh-release 创建 GitHub Release
        │
        ▼
人工确认 + 实际 cargo publish -p freemarker  → crates.io
        ▼
docs.rs 自动构建（无需额外 CI 步骤，docs.rs 监听 crates.io）
```

## 3. 阻断点登记

| 阻断 | 原因 | 解除路径 |
|---|---|---|
| **freemarker-pyo3 实际上传** | PyPI 通路独立，需 maturin 打 wheel + Trusted Publishing；`pyo3-publish.yml` 已就绪（tag `pyo3-v*` 触发，dry-run 演练通过） | 用户手动配置 PyPI 发布者后打 tag；本仓库演练到 TestPyPI |
| **golden 15 项永久 NA（已定格 113/128）** | JVM 反射 12 项（beans 1 + BeansWrapper 方法重载 11，决策 1）+ transforms JythonRuntime 1 + jython25 过期断言 2——用户决策永久 NA，分类确定化（golden.rs permanent_na_reason） | 不解除；"受限子集"边界文档化（docs/superpowers/specs/2026-08-03-acceptance-report-design.md v13 + 生产就绪审计报告） |
| **JVM 反射（决策 1）** | `BeansWrapper`/`ClassIntrospector`/`MemberAccessPolicy` 在 Rust 不可 1:1 实现 | 用户在 Rust 侧手工包装 POJO 为 `DynValue`；文档 specs/2026-08-03-security-model-design.md 明记 |
| ~~docs.rs metadata~~ | 已落地：`[package.metadata.docs.rs]`（all-features + rustdoc-args） | ✅ 已解除（阶段 A7） |
| ~~proptest fuzz~~ | 已落地：解析器 + 表达式 10000 用例（robustness_fuzz_smoke） | ✅ 已解除（阶段 C） |
| ~~criterion baseline~~ | 已落档 `docs/release/benchmarks.md`（性能基准报告 5/5 达标） | ✅ 已解除（阶段 C） |

## 4. 演练 Checklist（每个 release tag 前必跑）

```bash
# 本地演练
cargo publish --dry-run -p freemarker              # 打 .crate + 自检
cargo package --verify -p freemarker               # 验证 .crate 内容
cargo deny check                                    # 依赖治理全绿
cargo audit                                         # 漏洞扫描
cargo public-api -p freemarker | diff - docs/release/api-baseline.txt
cargo fmt --all --check                            # 格式
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --exclude freemarker-pyo3  # 全部测试
cargo llvm-cov --workspace --exclude freemarker-pyo3 --lcov  # 覆盖率快照
```

## 5. 实际发布（人工）

```bash
# 确认所有演练项绿后
git tag v0.1.0-alpha.1                    # 或后续版本
git push origin v0.1.0-alpha.1            # 触发 release.yml dry-run + GH Release
# 检查 GH Release 页面内容 → 人工 cargo login + cargo publish -p freemarker
cargo publish -p freemarker               # crates.io 上传
# docs.rs 在 1-2 分钟内自动重建
```

## 6. 安全回滚

如果发布后检测到严重问题：

1. `cargo yank --vers <version> -p freemarker`（crates.io 撤回该版本）
2. 删除 GitHub Release
3. 删除 git tag（本地 + 远程）
4. 立即修复并按 specs/2026-08-03-versioning-design.md §3 重新走晋级流程

## 7. 与治理工具链的对应

| 工具 | 本轮门禁状态 | 引用 |
|---|---|---|
| `deny.toml` + CI | 全绿 | `.github/workflows/ci.yml` |
| `.cargo/audit.toml` + CI | 0 error | `.github/workflows/ci.yml` |
| `cargo public-api` 基线 diff | 0 diff | `.github/workflows/ci.yml` |
| release workflow（dry-run） | 新增 | `.github/workflows/release.yml` |
| 多 OS CI（ubuntu/macos/windows + MSRV） | 阶段 A6 落地 | `.github/workflows/ci.yml` |
| proptest fuzz | 阶段 A8 落地 | `.github/workflows/ci.yml` |

---

## 对应计划

- `docs/superpowers/plans/2026-08-03-alpha0-production-readiness.md`（生产就绪）


---

## 2026-08-16 升级：release.yml 具备真实发布能力（wxrust 模板适配）

原 workflow 仅 dry-run + GitHub Release；现按 wxrust release 模板重写为完整
发布流水线：`validate-tag → build-and-test（test/clippy/fmt 全门禁）→
dry-run-publish → publish-crates（真实上传 crates.io）→ create-release`。

- 固定 toolchain 1.97.1 + `concurrency: release-{ref}`（cancel-in-progress: false）；
- token 前置守卫（未配置立即失败）+ 3 次重试（45s 退避）+ 幂等重跑
  （"already uploaded/exists" 视为成功，rerun 安全）；
- 预发布 tag（`vX.Y.Z-suffix`）通过 validate-tag 正则，GitHub Release 自动标
  prerelease；`pyo3-v*` 不触发本 workflow；
- create-release 依赖 publish 成功（发布失败不产生 Release），正文仍取
  CHANGELOG.md 对应版本段，找不到时回退链接；
- 前置条件：org 级 `CARGO_REGISTRY_TOKEN`（与 wxrust/easypdf 同一 secret）。
