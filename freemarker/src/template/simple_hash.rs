//! 简单哈希 —— 对应 Java `freemarker.template.SimpleHash`
//! （?keys/?values 顺序 = 插入序；Java SimpleHash 内部为 LinkedHashMap，
//! v1 用 indexmap 保持插入序）
//!
//! 内部表用 FNV 哈希（`utility::FnvBuildHasher`）：哈希成员访问是渲染热路径
//! （`${big.key_0}` 每次点访问一次 get），FNV 对短 ASCII 键比 SipHash 快 3-5 倍；
//! 插入序语义不受哈希器影响（indexmap 的序由内部向量维持）。

use crate::error::Result;
use crate::template::TModel;
use crate::template::{TemplateHashModel, TemplateHashModelEx};
use crate::utility::FnvBuildHasher;
use indexmap::IndexMap;

pub struct SimpleHash(pub IndexMap<String, TModel, FnvBuildHasher>);
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
