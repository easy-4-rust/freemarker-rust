# freemarker-rust 文档索引

> 所有设计规格文档已迁移至 `docs/superpowers/specs/` 作为唯一事实源。

## 设计规格

全部 25 个设计规格位于 `docs/superpowers/specs/`，详见
[`docs/superpowers/README.md`](superpowers/README.md) 的完整索引。

## 保留的测量数据与测试证据

以下文件由 CI 或脚本直接引用，保留在原位：

| 文件 | 用途 | 引用方 |
|------|------|--------|
| `release/api-baseline.txt` | 公共 API 基线 | `.github/workflows/ci.yml` |
| `release/benchmarks.md` | 性能基准测量数据 | 手动参考 |
| `release/v0.1.0-alpha.0-summary.md` | 历史发布记录 | 手动参考 |
| `测试/性能基准报告.md` | 性能基准测量数据 | `scripts/bench_compare.py` |
| `测试/compat_report.html` | 兼容性报告 | `scripts/gen_compat_report.py` |

> 注：原 `测试/` 下的 6 个验证/审计/账本规格已迁移至 `superpowers/specs/`（唯一事实源）。

## Superpowers 体系

- [`superpowers/README.md`](superpowers/README.md) — 体系约定与规格/计划索引
- [`superpowers/VERSION-PLAN.md`](superpowers/VERSION-PLAN.md) — 版本规划与晋级门禁
- [`superpowers/AUDIT-SUMMARY.md`](superpowers/AUDIT-SUMMARY.md) — 历史计划合规审计
- [`superpowers/plans/`](superpowers/plans/) — 12 个实施计划
- [`superpowers/specs/`](superpowers/specs/) — 23 个设计规格（唯一事实源）
