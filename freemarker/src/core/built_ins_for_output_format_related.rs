//! 输出格式相关内建 —— 对应 Java `freemarker.core.BuiltInsForOutputFormatRelated`
//! （输出格式相关的内建函数集合；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 类锚点：`BuiltInsForOutputFormatRelated`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化；
///  输出格式内建实现在 `built_ins_for_markup_outputs.rs`）
#[allow(dead_code)]
pub(crate) struct BuiltInsForOutputFormatRelated;
