//! 字符串内建基类 —— 对应 Java `freemarker.core.BuiltInForString`
//! （操作字符串值的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInForString`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化；
///  字符串内建实现在 `built_ins_for_strings_basic.rs` 等模块）
#[allow(dead_code)]
pub(crate) struct BuiltInForString;
