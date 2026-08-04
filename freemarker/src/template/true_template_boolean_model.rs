//! 真布尔模型 —— 对应 Java `freemarker.template.TrueTemplateBooleanModel`
//! （Java :35 行：true 单例；Rust 侧 `TModel::from_boolean(true)` 等价）

use crate::error::Result;
use crate::template::TemplateBooleanModel;

/// 真布尔模型（对应 TrueTemplateBooleanModel.java；单例 INSTANCE）
pub struct TrueTemplateBooleanModel;

impl TemplateBooleanModel for TrueTemplateBooleanModel {
    fn as_boolean(&self) -> Result<bool> {
        Ok(true)
    }
}

impl TrueTemplateBooleanModel {
    /// Java `TrueTemplateBooleanModel.TRUE`（:29-33）
    pub const INSTANCE: TrueTemplateBooleanModel = TrueTemplateBooleanModel;
}
