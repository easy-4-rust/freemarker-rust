//! 模板值格式抽象基类 —— 对应 Java `freemarker.core.TemplateValueFormat`
//! （所有值格式的公共基类；Rust 侧由 `CFormatKind` 枚举 + 各格式模块承载）

/// Java 抽象类锚点：`TemplateValueFormat`（Rust 侧由格式枚举/模块承载）
///
/// Java `TemplateValueFormat` 是 `TemplateNumberFormat`/`TemplateDateFormat` 的公共基类，
/// 定义 `getDescription()` 方法。Rust 无统一 trait——各格式模块独立实现。
#[allow(dead_code)]
pub(crate) struct TemplateValueFormat;
