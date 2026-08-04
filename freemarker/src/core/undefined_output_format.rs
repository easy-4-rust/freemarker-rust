//! 未定义输出格式 —— 对应 Java `freemarker.core.UndefinedOutputFormat`
//! （`?output_format` 未设置时的占位格式；无转义）

/// 未定义输出格式（对应 UndefinedOutputFormat.java；Rust 侧无对应枚举变体——
/// 输出格式始终有值，默认 plainText）
#[allow(dead_code)]
pub(crate) struct UndefinedOutputFormat;

#[allow(dead_code)]
impl UndefinedOutputFormat {
    #[allow(dead_code)]
    pub(crate) fn name() -> &'static str {
        "undefined"
    }
}
