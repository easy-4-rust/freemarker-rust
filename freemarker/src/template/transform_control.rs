//! 变换控制 —— 对应 Java `freemarker.template.TransformControl`
//! （Java :99 行：模板变换生命周期控制：START/END/REPAINT 常量 +
//! onStart/afterBody/onError 回调）
//!
//! v1 差异：Rust 变换模型（TemplateTransformModel）无 writer 对象，
//! 生命周期控制由 transform_with_body 承载——本文件对应 Java 接口，
//! 供需要回调语义的变换实现

/// 变换控制（对应 TransformControl.java；常量对应 Java :30-39）
pub trait TransformControl {
    /// Java `TransformControl.START`（:30）
    fn on_start(&self) -> i32 {
        0
    }
    /// Java `TransformControl.END`（:31）
    fn after_body(&self) -> i32 {
        0
    }
    /// Java `TransformControl.REPAINT`（:32）
    fn on_error(&self) -> i32 {
        0
    }
}

/// Java `TransformControl.START`（:30）
pub const START: i32 = 0;
/// Java `TransformControl.END`（:31）
pub const END: i32 = 1;
/// Java `TransformControl.REPAINT`（:32）
pub const REPAINT: i32 = 2;
