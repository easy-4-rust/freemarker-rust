//! 惰性短类名 —— 对应 Java `freemarker.core._DelayedShortClassName`
//! （_DelayedConversionToString 子类；doConversion = ClassUtil.getShortClassName(cl, true)；
//!  Rust 无运行时类名反射 → 锚点）

/// Java 类锚点：`_DelayedShortClassName`（Rust 无运行时类名反射）
#[allow(dead_code)]
pub(crate) struct _DelayedShortClassName;
