//! 插值基类 —— 对应 Java `freemarker.core.Interpolation`
//! （抽象类；DollarVariable/NumericalOutput 的父类；
//!  calculateInterpolatedStringOrMarkup 返回 String 或 TemplateMarkupOutputModel）

/// 对应 Java `Interpolation`（ElementKind::Interpolation 变体承载语义）
#[allow(dead_code)]
pub(crate) struct Interpolation;
