//! 软引用缓存存储 —— 对应 Java `freemarker.cache.SoftCacheStorage`
//! （条目以软引用持有，JVM 内存压力下可被 GC 回收）
//!
//! Rust 无软引用：用 `Rc::downgrade` 弱引用近似——条目的生命周期由强持有者
//! 决定（Java 在内存压力下回收，Rust 在最后一个强引用释放时失效；v1
//! 文档化偏差）。用作 TemplateCache 存储时，模板对象须由上层持有强引用，
//! 否则条目在强持有者释放后即刻失效。

use crate::cache::cache_storage_with_get_size::CacheStorageWithGetSize;
use crate::cache::CacheStorage;
use crate::template::Template;
use std::collections::HashMap;
use std::rc::Rc;

/// 软引用缓存存储（对应 SoftCacheStorage.java；Weak 近似软引用）
#[derive(Default)]
pub struct SoftCacheStorage {
    map: HashMap<String, std::rc::Weak<Template>>,
}

impl CacheStorage for SoftCacheStorage {
    /// 弱引用升级；强持有者已释放 → None（Java：软引用被 GC 回收）
    fn get(&mut self, key: &str) -> Option<Rc<Template>> {
        self.map.get(key)?.upgrade()
    }

    fn put(&mut self, key: &str, value: Rc<Template>) {
        self.map.insert(key.to_string(), Rc::downgrade(&value));
    }

    fn remove(&mut self, key: &str) {
        self.map.remove(key);
    }

    fn clear(&mut self) {
        self.map.clear();
    }
}

impl CacheStorageWithGetSize for SoftCacheStorage {
    fn get_size(&mut self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::template::Configuration;

    #[test]
    fn weak_semantics() {
        let mut s = SoftCacheStorage::default();
        let cfg = Rc::new(Configuration::default());
        let t = Rc::new(parser::parse(&cfg, "a.ftl", "x").unwrap());
        s.put("a.ftl", t.clone());
        // 强持有者存在 → get 成功
        assert!(s.get("a.ftl").is_some());
        // 强持有者释放 → 条目失效（Java 软引用被回收的近似）
        drop(t);
        assert!(s.get("a.ftl").is_none());
    }
}
