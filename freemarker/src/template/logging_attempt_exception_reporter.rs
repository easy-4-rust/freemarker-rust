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
