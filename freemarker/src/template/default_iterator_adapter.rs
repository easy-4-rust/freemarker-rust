//! 默认迭代器适配器 —— 对应 Java `freemarker.template.DefaultIteratorAdapter`
//! （Java :128 行：Iterator → 一次性 collection 适配；v1 包装 Vec 快照）

use crate::error::Result;
use crate::template::{TModel, TemplateCollectionModel};

/// 默认迭代器适配器（对应 DefaultIteratorAdapter.java；一次性消费语义）
pub struct DefaultIteratorAdapter {
    items: Vec<TModel>,
}

impl DefaultIteratorAdapter {
    pub fn adapt(items: Vec<TModel>) -> Self {
        DefaultIteratorAdapter { items }
    }
}

impl TemplateCollectionModel for DefaultIteratorAdapter {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        Ok(Box::new(
            self.items
                .iter()
                .cloned()
                .map(Ok)
                .collect::<Vec<_>>()
                .into_iter(),
        ))
    }
}
