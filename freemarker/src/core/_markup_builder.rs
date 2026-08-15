//! 标记构建器 —— 对应 Java `freemarker.core._MarkupBuilder`
//! （泛型类；MarkupOutputFormat.fromMarkup 的薄包装；
//!  build() 调用 fromMarkup(markupSource)；Rust 由 OutputFormat trait 承载）

/// Java 类锚点：`_MarkupBuilder` 的 Rust 语义由 OutputFormat::from_markup 承载
#[allow(dead_code)]
pub(crate) struct _MarkupBuilder;
