//! 扩展惰性生成集合模型 —— 对应 Java `freemarker.core.LazilyGeneratedCollectionModelEx`
//! （支持 size/isEmpty 查询的惰性集合；Rust 侧由 `TemplateCollectionModel` trait 承载）

/// Java 抽象类锚点：`LazilyGeneratedCollectionModelEx`
/// （Rust 侧由 `TemplateCollectionModel` trait 承载）
#[allow(dead_code)]
pub(crate) struct LazilyGeneratedCollectionModelEx;
