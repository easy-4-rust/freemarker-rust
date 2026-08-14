//! 模板日期格式工厂 —— 对应 Java `freemarker.core.TemplateDateFormatFactory`
//! （创建日期格式实例的工厂；Rust 侧由 `builtins::iso_date_format`/`java_date_format` 承载）

/// Java 抽象类锚点：`TemplateDateFormatFactory`（Rust 侧由日期格式模块承载）
#[allow(dead_code)]
pub(crate) struct TemplateDateFormatFactory;
