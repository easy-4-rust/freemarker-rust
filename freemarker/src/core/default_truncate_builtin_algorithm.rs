//! 默认截断内建算法 —— 对应 Java `freemarker.core.DefaultTruncateBuiltinAlgorithm`
//! （`?truncate` 系列内建的默认算法实现；Rust 侧由 `built_ins_for_strings_basic::truncate` 承载）

/// Java 类锚点：`DefaultTruncateBuiltinAlgorithm`（Rust 侧由 `built_ins_for_strings_basic::truncate` 承载）
///
/// Java `DefaultTruncateBuiltinAlgorithm` 定义了标准 ASCII/Unicode 终止符等常量，
/// Rust 的等价实现在 `built_ins_for_strings_basic` 模块中。
#[allow(dead_code)]
pub(crate) struct DefaultTruncateBuiltinAlgorithm;

impl DefaultTruncateBuiltinAlgorithm {
    /// Java `DefaultTruncateBuiltinAlgorithm.STANDARD_ASCII_TERMINATOR`
    #[allow(dead_code)]
    pub(crate) const STANDARD_ASCII_TERMINATOR: &'static str = "[...]";
    /// Java `DefaultTruncateBuiltinAlgorithm.STANDARD_UNICODE_TERMINATOR`
    #[allow(dead_code)]
    pub(crate) const STANDARD_UNICODE_TERMINATOR: &'static str = "[\u{2026}]";
}
