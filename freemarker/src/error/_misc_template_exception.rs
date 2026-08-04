//! 通用运行时异常（内部）—— 对应 Java `freemarker.core._MiscTemplateException`
//! （错误描述任意拼接；消息即用户可见文本，无位置/栈附加）

use crate::error::TemplateError;

/// Java `_MiscTemplateException(String description)` 的 Rust 入口
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::Misc {
        message: message.into(),
    }
}
