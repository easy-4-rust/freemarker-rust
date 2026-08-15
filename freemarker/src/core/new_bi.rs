//! ?new 内建 —— 对应 Java `freemarker.core.NewBI`
//! （BuiltIn 子类；eval 返回 ConstructorFunction（TemplateMethodModelEx 实现）；
//!  ConstructorFunction.exec 调用 TemplateClassResolver.resolve + Class.newInstance；
//!  Rust 由 built_in.rs 的 new 分支 + template_class_resolver 承载）

/// Java 类锚点：`NewBI`（Rust 由 built_in.rs ?new 分支 + NewBuiltinClassResolver 承载）
#[allow(dead_code)]
pub(crate) struct NewBI;
