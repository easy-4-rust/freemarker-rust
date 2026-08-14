//! 调用位置自定义数据初始化异常 —— 对应 Java `freemarker.core.CallPlaceCustomDataInitializationException`
//! （DirectiveCallPlace.getCustomData 首次调用时初始化失败；checked exception）

use crate::error::TemplateError;

/// Java `CallPlaceCustomDataInitializationException(String, Throwable)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
