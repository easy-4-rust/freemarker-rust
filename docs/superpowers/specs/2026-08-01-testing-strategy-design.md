# 测试与验证策略设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：N/A（测试策略文档，非迁移文档）
- **依赖**：全部子系统

## 1. 目标与范围

定义 freemarker-rust 的三层测试金字塔：L1 单元测试（Rust #[test]）、L2 黄金套件（数据驱动集成测试）、L3 Java 对比测试 + 性能基准。

详细设计见：`docs/11-测试与验证策略.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 测试与验证策略 | `docs/11-测试与验证策略.md` | 测试金字塔（L1/L2/L3）、L1 单元测试策略（解析器/表达式/内建/算术/格式化/缓存）、L2 黄金套件（.ftl + data + expected）、L3 Java 对比测试 + 性能基准、覆盖率目标（核心模块 >= 85%） |

## 3. 关键设计决策

- **三层测试金字塔**：L1（单元）-> L2（黄金套件）-> L3（Java 对比）
- **golden 套件**：128 用例（MIRRORED/NOT_APPLICABLE/BLOCKED/FAIL 分类）
- **proptest fuzz**：解析器 + 渲染 smoke，1024 用例无 panic
- **criterion 基准**：5 指标落档

## 4. 验收标准

1. cargo test --workspace 全绿
2. golden 套件 113/128 MIRRORED（88%）
3. proptest fuzz 10000 用例无 panic
4. criterion 基准集落档

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 7 Task 7.2）
- `docs/superpowers/plans/2026-08-03-alpha0-production-readiness.md`（Stage C）
- `docs/superpowers/plans/2026-08-03-alpha1-governance-hardening.md`（Stage B6）
