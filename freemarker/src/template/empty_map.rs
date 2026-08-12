//! 空哈希 —— 对应 Java `freemarker.template.EmptyMap`
//! （Java :102 行：不可变空哈希——get 恒 None、is_empty 恒 true、迭代恒空；
//! 指令无参调用时 Java 用空 Map 传入）

use crate::error::Result;
use crate::template::{TModel, TemplateHashModel};

/// 空哈希（对应 EmptyMap.java）
pub struct EmptyMap;

impl TemplateHashModel for EmptyMap {
    fn get(&self, _key: &str) -> Result<Option<TModel>> {
        Ok(None)
    }

    fn is_empty(&self) -> Result<bool> {
        Ok(true)
    }
}
