//! C 模板数字格式 —— 对应 Java `freemarker.core.CTemplateNumberFormat`
//! （`?c`/`?cn` 的数字格式化实现；Rust 侧由 `builtins::format` 模块承载）

/// Java 类锚点：`CTemplateNumberFormat`（Rust 侧由 `builtins::format` 模块承载）
///
/// Java `CTemplateNumberFormat` 实现 `TemplateNumberFormat`，
/// 提供整数 plain、BigDecimal stripTrailingZeros、Double/Float 最短表示等格式化逻辑。
/// Rust 的等价实现在 `builtins::format` 模块中。
#[allow(dead_code)]
pub(crate) struct CTemplateNumberFormat;
