//! 不支持的数字类异常 —— 对应 Java
//! `freemarker.template.utility.UnsupportedNumberClassException`
//! （数字模型背后的 Java 数字类型不受支持）

use std::fmt;

/// 不支持的数字类异常（对应 UnsupportedNumberClassException.java）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedNumberClassException(pub String);

impl fmt::Display for UnsupportedNumberClassException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported number class: {}", self.0)
    }
}

impl std::error::Error for UnsupportedNumberClassException {}
