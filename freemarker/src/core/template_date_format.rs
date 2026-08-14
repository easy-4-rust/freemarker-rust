//! 模板日期格式抽象基类 —— 对应 Java `freemarker.core.TemplateDateFormat`
//! （日期格式化的公共接口；Rust 侧由 `builtins::iso_date_format`/`java_date_format` 承载）

/// Java 抽象类锚点：`TemplateDateFormat`（Rust 侧由 `builtins::iso_date_format`/`java_date_format` 承载）
#[allow(dead_code)]
pub(crate) struct TemplateDateFormat;
