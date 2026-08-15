//! 惰性 toString —— 对应 Java `freemarker.core._DelayedToString`
//! （_DelayedConversionToString 子类；doConversion = String.valueOf(obj)；
//!  Rust 由 format!("{}", ...) 或 Display trait 即时构造覆盖）

/// Java 类锚点：`_DelayedToString`（Rust 由 Display trait 即时构造覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedToString;
