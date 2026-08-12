# 格式化与自动转义设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（OutputFormat*.java + CFormat*.java + TemplateDateFormat*.java + TemplateNumberFormat*.java）
- **依赖**：配置（07 文档）

## 1. 目标与范围

将 Java OutputFormat 体系 + CFormat + 数字/日期格式化迁移为 Rust 实现：OutputFormat 全家族、`?esc/?no_esc` + autoEscaping 矩阵、CFormat 五种、数字格式化（DecimalFormat 模式子集）、日期格式化（ISO 家族 + 别名工厂）。

详细设计见：`docs/08-格式化与自动转义.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 格式化与自动转义 | `docs/08-格式化与自动转义.md` | OutputFormat 体系（trait + 9 种格式）、autoEscaping 矩阵、CFormat 五种（Legacy/JSON/Java/JS/JSOrJSON/XSC）、数字格式化（DecimalFormat 模式子集）、日期格式化（ISO 家族 + 别名工厂）、`?esc/?no_esc` |

## 3. 关键设计决策

- **OutputFormatKind 枚举**：8 种输出格式类 -> 单一枚举
- **CFormatKind 枚举**：5 种 CFormat 变体
- **autoEscaping 矩阵**：incompatibleImprovements 组合矩阵测试
- **自动转义禁令**：check_legacy_escaping_ban（?html/?xml/?rtf/?web_safe）

## 4. 验收标准

1. OutputFormat 全家族可编译
2. `?esc/?no_esc` 语义正确
3. autoEscaping 矩阵测试通过
4. CFormat 五种格式一致

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 6 Task 6.5）
- `docs/superpowers/plans/2026-08-04-p6-polish-alignment.md`（Task 4.2-4.3：c_format 变体 + 自动转义禁令）
