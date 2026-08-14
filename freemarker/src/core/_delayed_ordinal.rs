//! 惰性序数后缀 —— 对应 Java `freemarker.core._DelayedOrdinal`
//! （_DelayedConversionToString 子类；doConversion = n + "st"/"nd"/"rd"/"th"；
//!  Rust 由 format! + ordinal 函数即时构造覆盖）

/// Java 类锚点：`_DelayedOrdinal`（Rust 由 format! + ordinal 函数即时构造覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedOrdinal;
