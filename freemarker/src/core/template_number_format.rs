//! 模板数字格式抽象基类 —— 对应 Java `freemarker.core.TemplateNumberFormat`
//! （数字格式化的公共接口；Rust 侧由 `builtins::format` 模块承载）

/// Java 抽象类锚点：`TemplateNumberFormat`（Rust 侧由 `builtins::format` 模块承载）
///
/// Java `TemplateNumberFormat` 定义 `format(Number)`/`parse(String)` 等方法，
/// Rust 的等价实现在 `builtins::format` 模块中。
#[allow(dead_code)]
pub(crate) struct TemplateNumberFormat;
