//! 自动转义时禁止的内建 —— 对应 Java `freemarker.core.BuiltInBannedWhenAutoEscaping`
//! （自动转义上下文中禁止使用的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInBannedWhenAutoEscaping`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化）
#[allow(dead_code)]
pub(crate) struct BuiltInBannedWhenAutoEscaping;
