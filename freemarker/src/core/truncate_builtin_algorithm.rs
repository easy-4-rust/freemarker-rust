//! 截断内建算法抽象基类 —— 对应 Java `freemarker.core.TruncateBuiltinAlgorithm`
//! （`?truncate` 系列内建的算法抽象；Rust 侧由 `built_ins_for_strings_basic::truncate` 承载）

/// Java 抽象类锚点：`TruncateBuiltinAlgorithm`（Rust 侧由 `built_ins_for_strings_basic::truncate` 承载）
///
/// Java `TruncateBuiltinAlgorithm` 定义了 `truncateM`/`truncateC`/`truncateW` 等抽象方法，
/// Rust 的等价实现在 `built_ins_for_strings_basic` 模块中。
#[allow(dead_code)]
pub(crate) struct TruncateBuiltinAlgorithm;
