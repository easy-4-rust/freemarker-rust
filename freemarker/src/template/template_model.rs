//! 模板模型公共接口 —— 对应 Java `freemarker.template.TemplateModel`
//! （所有 FTL 数据类型的公共超级接口；Rust 侧由 `TModel` 结构体承载多角色槽位）
//!
//! Java `TemplateModel` 是所有角色接口（TemplateScalarModel/TemplateNumberModel 等）的
//! 公共超级接口。Rust 的 `TModel` 用 Option 槽位实现等价的多角色语义。

/// Java 接口锚点：`TemplateModel`（Rust 侧由 `TModel` 承载多角色槽位）
///
/// Java `TemplateModel` 定义了 `TemplateModel.NOTHING` 单例，
/// Rust 的等价物是 `TModel::nothing()`。
#[allow(dead_code)]
pub(crate) struct TemplateModel;
