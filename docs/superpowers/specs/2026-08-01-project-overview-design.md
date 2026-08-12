# 项目概述与范围设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97，2.3 分支线）
- **依赖**：无外部框架依赖

## 1. 目标与范围

在 Rust 中实现与 Apache FreeMarker 语义功能一致的模板引擎：相同模板 + 相同数据模型 -> 相同输出与相同错误行为。

详细设计见：`docs/01-项目概述与范围.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 项目概述与范围 | `docs/01-项目概述与范围.md` | 项目目标、版本基线、迁移范围（范围内/外）、功能对齐矩阵、D1-D5 决策、迁移策略、风险登记 |

## 3. 关键设计决策

| 决策 | 内容 | 影响 |
|------|------|------|
| D1 | serde 替代 BeansWrapper（JVM 反射不实现） | 33 项 NOT_APPLICABLE 永久保留 |
| D2 | fancy-regex 替代 Java 正则 | 反向引用/环视支持，记录不支持清单 |
| D3 | ICI 锁定 2.3.34 | incompatibleImprovements 支持到 2.3.34 |
| D4 | Rust Result 替代 Java 异常传播 | 无日志框架，错误通过 Result 传播 |
| D5 | 无日志框架 | Rust 标准 Result 传播替代 Java SLF4J |

## 4. 验收标准

1. cargo build 通过
2. cargo test 空跑绿
3. D1-D5 有决议并落档
4. 迁移范围明确（范围内/外清单）

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p0-skeleton-baseline.md`（Stage 4：依赖锁定 + 决策落档）
