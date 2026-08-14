//! 错误描述构建器 —— 对应 Java `freemarker.core._ErrorDescriptionBuilder`
//! （延迟拼接错误消息；description + descriptionParts + blame + tips；
//!  toString 时才组装最终消息；Rust 由 TemplateError 变体直接承载消息）

/// Java 类锚点：`_ErrorDescriptionBuilder` 的 Rust 语义由 TemplateError 消息构造承载
#[allow(dead_code)]
pub(crate) struct _ErrorDescriptionBuilder;
