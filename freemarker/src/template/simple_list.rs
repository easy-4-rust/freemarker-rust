//! 简单列表 —— 对应 Java `freemarker.template.SimpleList`
//! （SimpleSequence 的可变列表子类：add 追加；Java :42 行）

use crate::error::Result;
use crate::template::{TModel, TemplateSequenceModel};

/// 简单列表（对应 SimpleList.java；可变长度的 SimpleSequence）
#[derive(Default, Clone)]
pub struct SimpleList {
    items: Vec<TModel>,
}

impl SimpleList {
    pub fn new() -> Self {
        SimpleList { items: Vec::new() }
    }

    /// 追加（Java `add` :28-31）
    pub fn add(&mut self, item: TModel) {
        self.items.push(item);
    }
}

impl TemplateSequenceModel for SimpleList {
    fn get(&self, index: usize) -> Result<TModel> {
        self.items
            .get(index)
            .cloned()
            .ok_or_else(|| crate::error::TemplateError::misc("index out of bounds".to_string()))
    }

    fn size(&self) -> Result<usize> {
        Ok(self.items.len())
    }
}
