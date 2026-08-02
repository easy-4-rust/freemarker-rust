//! Java `freemarker.core.AttemptLoggingTest` 的 Rust 1:1 实现
//! （对应 Java: AttemptLoggingTest —— `<#attempt>/<#recover>` 输出 + AttemptExceptionReporter）。
//!
//! 引擎差异总览：
//! - `<#attempt>/<#recover>` 模板层语义引擎完整支持（exec.rs exec_attempt；
//!   attemptExceptionReporter v1 忽略）。
//! - AttemptExceptionReporter（Java 接口）与 TemplateExceptionHandler（Java 接口）
//!   引擎均无 → 依赖它们的断言 NOT_APPLICABLE；模板层输出断言保留翻译。

// NOT_APPLICABLE: customConfigTest —— 依赖 Java 接口 AttemptExceptionReporter
//   （TestAttemptExceptionReporter 收集 report 调用 + te.getMessage()），引擎无
//   该接口（exec.rs 注释：attemptExceptionReporter v1 忽略）。
// NOT_APPLICABLE: dontReportSuppressedExceptionsTest —— 依赖 Java 接口
//   TemplateExceptionHandler（自定义 handler 写出 "[E]" 并吞掉异常，使 #attempt
//   不触发 recover 且 reporter 不被调用），引擎无异常处理器接口。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java standardConfigTest：
/// - 默认配置下 `<#attempt>${missingVar1}<#recover>r</#attempt>` → "r"；
/// - setAttemptExceptionReporter(LOG_WARN_REPORTER) 后同样输出 "r"
///   （reporter 只负责记录日志，不影响模板输出——引擎无该接口，仅保留模板层断言）。
#[test]
fn standard_config_test() {
    let (c, l) = test_config();
    assert_output(
        &c,
        &l,
        "<#attempt>${missingVar1}<#recover>r</#attempt>",
        "r",
    );
    // Java 注：此处日志应有 #attempt 块的 ERROR 条目，需人工检查——引擎无 reporter，跳过。
    // Java：setAttemptExceptionReporter(AttemptExceptionReporter.LOG_WARN_REPORTER)
    // —— Java 特有接口（NOT_APPLICABLE）；模板输出仍为 "r"：
    assert_output(
        &c,
        &l,
        "<#attempt>${missingVar2}<#recover>r</#attempt>",
        "r",
    );
}
