//! 强引用缓存存储 —— 对应 Java `freemarker.cache.StrongCacheStorage`
//! （全部条目强引用持有，永不自动淘汰——除 clear 外；Java 注释：实现
//! 非线程安全，由 TemplateCache 负责同步——Rust 侧方法取 &mut self 同语义）

use crate::cache::cache_storage_with_get_size::CacheStorageWithGetSize;
use crate::cache::CacheStorage;
use crate::template::Template;
use std::collections::HashMap;
use std::rc::Rc;

/// 强引用缓存存储（对应 StrongCacheStorage.java）
#[derive(Default)]
pub struct StrongCacheStorage {
    map: HashMap<String, Rc<Template>>,
}

impl CacheStorage for StrongCacheStorage {
    fn get(&mut self, key: &str) -> Option<Rc<Template>> {
        self.map.get(key).cloned()
    }

    fn put(&mut self, key: &str, value: Rc<Template>) {
        self.map.insert(key.to_string(), value);
    }

    fn remove(&mut self, key: &str) {
        self.map.remove(key);
    }

    fn clear(&mut self) {
        self.map.clear();
    }
}

impl CacheStorageWithGetSize for StrongCacheStorage {
    fn get_size(&mut self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::template::Configuration;

    fn tmpl(name: &str) -> Rc<Template> {
        let cfg = Rc::new(Configuration::default());
        Rc::new(parser::parse(&cfg, name, "x").unwrap())
    }

    #[test]
    fn strong_storage_basics() {
        let mut s = StrongCacheStorage::default();
        let t = tmpl("a.ftl");
        assert!(s.get("a.ftl").is_none());
        s.put("a.ftl", t.clone());
        assert!(Rc::ptr_eq(&s.get("a.ftl").unwrap(), &t));
        assert_eq!(s.get_size(), 1);
        s.remove("a.ftl");
        assert!(s.get("a.ftl").is_none());
        s.put("a.ftl", t.clone());
        s.clear();
        assert_eq!(s.get_size(), 0);
    }
}
