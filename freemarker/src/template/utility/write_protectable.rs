//! 写保护 —— 对应 Java `freemarker.template.utility.WriteProtectable`
//! （模板模型可写保护：Java 实现在写操作前检查 isWritable）

/// 写保护（对应 WriteProtectable.java）
pub trait WriteProtectable {
    /// 是否可写（Java `isWritable`）
    fn is_writable(&self) -> bool;
}
