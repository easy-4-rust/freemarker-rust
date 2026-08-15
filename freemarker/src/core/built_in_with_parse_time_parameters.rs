//! 解析时参数内建基类 —— 对应 Java `freemarker.core.BuiltInWithParseTimeParameters`
//! （在解析阶段接收参数的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInWithParseTimeParameters`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化）
#[allow(dead_code)]
pub(crate) struct BuiltInWithParseTimeParameters;
