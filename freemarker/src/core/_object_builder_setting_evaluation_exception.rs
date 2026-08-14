//! 对象构建器设置求值异常 —— 对应 Java `freemarker.core._ObjectBuilderSettingEvaluationException`
//! （checked exception；设置值语法错误或类实例化失败；Rust 由 TemplateError::misc 承载）

/// Java 类锚点：`_ObjectBuilderSettingEvaluationException`
/// （Rust 由 TemplateError::misc 承载设置求值错误语义）
#[allow(dead_code)]
pub(crate) struct _ObjectBuilderSettingEvaluationException;
