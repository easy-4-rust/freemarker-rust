//! 惰性 Java 引用 —— 对应 Java `freemarker.core._DelayedJQuote`
//! （_DelayedConversionToString 子类；doConversion = StringUtil.jQuote(toString(obj))；
//!  Rust 由 format!("{:?}", ...) 或 jQuote 工具覆盖）

/// Java 类锚点：`_DelayedJQuote`（Rust 由 Debug 格式化或 jQuote 工具覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedJQuote;
