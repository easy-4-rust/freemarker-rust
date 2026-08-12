//! 可序列化布尔模型 —— 对应 Java `freemarker.template.SerializableTemplateBooleanModel`
//! （Java :23 行：标记接口——布尔模型可序列化；Rust 无序列化契约，标记空）

use crate::template::TemplateBooleanModel;

/// 可序列化布尔模型（对应 SerializableTemplateBooleanModel.java；标记接口）
pub trait SerializableTemplateBooleanModel: TemplateBooleanModel {}
