//! 模板值格式工厂抽象基类 —— 对应 Java `freemarker.core.TemplateValueFormatFactory`
//! （创建格式实例的工厂；Rust 侧由各格式模块的构造函数承载）

/// Java 抽象类锚点：`TemplateValueFormatFactory`（Rust 侧由各格式模块构造函数承载）
#[allow(dead_code)]
pub(crate) struct TemplateValueFormatFactory;
