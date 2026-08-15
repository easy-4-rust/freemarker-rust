//! 直接调用优化内建基类 —— 对应 Java `freemarker.core.BuiltInWithDirectCallOptimization`
//! （支持直接调用优化的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInWithDirectCallOptimization`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化）
#[allow(dead_code)]
pub(crate) struct BuiltInWithDirectCallOptimization;
