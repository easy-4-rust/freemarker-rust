//! 恢复块 —— 对应 Java `freemarker.core.RecoveryBlock`
//! （TemplateElement 子类；attempt 指令的 recover 分支；accept 直接返回 children）

/// 对应 Java `RecoveryBlock`（ElementKind::Attempt 的 recover 子结构）
#[allow(dead_code)]
pub(crate) struct RecoveryBlock;
