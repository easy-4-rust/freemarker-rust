//! 解析异常 —— 对应 Java `freemarker.core.ParseException`
//! （`Syntax error in template "{name}" in line L, column C:\n{details}`，
//! Java 2.3.34 ParseException.getMessage 格式，jar 实测）

use crate::error::TemplateError;

/// Java `ParseException(String message)` 的 Rust 入口（模板名/行列由解析器填充）
#[allow(dead_code)]
pub(crate) fn new(template: String, line: u32, col: u32, message: String) -> TemplateError {
    TemplateError::Parse {
        template,
        line,
        col,
        message,
    }
}
