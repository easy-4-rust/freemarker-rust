//! 模板空值模型 —— 对应 Java `freemarker.core.TemplateNullModel`
//! （单例 INSTANCE；TemplateModel 实现；区分 null 与"未设置"；
//!  fallbackOnNullLoopVariable=false 时循环变量返回此值而非回退上层作用域；
//!  Rust 由 TModel::nothing() 承载 null 语义）

/// Java 类锚点：`TemplateNullModel`（Rust 由 TModel::nothing() 承载 null 语义）
#[allow(dead_code)]
pub(crate) struct TemplateNullModel;
