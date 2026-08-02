//! Java 测试逻辑的 Rust 1:1 实现（freemarker-core/src/test + freemarker-jython25/src/test）
//!
//! 每个 Java 测试类对应一个模块文件（tests/java_ported/<类名>.rs），测试函数与
//! Java 测试方法同名、同断言、错误消息逐字对齐。共享辅助见 util.rs
//! （对应 freemarker-test-utils 的 TemplateTest 基类）。

#[path = "java_ported/misc_error_messages.rs"]
mod misc_error_messages;
#[path = "java_ported/util.rs"]
pub mod util;
