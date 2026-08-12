//! 迭代器→模板模型迭代器适配 —— 对应 Java
//! `freemarker.template.IteratorToTemplateModelIteratorAdapter`
//! （Java :51 行：Iterator<TemplateModel> → TemplateModelIterator 适配）

use crate::error::Result;
use crate::template::{TModel, TemplateModelIterator};

/// 迭代器→模板模型迭代器适配（对应 IteratorToTemplateModelIteratorAdapter.java）
pub struct IteratorToTemplateModelIteratorAdapter {
    inner: std::vec::IntoIter<TModel>,
}

impl IteratorToTemplateModelIteratorAdapter {
    /// Java `adapt(Iterator)`（:33-37）
    pub fn adapt(inner: Vec<TModel>) -> Self {
        IteratorToTemplateModelIteratorAdapter {
            inner: inner.into_iter(),
        }
    }
}

impl TemplateModelIterator for IteratorToTemplateModelIteratorAdapter {
    fn has_next(&self) -> Result<bool> {
        Ok(self.inner.clone().next().is_some())
    }

    fn next(&self) -> Result<TModel> {
        self.inner
            .clone()
            .next()
            .ok_or_else(|| crate::error::TemplateError::misc("no more elements".to_string()))
    }
}
