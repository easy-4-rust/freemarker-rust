//! 可计数缓存存储 —— 对应 Java `freemarker.cache.CacheStorageWithGetSize`
//! （附带 `getSize` 的缓存存储；MruCacheStorage 等实现）

use crate::cache::CacheStorage;

/// 可计数缓存存储（对应 CacheStorageWithGetSize.java：额外提供 getSize）
pub trait CacheStorageWithGetSize: CacheStorage {
    /// 当前条目数（Java `getSize`）
    fn get_size(&mut self) -> usize;
}
