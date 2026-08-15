//! Java C 格式 —— 对应 Java `freemarker.core.JavaCFormat`
//! （`CFormatKind::Java` 变体承载；字符串用 javaStringEnc）

/// Java 类锚点：`JavaCFormat`（Rust 侧由 `CFormatKind::Java` 承载）
#[allow(dead_code)]
pub(crate) struct JavaCFormat;

impl JavaCFormat {
    /// Java `JavaCFormat.NAME`
    #[allow(dead_code)]
    pub(crate) const NAME: &'static str = "Java";
}
