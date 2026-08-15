//! 局部上下文栈 —— 对应 Java `freemarker.core.LocalContextStack`
//! （LocalContext[] 数组 + size 计数；push/pop/get；
//!  Rust 由 Environment.local_stack: Vec<LocalEntry> 承载）

/// Java 类锚点：`LocalContextStack`（Rust 由 Environment.local_stack 承载）
#[allow(dead_code)]
pub(crate) struct LocalContextStack;
