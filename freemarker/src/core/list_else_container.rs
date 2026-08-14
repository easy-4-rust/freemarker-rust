//! list+else 容器 —— 对应 Java `freemarker.core.ListElseContainer`
//! （IteratorBlock + ElseOfList 的父容器；accept 时先执行 list 部分，
//!  若无迭代项则执行 else 部分）

/// 对应 Java `ListElseContainer`（ElementKind::List 变体承载 list+else 语义）
#[allow(dead_code)]
pub(crate) struct ListElseContainer;
