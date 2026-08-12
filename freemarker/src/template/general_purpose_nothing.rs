//! 全能空模型 —— 对应 Java `freemarker.template.GeneralPurposeNothing`
//! （Java :93 行：scalar ""、boolean false、number 0、sequence 空、hash 空、
//! date 1970-01-01 等全能空角色；Rust 侧 `TModel::nothing()` 为槽位全空
//! 结构，语义等价——本类型承载 Java 对应物的构造与说明）

/// 全能空模型（对应 GeneralPurposeNothing.java；Rust 等价物为
/// `TModel::nothing()`——槽位全空结构）
pub struct GeneralPurposeNothing;

impl GeneralPurposeNothing {
    /// Java `GeneralPurposeNothing.getInstance()`（:31）
    pub fn instance() -> Self {
        GeneralPurposeNothing
    }
}
