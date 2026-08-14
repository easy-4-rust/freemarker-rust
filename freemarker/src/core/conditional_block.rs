//! 条件分支块 —— 对应 Java `freemarker.core.ConditionalBlock`
//! （TYPE_IF=0 / TYPE_ELSE=1 / TYPE_ELSE_IF=2；IfBlock 的子结构；
//!  accept: condition==null 或 condition.evalToBoolean → 执行 children）

/// 对应 Java `ConditionalBlock`（ElementKind::If 的子结构；Rust 由 If 变体承载）
#[allow(dead_code)]
pub(crate) struct ConditionalBlock;

impl ConditionalBlock {
    /// 条件类型：if（Java TYPE_IF = 0）
    #[allow(dead_code)]
    pub(crate) const TYPE_IF: i32 = 0;
    /// 条件类型：else（Java TYPE_ELSE = 1）
    #[allow(dead_code)]
    pub(crate) const TYPE_ELSE: i32 = 1;
    /// 条件类型：else if（Java TYPE_ELSE_IF = 2）
    #[allow(dead_code)]
    pub(crate) const TYPE_ELSE_IF: i32 = 2;
}
