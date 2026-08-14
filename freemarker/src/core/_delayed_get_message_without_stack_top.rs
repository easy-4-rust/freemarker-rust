//! 惰性获取无栈顶异常消息 —— 对应 Java `freemarker.core._DelayedGetMessageWithoutStackTop`
//! （_DelayedConversionToString 子类；doConversion = TemplateException.getMessageWithoutStackTop()；
//!  Rust 由 TemplateError 消息格式覆盖）

/// Java 类锚点：`_DelayedGetMessageWithoutStackTop`（Rust 由 TemplateError 消息格式覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedGetMessageWithoutStackTop;
