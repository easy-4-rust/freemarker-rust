//! 对应 Java `freemarker.template.TemplateCollectionModel（+ TemplateCollectionModelEx 的 iterator 语义）`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::template::TModel;

/// 一次性集合（对应 TemplateCollectionModel：iterator 只能消费一次）
pub trait TemplateCollectionModel {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>>;
}
