//! 对应 Java `freemarker.template.TemplateHashModelEx2`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::template::template_model::TemplateHashModelEx;
use crate::template::TModel;

/// 键值对（对应 TemplateHashModelEx2.KeyValuePair；v1 的 entries() 默认实现
/// 已覆盖 Java Ex2 的 KeyValuePair 迭代语义——本 trait 供需要原始键值对
/// 语义（重复键）的模型覆写）
pub trait TemplateHashModelEx2: TemplateHashModelEx {
    /// 键值对迭代（Java `KeyValuePairIterator`；默认走 entries()）
    fn key_value_pair_iterator(&self) -> Result<Vec<(String, TModel)>> {
        self.entries()
    }
}
