//! 默认映射适配器 —— 对应 Java `freemarker.template.DefaultMapAdapter`
//! （Java :162 行：Map → TemplateHashModelEx 适配 + ?api 支持；
//! v1 包装 IndexMap<String, TModel>）

use crate::error::Result;
use crate::template::{TModel, TemplateHashModel, TemplateHashModelEx};
use indexmap::IndexMap;

/// 默认映射适配器（对应 DefaultMapAdapter.java：Map → extended hash）
pub struct DefaultMapAdapter {
    map: IndexMap<String, TModel>,
}

impl DefaultMapAdapter {
    /// Java `adapt(Map, ObjectWrapper)` :50-56；v1 直接包装 IndexMap
    pub fn adapt(map: IndexMap<String, TModel>) -> Self {
        DefaultMapAdapter { map }
    }
}

impl TemplateHashModel for DefaultMapAdapter {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        Ok(self.map.get(key).cloned())
    }

    fn is_empty(&self) -> Result<bool> {
        Ok(self.map.is_empty())
    }
}

impl TemplateHashModelEx for DefaultMapAdapter {
    fn size(&self) -> Result<usize> {
        Ok(self.map.len())
    }

    fn keys(&self) -> Result<Vec<String>> {
        Ok(self.map.keys().cloned().collect())
    }
}
