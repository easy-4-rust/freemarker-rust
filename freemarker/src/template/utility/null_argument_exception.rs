//! 空参数异常 —— 对应 Java `freemarker.template.utility.NullArgumentException`
//! （参数为 null 时的 IllegalArgumentException；Java 构造
//! `NullArgumentException(String paramName)`）

use std::fmt;

/// 空参数异常（对应 NullArgumentException.java）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullArgumentException(pub String);

impl fmt::Display for NullArgumentException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The \"{}\" argument can't be null.", self.0)
    }
}

impl std::error::Error for NullArgumentException {}

/// 空参数校验（Java `NullArgumentException.check(Object, String)`）
pub fn check<T>(value: Option<T>, param_name: &str) -> Result<T, NullArgumentException> {
    value.ok_or_else(|| NullArgumentException(param_name.to_string()))
}
