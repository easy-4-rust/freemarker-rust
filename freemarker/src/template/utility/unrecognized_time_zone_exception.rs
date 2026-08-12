//! 未知时区异常 —— 对应 Java
//! `freemarker.template.utility.UnrecognizedTimeZoneException`
//! （时区 ID 无法解析）

use std::fmt;

/// 未知时区异常（对应 UnrecognizedTimeZoneException.java）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecognizedTimeZoneException {
    pub time_zone_id: String,
}

impl fmt::Display for UnrecognizedTimeZoneException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unrecognized time zone: {} (FreeMarker doesn't recognize it)",
            self.time_zone_id
        )
    }
}

impl std::error::Error for UnrecognizedTimeZoneException {}
