//! 模板对象基类 —— 对应 Java `freemarker.core.TemplateObject`
//! （@Deprecated 抽象类；持有 template + beginColumn/Line/endColumn/Line 位置信息；
//!  copyFieldsFrom/setLocation/getCanonicalForm；Rust 由 Element/Expr 的 span 字段承载）

/// 对应 Java `TemplateObject`（@Deprecated；Rust 由 Element.span / Expr.span 承载位置语义）
#[allow(dead_code)]
pub(crate) struct TemplateObject;
