//! 默认可迭代适配器 —— 对应 Java `freemarker.template.DefaultIterableAdapter`
//! （Java :90 行：Iterable → collection 适配；v1 包装 Vec）

use crate::error::Result;
use crate::template::{TModel, TemplateCollectionModel};

/// 默认可迭代适配器（对应 DefaultIterableAdapter.java）
pub struct DefaultIterableAdapter {
    items: Vec<TModel>,
}

impl DefaultIterableAdapter {
    pub fn adapt(items: Vec<TModel>) -> Self {
        DefaultIterableAdapter { items }
    }
}

impl TemplateCollectionModel for DefaultIterableAdapter {
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
