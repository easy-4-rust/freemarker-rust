//! 非标记输出异常 —— 对应 Java `freemarker.core.NonMarkupOutputException`
//! （期望 TemplateMarkupOutputModel 值但得到其他类型；Rust 侧由 `TemplateError` 承载）

use crate::error::TemplateError;

/// Java `NonMarkupOutputException(Environment env)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new() -> TemplateError {
    TemplateError::type_mismatch("markup output", "non-markup value")
}
