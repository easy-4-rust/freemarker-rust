# 数据模型与对象包装设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（template/ 100 文件）
- **依赖**：无

## 1. 目标与范围

将 Java TemplateModel 接口家族迁移为 Rust trait 映射：TModel 角色槽位结构（对应 Java 多接口实现）+ SimpleObjectWrapper（Rust 值 -> TModel）。

详细设计见：`docs/06-数据模型与对象包装.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 数据模型与对象包装 | `docs/06-数据模型与对象包装.md` | TemplateModel 接口家族 -> Rust trait 映射（20+ trait）、TModel 角色槽位结构、SimpleObjectWrapper（DeepUnwrap）、AdapterTemplateModel/WrapperTemplateModel（pyo3 用） |

## 3. 关键设计决策

- **TModel 角色槽位**：struct 字段表达角色（scalar/number/boolean/date/sequence/collection/hash/method/directive/transform/node）
- **SimpleObjectWrapper**：Rust 值 -> TModel，无反射
- **DeepUnwrap**：递归解包到基础类型

## 4. 验收标准

1. 数据模型类型可编译
2. SimpleObjectWrapper 可将 Rust 基础类型包装为 TModel
3. DeepUnwrap 可递归解包

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 4 Task 4.3-4.4）
