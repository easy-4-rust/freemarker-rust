//! 默认列表适配器 —— 对应 Java `freemarker.template.DefaultListAdapter`
//! （Java :114 行：List → TemplateSequenceModel 适配；v1 包装 Vec<TModel>）

use crate::error::Result;
use crate::template::{TModel, TemplateSequenceModel};

/// 默认列表适配器（对应 DefaultListAdapter.java：List → sequence）
pub struct DefaultListAdapter {
    list: Vec<TModel>,
}

impl DefaultListAdapter {
    /// Java `adapt(List, ObjectWrapper)` :52-58；v1 直接包装 Vec
    pub fn adapt(list: Vec<TModel>) -> Self {
        DefaultListAdapter { list }
    }
}

impl TemplateSequenceModel for DefaultListAdapter {
    fn get(&self, index: usize) -> Result<TModel> {
        self.list
            .get(index)
            .cloned()
            .ok_or_else(|| crate::error::TemplateError::misc("index out of bounds".to_string()))
    }

    fn size(&self) -> Result<usize> {
        Ok(self.list.len())
    }
}
