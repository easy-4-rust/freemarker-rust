//! 杂项工具 —— 对应 Java `freemarker.core.MiscUtil`
//! （C_FALSE/C_TRUE 常量；sortMapOfExpressions：按源码位置排序 Map<Expression>；
//!  Rust 无直接对应 → 锚点）

/// Java 类锚点：`MiscUtil` 的 Rust 语义分散在各模块中
#[allow(dead_code)]
pub(crate) struct MiscUtil;

impl MiscUtil {
    /// 常量 "false"
    #[allow(dead_code)]
    pub(crate) const C_FALSE: &'static str = "false";
    /// 常量 "true"
    #[allow(dead_code)]
    pub(crate) const C_TRUE: &'static str = "true";
}
