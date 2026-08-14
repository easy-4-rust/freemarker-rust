//! 未知日期类型格式化异常 —— 对应 Java `freemarker.core.UnknownDateTypeFormattingUnsupportedException`
//! （TemplateDateModel 的类型为 UNKNOWN 时无法格式化；Rust 侧由 `TemplateError` 承载）
//!
//! 注意：与 `unknown_date_type_parsing_unsupported_exception` 不同——
//! 本文件对应 Formatting（格式化输出），Parsing 对应解析输入。

use crate::error::TemplateError;

/// Java `UnknownDateTypeFormattingUnsupportedException` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new() -> TemplateError {
    TemplateError::misc(
        "Can't format a date value where the date type (date/time/datetime) is unknown (\"UNKNOWN\").",
    )
}
