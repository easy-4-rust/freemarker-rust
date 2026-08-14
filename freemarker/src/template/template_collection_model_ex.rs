//! 扩展集合模板模型 —— 对应 Java `freemarker.template.TemplateCollectionModelEx`
//! （在 TemplateCollectionModel 基础上添加 size/isEmpty 查询；
//!  Rust 侧由 `TemplateCollectionModel` trait 的 size/isEmpty 方法承载）

/// Java 接口锚点：`TemplateCollectionModelEx`
/// （Rust 侧由 `TemplateCollectionModel` trait 承载 size/isEmpty 查询）
#[allow(dead_code)]
pub(crate) struct TemplateCollectionModelEx;
