//! 对应 Java `freemarker.core.RangeModel（BoundedRangeModel 等合并；Rust 侧为切片键类型判定规格 RangeSpec）`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

/// 范围模型规格 —— 对应 Java `RangeModel`（`seq[range]`/`"str"[range]` 的切片键
/// 类型判定，Java DynamicKeyName `instanceof RangeModel`；有界与无界共用）
#[derive(Clone, Copy)]
pub struct RangeSpec {
    pub start: i64,
    /// 有界范围长度（`2..5` = 4；`2..!5` = 3；`2..*3` = 3）
    pub count: usize,
    pub ascending: bool,
    /// `2..*` 无界（切片时以目标长度为准）
    pub unbounded: bool,
    /// 右自适应（Java `range.isRightAdaptive()`：`..*` size-limited 与无界；
    /// 切片时越界索引被裁剪而非报错，DynamicKeyName.java:244-262）
    pub adaptive: bool,
    /// Java `RangeModel.isAffectedByStringSlicingBug()`（BoundedRangeModel.java:40-41：
    /// = inclusiveEnd，仅 `a..b` 闭区间范围受影响；`..<`/`..!`/`..*`/无界不受影响）。
    /// 字符串降序切片且结果长为 2 时模拟旧版 bug 返回 "" 而非报错
    /// （DynamicKeyName.java:322-330："foo"[n .. n-1] 给 "" 而非错误）
    pub affected_by_string_slicing_bug: bool,
}
