//! 局部上下文接口 —— 对应 Java `freemarker.core.LocalContext`
//! （getLocalVariable/getLocalVariableNames；循环体/宏体的局部变量作用域；
//!  Rust 由 Environment 的 local_stack 承载局部变量语义）

/// 对应 Java `LocalContext`（Rust 由 Environment.local_stack 承载局部变量语义）
#[allow(dead_code)]
pub(crate) struct LocalContext;
