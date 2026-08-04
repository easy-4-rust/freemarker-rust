//! 对应 Java `freemarker.template.TemplateBooleanModel`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;

pub trait TemplateBooleanModel {
    fn as_boolean(&self) -> Result<bool>;
}
