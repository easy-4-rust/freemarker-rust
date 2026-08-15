//! 扩展节点内建基类 —— 对应 Java `freemarker.core.BuiltInForNodeEx`
//! （操作 TemplateNodeModelEx 值的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）

/// Java 抽象类锚点：`BuiltInForNodeEx`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化）
#[allow(dead_code)]
pub(crate) struct BuiltInForNodeEx;
