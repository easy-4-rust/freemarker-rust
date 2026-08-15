//! 循环变量内建基类 —— 对应 Java `freemarker.core.BuiltInForLoopVariable`
//! （操作循环变量的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInForLoopVariable`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化；
///  循环变量内建实现在 `built_ins_for_loop_variables.rs`）
#[allow(dead_code)]
pub(crate) struct BuiltInForLoopVariable;
