//! switch case 分支 —— 对应 Java `freemarker.core.Case`
//! （TYPE_CASE=0 / TYPE_DEFAULT=1；condition 为匹配值表达式；
//!  SwitchBlock 按 case/default 顺序求值，首个匹配执行 body）

/// 对应 Java `Case`（ElementKind::Switch 的子元素；Rust 由 SwitchBlock 承载）
#[allow(dead_code)]
pub(crate) struct Case;

impl Case {
    /// case 类型常量：普通 case（Java TYPE_CASE = 0）
    #[allow(dead_code)]
    pub(crate) const TYPE_CASE: i32 = 0;
    /// case 类型常量：default（Java TYPE_DEFAULT = 1）
    #[allow(dead_code)]
    pub(crate) const TYPE_DEFAULT: i32 = 1;
}
