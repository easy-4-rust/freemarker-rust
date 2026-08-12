# 迁移路线图设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34
- **依赖**：全部设计文档（01-11）

## 1. 目标与范围

定义 freemarker-rust 的迁移路线图：P0 骨架 -> P1 解析 -> P2 渲染核 -> P3 表达式+内建 -> P4 配置/缓存/格式 -> P5 pyo3 -> P6 打磨。每阶段含任务清单（WBS）、验收标准、工作量估算。

详细设计见：`docs/12-迁移路线图.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 迁移路线图 | `docs/12-迁移路线图.md` | P0-P6 七阶段划分、每阶段 WBS 任务清单、验收标准（引用对应文档）、工作量估算（24-34 人日）、里程碑（M1-M5）、风险登记 |

## 3. 关键设计决策

- **阶段划分**：P0 骨架 -> P1 解析 -> P2 渲染核 -> P3 表达式+内建 -> P4 配置/缓存/格式 -> P5 pyo3 -> P6 打磨
- **依赖关系**：P0 -> P1 -> P2 -> P3 -> P4 -> P6；P3 -> P5 -> P6（pyo3 可与 P4 并行）
- **里程碑**：M1（P2 完成）-> M2（P3 完成）-> M3（P4 完成）-> M4（P5 完成）-> M5（P6 完成）

## 4. 验收标准

1. P0-P6 各阶段验收标准（引用对应文档）
2. 里程碑 M1-M5 达成
3. 风险缓解措施落实

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p0-skeleton-baseline.md`（P0）
- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（P1-P4）
- `docs/superpowers/plans/2026-08-04-p5-pyo3-integration.md`（P5）
- `docs/superpowers/plans/2026-08-04-p6-polish-alignment.md`（P6）
