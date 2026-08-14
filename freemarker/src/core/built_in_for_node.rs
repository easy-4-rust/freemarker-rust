//! 节点内建基类 —— 对应 Java `freemarker.core.BuiltInForNode`
//! （操作 TemplateNodeModel 值的内建函数基类；Rust 以 BuiltinFn 签名约束取代 Java 特化基类）
//!
//! 注意：本文件是 Java 基类的锚点，实际节点内建实现在 `built_ins_for_nodes.rs`。

/// Java 抽象类锚点：`BuiltInForNode`
/// （Rust 以 `BuiltinFn` 函数指针签名约束取代 Java 的类层级特化；
///  节点内建实现在 `built_ins_for_nodes.rs`）
#[allow(dead_code)]
pub(crate) struct BuiltInForNodeBase;
