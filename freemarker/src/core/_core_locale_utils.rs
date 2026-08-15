//! Locale 工具 —— 对应 Java `freemarker.core._CoreLocaleUtils`
//! （getLessSpecificLocale：逐级降低 locale 特定性；
//!  Rust 无 std locale 类型 → 锚点；模板 locale 回退由 cache 模块处理）

/// Java 类锚点：`_CoreLocaleUtils` 的 Rust 语义由 cache 模块 locale 回退承载
#[allow(dead_code)]
pub(crate) struct _CoreLocaleUtils;
