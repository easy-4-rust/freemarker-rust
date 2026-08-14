//! ICE 链成员接口 —— 对应 Java `freemarker.core.ICIChainMember`
//! （IncompatibleImprovements 链；getMinimumICIVersion 返回最低 ICE 版本，
//!  getPreviousICIChainMember 返回链中前一版本对象；Rust 无 ICE 机制 → 锚点）

/// 对应 Java `ICIChainMember`（Rust 无 IncompatibleImprovements 链机制）
#[allow(dead_code)]
pub(crate) struct ICIChainMember;
