//! 默认枚举适配器 —— 对应 Java `freemarker.template.DefaultEnumerationAdapter`
//! （Java :119 行：Enumeration → 一次性 collection 适配；v1 包装 Vec 快照）

use crate::error::Result;
use crate::template::{TModel, TemplateCollectionModel};

/// 默认枚举适配器（对应 DefaultEnumerationAdapter.java）
pub struct DefaultEnumerationAdapter {
    items: Vec<TModel>,
}

impl DefaultEnumerationAdapter {
    pub fn adapt(items: Vec<TModel>) -> Self {
        DefaultEnumerationAdapter { items }
    }
}

impl TemplateCollectionModel for DefaultEnumerationAdapter {
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
