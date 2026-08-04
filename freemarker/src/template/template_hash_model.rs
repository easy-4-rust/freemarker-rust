//! 对应 Java `freemarker.template.TemplateHashModel`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::template::TModel;

pub trait TemplateHashModel {
    fn get(&self, key: &str) -> Result<Option<TModel>>;
    fn is_empty(&self) -> Result<bool>;
}
