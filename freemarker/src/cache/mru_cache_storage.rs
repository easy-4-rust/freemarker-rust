//! MRU 缓存存储 —— 对应 Java `freemarker.cache.MruCacheStorage`
//! （Java TemplateCache 的默认存储；强引用区（上限 strong_size_limit）+
//! 软引用区（上限 soft_size_limit）双区双向链表，最近使用在区头；强区满时
//! 最旧条目降级到软区，软区满时移除最旧条目。Java 注释：实现非线程安全，
//! 由 TemplateCache 负责同步——Rust 侧方法取 &mut self 同语义）
//!
//! Rust 近似：软区用 Weak（见 soft_cache_storage.rs 的近似说明）；
//! 双链表用 `VecDeque` 记录使用顺序（尾部 = 最近使用）

use crate::cache::cache_storage_with_get_size::CacheStorageWithGetSize;
use crate::cache::CacheStorage;
use crate::template::Template;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

enum Entry {
    /// 强引用区条目（Java strongHead 链表）
    Strong(Rc<Template>),
    /// 软引用区条目（Java softHead 链表；Rust Weak 近似）
    Soft(std::rc::Weak<Template>),
}

/// MRU 缓存存储（对应 MruCacheStorage.java；构造参数 = Java :78-82 的
/// strongSizeLimit/softSizeLimit）
pub struct MruCacheStorage {
    strong_size_limit: usize,
    soft_size_limit: usize,
    map: HashMap<String, Entry>,
    /// 强区使用顺序（尾部 = 最近使用；Java strongHead 链表）
    strong_order: VecDeque<String>,
    /// 软区使用顺序（Java softHead 链表）
    soft_order: VecDeque<String>,
}

impl MruCacheStorage {
    /// 对应 `MruCacheStorage(strongSizeLimit, softSizeLimit)`（Java:78-82）
    pub fn new(strong_size_limit: usize, soft_size_limit: usize) -> Self {
        MruCacheStorage {
            strong_size_limit,
            soft_size_limit,
            map: HashMap::new(),
            strong_order: VecDeque::new(),
            soft_order: VecDeque::new(),
        }
    }
}

impl Default for MruCacheStorage {
    /// Java TemplateCache 默认 `new MruCacheStorage(50, 50)`（Java:134）
    fn default() -> Self {
        MruCacheStorage::new(50, 50)
    }
}

impl CacheStorage for MruCacheStorage {
    /// Java get（:85-119）：强区命中 → relink 到区头（最近使用）并返回；
    /// 软区命中 → 升级到强区（强区满则先降级最旧强条目），返回；均未命中 → None
    fn get(&mut self, key: &str) -> Option<Rc<Template>> {
        // 先取值副本（避免 map 借用与后续可变借用冲突）
        let value = match self.map.get(key) {
            Some(Entry::Strong(t)) => Some(t.clone()),
            Some(Entry::Soft(w)) => w.upgrade(),
            None => None,
        }?;
        match self.map.get(key) {
            Some(Entry::Strong(_)) => {
                self.relink_strong(key);
            }
            Some(Entry::Soft(_)) => {
                // 软命中 → 升级到强区：先 unlink 软区（避免随后的降级误删
                // 当前条目；Java :90-100 先 unlink 再 relink）
                remove_from(&mut self.soft_order, key);
                if self.strong_size_limit > 0 && self.strong_order.len() >= self.strong_size_limit {
                    if let Some(oldest) = self.strong_order.pop_front() {
                        self.demote(&oldest);
                    }
                }
                self.strong_order.push_back(key.to_string());
            }
            None => {}
        }
        Some(value)
    }

    /// Java put（:123-136）：已存在 → 更新值并 relink；新键 → 插入强区头，
    /// 强区超限降级最旧，软区超限移除最旧
    fn put(&mut self, key: &str, value: Rc<Template>) {
        if self.map.contains_key(key) {
            self.map.insert(key.to_string(), Entry::Strong(value));
            remove_from(&mut self.soft_order, key);
            self.relink_strong(key);
            return;
        }
        self.map.insert(key.to_string(), Entry::Strong(value));
        self.strong_order.push_back(key.to_string());
        if self.strong_size_limit > 0 && self.strong_order.len() > self.strong_size_limit {
            if let Some(oldest) = self.strong_order.pop_front() {
                self.demote(&oldest);
            }
        }
    }

    fn remove(&mut self, key: &str) {
        self.map.remove(key);
        remove_from(&mut self.strong_order, key);
        remove_from(&mut self.soft_order, key);
    }

    fn clear(&mut self) {
        self.map.clear();
        self.strong_order.clear();
        self.soft_order.clear();
    }
}

impl MruCacheStorage {
    /// 降级强条目到软区（Java :104-118：软区满 → 移除最旧软条目）
    fn demote(&mut self, key: &str) {
        let strong: Option<Rc<Template>> = match self.map.get(key) {
            Some(Entry::Strong(t)) => Some(t.clone()),
            _ => None,
        };
        let Some(t) = strong else { return };
        if self.soft_size_limit > 0 && self.soft_order.len() >= self.soft_size_limit {
            if let Some(oldest) = self.soft_order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.map
            .insert(key.to_string(), Entry::Soft(Rc::downgrade(&t)));
        self.soft_order.push_back(key.to_string());
    }

    /// 移到强区使用顺序尾部（Java relinkEntryAfterStrongHead :138-）
    fn relink_strong(&mut self, key: &str) {
        remove_from(&mut self.strong_order, key);
        self.strong_order.push_back(key.to_string());
    }
}

/// 从使用顺序中移除（Java 链表 unlink；自由函数避免 self 双借）
fn remove_from(order: &mut VecDeque<String>, key: &str) {
    if let Some(pos) = order.iter().position(|k| k == key) {
        order.remove(pos);
    }
}

impl CacheStorageWithGetSize for MruCacheStorage {
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
    fn mru_evicts_strong_then_soft() {
        let mut s = MruCacheStorage::new(2, 1);
        // 外部强持有（Java 软引用/调用方持有场景；Weak 近似需要强持有者）
        let (ta, tb, tc, td) = (tmpl("a"), tmpl("b"), tmpl("c"), tmpl("d"));
        s.put("a.ftl", ta.clone());
        s.put("b.ftl", tb.clone());
        s.put("c.ftl", tc.clone());
        // 强区满（2）：最旧的 a 降级到软区，仍可命中
        assert!(s.get("a.ftl").is_some());
        // 再次访问 a → 升级回强区（b 降级）
        assert!(s.get("a.ftl").is_some());
        // 强区满 + 软区满：d 入强区 → 最旧降级 → 软区满移除最旧软条目
        s.put("d.ftl", td.clone());
        assert_eq!(s.get_size(), 3, "总容量 = strong 2 + soft 1");
        // 最近使用的 a/c/d 可命中
        assert!(s.get("a.ftl").is_some());
        assert!(s.get("c.ftl").is_some());
        assert!(s.get("d.ftl").is_some());
    }

    #[test]
    fn mru_unlimited() {
        // 无上限（0 = 无限，与 Java 负数同义）
        let mut s = MruCacheStorage::new(0, 0);
        for i in 0..100 {
            s.put(&format!("t{i}.ftl"), tmpl(&format!("t{i}")));
        }
        assert_eq!(s.get_size(), 100);
        assert!(s.get("t0.ftl").is_some());
    }
}
