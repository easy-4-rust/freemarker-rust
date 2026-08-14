//! 未知日期类型解析不支持异常 —— 对应 Java `freemarker.core.UnknownDateTypeParsingUnsupportedException`
//! （UnformattableValueException 子类；未指定 ?date/?time/?datetime 时无法解析日期字符串）

use crate::error::TemplateError;

/// Java `UnknownDateTypeParsingUnsupportedException()` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new() -> TemplateError {
    TemplateError::misc(
        "Can't parse the string to date-like value because it isn't known if the desired result \n         should be a date (no time part), a time, or a date-time value. \n         Use ?date, ?time, or ?datetime to tell FreeMarker the exact type.",
    )
}
