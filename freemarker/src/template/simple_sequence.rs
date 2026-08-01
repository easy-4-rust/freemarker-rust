//! 简单序列 —— 对应 Java `freemarker.template.SimpleSequence` / `SimpleList`
//! （同时实现 TemplateCollectionModel：可枚举；iterator 返回所有权迭代器）

use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::template::{TemplateCollectionModel, TemplateSequenceModel};

pub struct SimpleSequence(pub Vec<TModel>);
impl TemplateSequenceModel for SimpleSequence {
    fn get(&self, index: usize) -> Result<TModel> {
        self.0
            .get(index)
            .cloned()
            .ok_or_else(|| TemplateError::misc(format!("Sequence index out of bounds: {index}")))
    }
    fn size(&self) -> Result<usize> {
        Ok(self.0.len())
    }
}
impl TemplateCollectionModel for SimpleSequence {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        Ok(Box::new(self.0.clone().into_iter().map(Ok)))
    }
}
