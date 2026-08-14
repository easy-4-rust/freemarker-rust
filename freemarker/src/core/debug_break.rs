//! 调试断点 —— 对应 Java `freemarker.core.DebugBreak`
//! （@Deprecated TemplateElement 子类；DebuggerService.suspendEnvironment 挂起；
//!  Rust 无调试器集成 → 锚点）

/// Java 类锚点：`DebugBreak`（@Deprecated；Rust 无调试器集成）
#[allow(dead_code)]
pub(crate) struct DebugBreak;
