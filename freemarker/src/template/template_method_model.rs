//! 方法模板模型 —— 对应 Java `freemarker.template.TemplateMethodModel`
//! （旧式方法接口，参数强制为字符串；已被 TemplateMethodModelEx 取代；
//!  Rust 侧由 `TemplateMethodModelEx` trait 承载）
//!
//! Java `TemplateMethodModel` 是 @Deprecated 的旧接口，
//! `TemplateMethodModelEx` 接受 `TemplateModel` 参数。Rust 只实现 Ex 版本。

/// Java 接口锚点：`TemplateMethodModel`（@Deprecated；Rust 由 `TemplateMethodModelEx` 承载）
#[allow(dead_code)]
pub(crate) struct TemplateMethodModel;
