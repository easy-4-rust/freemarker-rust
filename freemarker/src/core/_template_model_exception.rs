//! 内部模板模型异常 —— 对应 Java `freemarker.core._TemplateModelException`
//! （TemplateModelException 子类；增加 _ErrorDescriptionBuilder 参数构造；
//!  延迟消息拼接；Rust 由 TemplateError::model_error 承载）

/// Java 类锚点：`_TemplateModelException`
/// （Rust 由 TemplateError 模型错误变体承载）
#[allow(dead_code)]
pub(crate) struct _TemplateModelException;
