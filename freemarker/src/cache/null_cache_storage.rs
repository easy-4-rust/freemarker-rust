//! 空缓存存储 —— 对应 Java `freemarker.cache.NullCacheStorage`
//! （不缓存任何条目；get 恒 miss、put/remove/clear 空操作——配合
//! Configuration.setCacheStorage(NullCacheStorage) 关闭模板缓存）

use crate::cache::cache_storage_with_get_size::CacheStorageWithGetSize;
use crate::cache::CacheStorage;
use crate::template::Template;
use std::rc::Rc;

/// 空缓存存储（对应 NullCacheStorage.java）
pub struct NullCacheStorage;

impl CacheStorage for NullCacheStorage {
    fn get(&mut self, _key: &str) -> Option<Rc<Template>> {
        None
    }

    fn put(&mut self, _key: &str, _value: Rc<Template>) {}

    fn remove(&mut self, _key: &str) {}

    fn clear(&mut self) {}
}

impl CacheStorageWithGetSize for NullCacheStorage {
    fn get_size(&mut self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_storage_never_holds() {
        let mut s = NullCacheStorage;
        assert!(s.get("a.ftl").is_none());
        s.put(
            "a.ftl",
            Rc::new(crate::template::Template::new(
                "a.ftl".to_string(),
                Vec::new(),
                std::collections::HashMap::new(),
                std::rc::Rc::new(crate::template::Configuration::default()),
            )),
        );
        assert!(s.get("a.ftl").is_none());
        assert_eq!(s.get_size(), 0);
    }
}
