//! 日期类型内建基类 —— 对应 Java `freemarker.core.BuiltInForDate`
//! （操作日期值的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInForDate`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化；
///  日期内建实现在 `built_ins_for_dates.rs`）
#[allow(dead_code)]
pub(crate) struct BuiltInForDate;
