//! 模板异常处理器 —— 对应 Java `freemarker.template.TemplateExceptionHandler`
//! （template_exception_handler 配置设置的接口；
//!  Rust 侧由 `Settings.template_exception_handler` 字符串枚举承载）

/// Java 接口锚点：`TemplateExceptionHandler`
/// （Rust 侧由 `Settings.template_exception_handler` 字符串枚举承载）
///
/// Java 的三种内置实现：
/// - `RETHROW_HANDLER`：重新抛出异常（生产环境默认）
/// - `HTML_DEBUG_HANDLER`：输出 HTML 格式的调试信息
/// - `IGNORE_HANDLER`：忽略异常继续执行
#[allow(dead_code)]
pub(crate) struct TemplateExceptionHandler;
