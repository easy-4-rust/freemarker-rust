//! 惰性 "a/an" 前缀 —— 对应 Java `freemarker.core._DelayedAOrAn`
//! （_DelayedConversionToString 子类；doConversion = getAOrAn(s) + " " + s；
//!  Java 惰性字符串拼接优化 → Rust 由 format! 即时构造覆盖）

/// Java 类锚点：`_DelayedAOrAn`（惰性字符串优化；Rust 即时 format! 覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedAOrAn;
