//! FNV-1a 哈希（热路径查找用）—— 变量/宏查找在渲染循环内高频执行
//! （每次 get_variable 都做多次 HashMap 查找），std HashMap 默认 SipHash
//! 对短键开销偏高；FNV-1a 对短 ASCII 键快 ~3-5 倍且确定性（无随机种子）。
//! 仅用于内部表（Namespace/宏帧），迭代序不依赖哈希值（需要确定性序的
//! 读取点已自行排序）；碰撞率对短键可接受（与 Java HashMap 同类权衡）。

use std::hash::BuildHasherDefault;

/// FNV-1a 64 位哈希器（无 unsafe；确定性）
#[derive(Default)]
pub struct FnvHasher(u64);

impl std::hash::Hasher for FnvHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 {
            0xcbf29ce484222325 // FNV offset basis（首次写入时播种）
        } else {
            self.0
        };
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.0 = h;
    }
}

/// FNV 构建器（HashMap<String, V, FnvBuildHasher>）
pub type FnvBuildHasher = BuildHasherDefault<FnvHasher>;
