//! 消息工具 —— 对应 Java `freemarker.core._MessageUtil`
//! （UNKNOWN_DATE_TO_STRING_ERROR_MESSAGE 等常量；
//!  newInstantiatingClassNotAllowedException/getAOrAn 等工具方法；
//!  Rust 由 TemplateError 构造与 template_class_resolver 承载）

/// Java 类锚点：`_MessageUtil` 的 Rust 语义分散在 TemplateError 与 template_class_resolver 中
#[allow(dead_code)]
pub(crate) struct _MessageUtil;

impl _MessageUtil {
    /// 未知日期类型转字符串错误消息
    #[allow(dead_code)]
    pub(crate) const UNKNOWN_DATE_TO_STRING_ERROR_MESSAGE: &'static str =
        "Can't convert the date-like value to string because it isn't \
         known if it's a date (no time part), time or date-time value.";
    /// 未知日期类型解析错误消息
    #[allow(dead_code)]
    pub(crate) const UNKNOWN_DATE_PARSING_ERROR_MESSAGE: &'static str =
        "Can't parse the string to date-like value because it isn't \
         known if the desired result should be a date (no time part), a time, or a date-time value.";
}
