//! 假布尔模型 —— 对应 Java `freemarker.template.FalseTemplateBooleanModel`
//! （Java :35 行：false 单例；Rust 侧 `TModel::from_boolean(false)` 等价）

use crate::error::Result;
use crate::template::TemplateBooleanModel;

/// 假布尔模型（对应 FalseTemplateBooleanModel.java；单例 INSTANCE）
pub struct FalseTemplateBooleanModel;

impl TemplateBooleanModel for FalseTemplateBooleanModel {
    fn as_boolean(&self) -> Result<bool> {
        Ok(false)
    }
}

impl FalseTemplateBooleanModel {
    /// Java `FalseTemplateBooleanModel.FALSE`（:29-33）
    pub const INSTANCE: FalseTemplateBooleanModel = FalseTemplateBooleanModel;
}
