//! 类型错误解释接口 —— 对应 Java `freemarker.core._UnexpectedTypeErrorExplainerTemplateModel`
//! （TemplateModel 子接口；explainTypeError 返回 _ErrorDescriptionBuilder tip 数组；
//!  用于改善类型不匹配错误消息；当前 Rust 无调用方）

/// 对应 Java `_UnexpectedTypeErrorExplainerTemplateModel`（当前 Rust 无调用方）
#[allow(dead_code)]
pub(crate) trait UnexpectedTypeErrorExplainer {
    /// 返回类型错误解释提示（Java explainTypeError）
    fn explain_type_error(&self, expected_classes: &[&str]) -> Option<String>;
}
