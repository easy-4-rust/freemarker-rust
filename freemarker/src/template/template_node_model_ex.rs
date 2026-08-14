//! 扩展节点模板模型 —— 对应 Java `freemarker.template.TemplateNodeModelEx`
//! （在 TemplateNodeModel 基础上添加 previousSibling/nextSibling 导航；
//!  Rust 侧由 `TemplateNodeModel` trait 的 sibling 方法承载）

/// Java 接口锚点：`TemplateNodeModelEx`
/// （Rust 侧由 `TemplateNodeModel` trait 承载 sibling 导航）
#[allow(dead_code)]
pub(crate) struct TemplateNodeModelEx;
