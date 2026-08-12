//! 模板模型列表序列 —— 对应 Java `freemarker.template.TemplateModelListSequence`
//! （Java :54 行：List<TemplateModel> → sequence 的只读视图）

use crate::error::Result;
use crate::template::{TModel, TemplateSequenceModel};

/// 模板模型列表序列（对应 TemplateModelListSequence.java）
pub struct TemplateModelListSequence {
    list: Vec<TModel>,
}

impl TemplateModelListSequence {
    /// Java `TemplateModelListSequence(List)`（:31-35）
    pub fn new(list: Vec<TModel>) -> Self {
        TemplateModelListSequence { list }
    }
}

impl TemplateSequenceModel for TemplateModelListSequence {
    fn get(&self, index: usize) -> Result<TModel> {
        self.list
            .get(index)
            .cloned()
            .ok_or_else(|| crate::error::TemplateError::misc("index out of bounds".to_string()))
    }

    fn size(&self) -> Result<usize> {
        Ok(self.list.len())
    }
}
