# 发布流程（crates.io + 阻断点）

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
| **golden 15 项永久 NA（已定格 113/128）** | JVM 反射 12 项（beans 1 + BeansWrapper 方法重载 11，决策 1）+ transforms JythonRuntime 1 + jython25 过期断言 2——用户决策永久 NA，分类确定化（golden.rs permanent_na_reason） | 不解除；"受限子集"边界文档化（docs/测试/验收报告.md v13 + 生产就绪审计报告） |
| **JVM 反射（决策 1）** | `BeansWrapper`/`ClassIntrospector`/`MemberAccessPolicy` 在 Rust 不可 1:1 实现 | 用户在 Rust 侧手工包装 POJO 为 `DynValue`；文档 `docs/release/security.md` 明记 |
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
4. 立即修复并按 versioning.md §3 重新走晋级流程

## 7. 与治理工具链的对应

| 工具 | 本轮门禁状态 | 引用 |
|---|---|---|
| `deny.toml` + CI | 全绿 | `.github/workflows/ci.yml` |
| `.cargo/audit.toml` + CI | 0 error | `.github/workflows/ci.yml` |
| `cargo public-api` 基线 diff | 0 diff | `.github/workflows/ci.yml` |
| release workflow（dry-run） | 新增 | `.github/workflows/release.yml` |
| 多 OS CI（ubuntu/macos/windows + MSRV） | 阶段 A6 落地 | `.github/workflows/ci.yml` |
| proptest fuzz | 阶段 A8 落地 | `.github/workflows/ci.yml` |