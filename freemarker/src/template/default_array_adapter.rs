//! 默认数组适配器 —— 对应 Java `freemarker.template.DefaultArrayAdapter`
//! （Java :370 行：数组 → sequence 适配（含原始类型数组装箱）；v1 包装
//! Vec<TModel>，原始类型装箱由调用方完成——文档化差异）

use crate::error::Result;
use crate::template::{TModel, TemplateSequenceModel};

/// 默认数组适配器（对应 DefaultArrayAdapter.java：数组 → sequence）
pub struct DefaultArrayAdapter {
    array: Vec<TModel>,
}

impl DefaultArrayAdapter {
    /// Java `adapt(Object array, ObjectWrapper)`；v1 直接包装 Vec<TModel>
    pub fn adapt(array: Vec<TModel>) -> Self {
        DefaultArrayAdapter { array }
    }
}

impl TemplateSequenceModel for DefaultArrayAdapter {
    fn get(&self, index: usize) -> Result<TModel> {
        self.array
            .get(index)
            .cloned()
            .ok_or_else(|| crate::error::TemplateError::misc("index out of bounds".to_string()))
    }

    fn size(&self) -> Result<usize> {
        Ok(self.array.len())
    }
}
