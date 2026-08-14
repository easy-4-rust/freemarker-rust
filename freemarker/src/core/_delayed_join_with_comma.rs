//! 惰性逗号连接 —— 对应 Java `freemarker.core._DelayedJoinWithComma`
//! （_DelayedConversionToString 子类；doConversion = String.join(", ", items)；
//!  Rust 由 items.join(", ") 即时构造覆盖）

/// Java 类锚点：`_DelayedJoinWithComma`（Rust 由 slice.join(", ") 即时构造覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedJoinWithComma;
