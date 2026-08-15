//! 惰性转字符串基类 —— 对应 Java `freemarker.core._DelayedConversionToString`
//! （抽象类；toString 时调用 doConversion 并缓存（double-checked locking）；
//!  所有 _Delayed* 的父类；Rust 惰性求值由 OnceCell/Cow 或即时构造覆盖）

/// Java 类锚点：`_DelayedConversionToString`（惰性字符串基类；Rust 即时构造覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedConversionToString;
