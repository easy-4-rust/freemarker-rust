# 架构设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）
- **依赖**：Cargo workspace、PyO3 0.29、roxmltree

## 1. 目标与范围

定义 freemarker-rust 的 workspace 布局、模块结构、核心类型设计（TModel 角色槽位结构、ExprKind/ElementKind 枚举、TemplateError 统一错误模型）。

详细设计见：`docs/02-架构设计.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 架构设计 | `docs/02-架构设计.md` | Workspace 布局、模块结构（parser/core/template/error/builtins/cache/xml）、TModel 角色槽位、ExprKind/ElementKind 枚举、TemplateError 统一错误模型、渲染循环伪代码、设置继承链 |

## 3. 关键设计决策

- **TModel 角色槽位结构**：对应 Java 多接口实现，Rust 用 struct 字段表达角色
- **ExprKind/ElementKind 枚举**：Java 每类一文件，Rust 用 enum sum type（语言特性）
- **TemplateError 统一错误模型**：Java 30+ 异常类 -> Rust 单一 enum + ErrorCtx
- **设置继承链**：`Option<T>` 表达"未设置"，父链向上查找

## 4. 验收标准

1. workspace build 通过
2. 模块间依赖关系清晰
3. 核心类型可编译

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p0-skeleton-baseline.md`（Stage 1-3）
- `docs/superpowers/plans/2026-08-04-p6-polish-alignment.md`（文件级拆分）
