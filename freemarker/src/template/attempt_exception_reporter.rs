//! 尝试异常报告器 —— 对应 Java `freemarker.template.AttemptExceptionReporter`
//! （Java :48 行：`<#attempt>` 捕获异常的报告回调；Java 默认
//! LoggingAttemptExceptionReporter 记录日志——Rust 引擎无日志框架，
//! v1 由调用方提供）

use crate::error::TemplateError;

/// 尝试异常报告器（对应 AttemptExceptionReporter.java）
pub trait AttemptExceptionReporter {
    /// 报告 `<#attempt>` 捕获的异常（Java `reportException`）
    fn report_exception(&self, error: &TemplateError);
}
