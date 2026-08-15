//! 混合内容块 —— 对应 Java `freemarker.core.MixedContent`
//! （TemplateElement 子类；容纳文本/指令/插值的混合子元素序列；
//!  postParseCleanup 做空白剥离）

/// 对应 Java `MixedContent`（ElementKind 变体或 exec 承载混合子元素语义）
#[allow(dead_code)]
pub(crate) struct MixedContent;
