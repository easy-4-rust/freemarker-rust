//! 并发缓存存储 —— 对应 Java `freemarker.cache.ConcurrentCacheStorage`
//! （可选接口：标明实现能否并发访问；Java 的 ConcurrentMapCacheStorage 实现
//! 返回 true，其余为 false）

use crate::cache::CacheStorage;

/// 并发缓存存储（对应 ConcurrentCacheStorage.java：仅 isConcurrent() 一个方法）
pub trait ConcurrentCacheStorage: CacheStorage {
    /// 实现是否天然支持并发访问（Java `isConcurrent`）
    fn is_concurrent(&mut self) -> bool;
}
