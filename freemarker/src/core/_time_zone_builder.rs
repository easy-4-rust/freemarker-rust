//! 时区构建器 —— 对应 Java `freemarker.core._TimeZoneBuilder`
//! （timeZoneId → TimeZone.getTimeZone + 无效 ID 检测（GMT 但非 GMT/UTC → 异常）；
//!  Rust 由 chrono-tz 或 Settings.time_zone 字符串承载）

/// Java 类锚点：`_TimeZoneBuilder` 的 Rust 语义由 Settings.time_zone 字符串承载
#[allow(dead_code)]
pub(crate) struct _TimeZoneBuilder;
