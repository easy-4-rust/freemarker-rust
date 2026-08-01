//! 简单集合 —— 对应 Java `freemarker.template.SimpleCollection`
//! （一次性迭代语义：TemplateCollectionModel 的 iterator 只能消费一次）

use crate::error::Result;
use crate::template::TModel;
use crate::template::TemplateCollectionModel;

pub struct SimpleCollection(pub Vec<TModel>);
impl TemplateCollectionModel for SimpleCollection {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        Ok(Box::new(self.0.clone().into_iter().map(Ok)))
    }
}
