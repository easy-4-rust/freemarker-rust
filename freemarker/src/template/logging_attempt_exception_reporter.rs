//! 日志尝试异常报告器 —— 对应 Java
//! `freemarker.template.LoggingAttemptExceptionReporter`
//! （Java :47 行：AttemptExceptionReporter 的日志实现——记录到 SLF4J；
//! v1 无日志框架：默认静默，文档化差异）

use crate::error::TemplateError;
use crate::template::attempt_exception_reporter::AttemptExceptionReporter;

/// 日志尝试异常报告器（对应 LoggingAttemptExceptionReporter.java；
/// v1 静默实现——Rust 无日志框架）
pub struct LoggingAttemptExceptionReporter;

impl AttemptExceptionReporter for LoggingAttemptExceptionReporter {
    fn report_exception(&self, _error: &TemplateError) {
        // v1：无日志框架，静默丢弃（Java 记录 WARN 日志）
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Java AttemptExceptionReporterTest 语义：reportException 不抛出、
    /// 不产生输出——v1 静默实现文档化差异（Java 记录 WARN）
    #[test]
    fn report_exception_is_silent() {
        let reporter = LoggingAttemptExceptionReporter;
        let err = TemplateError::misc("simulated attempt error");
        // 静默实现：调用不 panic、不改变外部状态
        reporter.report_exception(&err);
        reporter.report_exception(&TemplateError::type_mismatch("string", "number"));
    }

    /// 实现 AttemptExceptionReporter trait 的对象可作为 trait object 传入
    /// `<#attempt>` 捕获回调（Java Configuration.setAttemptExceptionReporter）
    #[test]
    fn trait_object_dispatch() {
        let reporter: Box<dyn AttemptExceptionReporter> = Box::new(LoggingAttemptExceptionReporter);
        let err = TemplateError::misc("caught by attempt");
        reporter.report_exception(&err);
    }
}
