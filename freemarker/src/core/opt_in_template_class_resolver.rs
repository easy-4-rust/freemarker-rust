//! 选择性类解析器 —— 对应 Java `freemarker.core.OptInTemplateClassResolver`
//! （TemplateClassResolver 实现；allowed_classes 白名单 + trusted_templates 信任模板前缀；
//!  信任模板内走 SAFER 策略；Rust 由 template_class_resolver.rs 的 OptInClassResolver 承载）

/// Java 类锚点：`OptInTemplateClassResolver`
/// （Rust 由 template_class_resolver.rs 的 NewBuiltinClassResolver::OptIn 承载）
#[allow(dead_code)]
pub(crate) struct OptInTemplateClassResolver;
