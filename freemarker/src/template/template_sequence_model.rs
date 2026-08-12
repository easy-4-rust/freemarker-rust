//! 对应 Java `freemarker.template.TemplateSequenceModel`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::template::TModel;

pub trait TemplateSequenceModel {
    fn get(&self, index: usize) -> Result<TModel>;
    fn size(&self) -> Result<usize>;
}
