//! 惰性生成集合模型 —— 对应 Java `freemarker.core.LazilyGeneratedCollectionModel`
//! （惰性求值的集合模型基类；Rust 侧由 `TemplateCollectionModel` trait 的惰性迭代器承载）

/// Java 抽象类锚点：`LazilyGeneratedCollectionModel`
/// （Rust 侧由 `TemplateCollectionModel` trait 的惰性迭代器承载）
#[allow(dead_code)]
pub(crate) struct LazilyGeneratedCollectionModel;
