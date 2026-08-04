//! 模板模型迭代器 —— 对应 Java `freemarker.template.TemplateModelIterator`
//! （Java :39 行：hasNext/next）

use crate::error::Result;
use crate::template::TModel;

/// 模板模型迭代器（对应 TemplateModelIterator.java）
pub trait TemplateModelIterator {
    fn has_next(&self) -> Result<bool>;
    fn next(&self) -> Result<TModel>;
}
