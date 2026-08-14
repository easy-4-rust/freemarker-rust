//! 序列内建基类 —— 对应 Java `freemarker.core.BuiltInForSequence`
//! （操作序列值的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInForSequence`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化；
///  序列内建实现在 `built_ins_for_sequences.rs`）
#[allow(dead_code)]
pub(crate) struct BuiltInForSequence;
