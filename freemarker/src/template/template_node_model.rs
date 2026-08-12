//! 对应 Java `freemarker.template.TemplateNodeModel（next_sibling/previous_sibling 对应 TemplateNodeModelEx，合并于此）`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::template::TModel;

pub trait TemplateNodeModel {
    fn parent(&self) -> Result<Option<TModel>>;
    fn children(&self) -> Result<Vec<TModel>>;
    fn name(&self) -> Result<Option<String>>;
    fn node_type(&self) -> Result<String>;
    fn namespace(&self) -> Result<Option<String>>;
    /// Java TemplateNodeModelEx.getNextSibling（BuiltInsForNodes.nextSiblingBI）：
    /// 无兄弟节点 → None（?next_sibling 求值为 null）
    fn next_sibling(&self) -> Result<Option<TModel>> {
        Ok(None)
    }
    /// Java TemplateNodeModelEx.getPreviousSibling（BuiltInsForNodes.previousSiblingBI）
    fn previous_sibling(&self) -> Result<Option<TModel>> {
        Ok(None)
    }
}
