//! 旧式转义内建基类 —— 对应 Java `freemarker.core.BuiltInForLegacyEscaping`
//! （旧式转义上下文中的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInForLegacyEscaping`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化）
#[allow(dead_code)]
pub(crate) struct BuiltInForLegacyEscaping;
