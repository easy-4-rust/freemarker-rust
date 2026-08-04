//! 规范化输出 —— 对应 Java `freemarker.template.utility.ToCanonical`
//! （`?c` 内建的变换模型实现；v1 的 C 格式化在 builtins/format.rs
//! format_c_number/format_c_string——本类型为 Java 对应物）

use crate::builtins::format::{format_c_number, CFormatKind};

/// 规范化输出（对应 ToCanonical.java；`?c` 语义）
pub struct ToCanonical;

impl ToCanonical {
    /// 数字 → C 字面量（Java `format` 的 Rust 等价）
    pub fn format_number(n: &crate::value::TNumber) -> String {
        format_c_number(n, CFormatKind::JavaScriptOrJson)
    }
}
