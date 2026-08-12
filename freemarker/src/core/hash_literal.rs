//! 哈希字面量模型 —— 对应 Java `freemarker.core.HashLiteral` 的 legacy 分支
//! （HashLiteral.java:126-151，`SequenceHash` 的 ICI < 2.3.21 路径）。

use crate::error::Result;
use crate::template::utility::FnvBuildHasher;
use crate::template::{TModel, TemplateHashModel, TemplateHashModelEx};
use indexmap::IndexMap;

/// 旧版哈希字面量 —— 对应 Java `HashLiteral.SequenceHash` 的 ICI < 2.3.21 分支
/// （HashLiteral.java:126-151）：重复键**保留**——`?keys`/`?values`/`#list h as k, v`
/// 输出全部字面量键值对（含重复键，Java keyList/valueList 全量记录），而 `h[key]`
/// 走 map 覆盖语义（HashMap.put，取最后值）。`size()` 为字面量条目数
/// （Java `size` 字段，HashLiteral.java:157-161，非 map.size()）。
pub struct LegacyHashLiteral {
    /// 全部字面量键值对（插入序，含重复键；Java keyList/valueList 的对偶）
    pairs: Vec<(String, TModel)>,
    /// 键 → 最后值（Java legacy 分支 HashMap.put 的覆盖语义）
    map: IndexMap<String, TModel, FnvBuildHasher>,
}

impl LegacyHashLiteral {
    pub fn new(pairs: Vec<(String, TModel)>) -> Self {
        let mut map = IndexMap::with_hasher(FnvBuildHasher::default());
        for (k, v) in &pairs {
            map.insert(k.clone(), v.clone());
        }
        LegacyHashLiteral { pairs, map }
    }
}

impl TemplateHashModel for LegacyHashLiteral {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        Ok(self.map.get(key).cloned())
    }
    fn is_empty(&self) -> Result<bool> {
        Ok(self.pairs.is_empty())
    }
}

impl TemplateHashModelEx for LegacyHashLiteral {
    fn size(&self) -> Result<usize> {
        Ok(self.pairs.len())
    }
    fn keys(&self) -> Result<Vec<String>> {
        Ok(self.pairs.iter().map(|(k, _)| k.clone()).collect())
    }
    /// 原始键值对列表（覆盖默认 keys+get：Java `keyValuePairsIterator`——
    /// TemplateHashModelEx2 的键值对迭代，重复键各出现一次）
    fn entries(&self) -> Result<Vec<(String, TModel)>> {
        Ok(self.pairs.clone())
    }
}
