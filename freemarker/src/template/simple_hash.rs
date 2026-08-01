//! 简单哈希 —— 对应 Java `freemarker.template.SimpleHash`
//! （?keys/?values 顺序 = 插入序；Java SimpleHash 内部为 LinkedHashMap，
//! v1 用 indexmap 保持插入序）

use crate::error::Result;
use crate::template::TModel;
use crate::template::{TemplateHashModel, TemplateHashModelEx};
use indexmap::IndexMap;

pub struct SimpleHash(pub IndexMap<String, TModel>);
impl TemplateHashModel for SimpleHash {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        Ok(self.0.get(key).cloned())
    }
    fn is_empty(&self) -> Result<bool> {
        Ok(self.0.is_empty())
    }
}
impl TemplateHashModelEx for SimpleHash {
    fn size(&self) -> Result<usize> {
        Ok(self.0.len())
    }
    fn keys(&self) -> Result<Vec<String>> {
        Ok(self.0.keys().cloned().collect())
    }
}
