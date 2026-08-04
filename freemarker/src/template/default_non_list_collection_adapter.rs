//! 默认非列表集合适配器 —— 对应 Java `freemarker.template.DefaultNonListCollectionAdapter`
//! （Java :114 行：Collection（非 List）→ TemplateCollectionModel 适配；
//! v1 包装 Vec 为一次性集合）

use crate::error::Result;
use crate::template::{TModel, TemplateCollectionModel};

/// 默认非列表集合适配器（对应 DefaultNonListCollectionAdapter.java）
pub struct DefaultNonListCollectionAdapter {
    items: Vec<TModel>,
}

impl DefaultNonListCollectionAdapter {
    /// Java `adapt(Collection, ObjectWrapper)`；v1 直接包装 Vec
    pub fn adapt(items: Vec<TModel>) -> Self {
        DefaultNonListCollectionAdapter { items }
    }
}

impl TemplateCollectionModel for DefaultNonListCollectionAdapter {
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
