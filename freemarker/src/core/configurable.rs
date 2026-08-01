//! 设置集合 —— 对应 Java `freemarker.core.Configurable`
//! （全部设置项见 docs/07 §2；继承链 v1 为单一层级）

use crate::cache::LookupStrategyKind;
use crate::core::{AutoEscaping, OutputFormatKind};
use crate::template::Version;
use chrono::Offset as _; // fix()（TzOffset → FixedOffset）
use chrono::TimeZone as _; // offset_from_utc_datetime（TzSetting::Named）
use chrono::{FixedOffset, NaiveDateTime};
use chrono_tz::Tz;
use std::str::FromStr;

/// 时区设置 —— 对应 Java `java.util.TimeZone`（Environment.getTimeZone）：
/// IANA 名称（含 DST 规则，如 `Etc/GMT-1`、`America/New_York`）或 `GMT±HH[:mm]`/`UTC`
/// 固定偏移（Java TimeZone.getTimeZone 对 GMT 名字的解析）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TzSetting {
    /// IANA 名称（chrono-tz 含 DST 表）
    Named(Tz),
    /// 固定偏移（GMT+02 / GMT+02:00 / GMT+0230 / UTC）
    Fixed(FixedOffset),
}

impl TzSetting {
    /// 目标 UTC 时刻的固定偏移（DST 感知；对应 Java TimeZone.getOffset(epochMillis)）
    pub fn offset_at(&self, naive_utc: &NaiveDateTime) -> FixedOffset {
        match self {
            TzSetting::Named(t) => t.offset_from_utc_datetime(naive_utc).fix(),
            TzSetting::Fixed(f) => *f,
        }
    }
}

impl FromStr for TzSetting {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        let upper = t.to_ascii_uppercase();
        if upper == "UTC" || upper == "GMT" || upper == "Z" {
            return Ok(TzSetting::Fixed(FixedOffset::east_opt(0).unwrap()));
        }
        if let Some(rest) = upper.strip_prefix("GMT") {
            // GMT+02 / GMT+02:00 / GMT+0230 / GMT-05（Java TimeZone.getTimeZone 的 GMT 名）
            let (sign, rest) = match rest.chars().next() {
                Some('+') => (1i32, &rest[1..]),
                Some('-') => (-1i32, &rest[1..]),
                _ => return Err(()),
            };
            let (h, m) = if let Some(i) = rest.find(':') {
                (
                    rest[..i].parse::<i32>().map_err(|_| ())?,
                    rest[i + 1..].parse::<i32>().map_err(|_| ())?,
                )
            } else if rest.len() == 4 {
                (
                    rest[..2].parse::<i32>().map_err(|_| ())?,
                    rest[2..].parse::<i32>().map_err(|_| ())?,
                )
            } else {
                (rest.parse::<i32>().map_err(|_| ())?, 0)
            };
            if h > 23 || m > 59 {
                return Err(());
            }
            return Ok(TzSetting::Fixed(
                FixedOffset::east_opt(sign * (h * 3600 + m * 60)).unwrap(),
            ));
        }
        // IANA 名（chrono-tz 区分大小写，用原名解析）
        t.parse::<Tz>().map(TzSetting::Named).map_err(|_| ())
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub locale: String,
    pub time_zone: TzSetting,
    /// 时区 ID 字符串（Java `TimeZone.getID()`；`.time_zone` 内置变量读数。
    /// 与 time_zone 的差异：GMT 名归一化为 `GMT±HH:MM`，IANA 名按原样）
    pub time_zone_id: String,
    pub number_format: String,
    pub boolean_format: String,
    pub date_format: String,
    pub time_format: String,
    pub date_time_format: String,
    pub output_format: OutputFormatKind,
    pub auto_escaping: AutoEscaping,
    pub whitespace_stripping: bool,
    pub strict_syntax: bool,
    pub classic_compatible: bool,
    pub incompatible_improvements: Version,
    pub output_encoding: String,
    pub url_escaping_charset: String,
    /// 循环变量为 null 时回退到上次循环（fallbackOnNullLoopVariable）
    pub fallback_on_null_loop_variable: bool,
    /// 模板更新延迟（秒）—— 对应 Configuration.setTemplateUpdateDelay（TemplateCache.setDelay
    /// 语义见 docs/07 §2 :66）；默认 1
    pub delay: u64,
    /// 局部化模板查找 —— 对应 Configuration.setLocalizedLookup（docs/07 §2 :66）；默认 true
    pub localized_lookup: bool,
    /// 模板查找策略 —— 对应 Configuration.setTemplateLookupStrategy（docs/07 §2 :63）；
    /// 默认 Default020300（本地化回退 + acquisition，见 cache/template_lookup_strategy.rs）
    pub lookup_strategy: LookupStrategyKind,
}

/// Java `TimeZone.getTimeZone(id).getID()` 的 v1 复刻（以 Java 实测为准）：
/// - `GMT+1`/`GMT+01`/`GMT+01:00`/`GMT+0130` → 归一化 `GMT+01:00`；
/// - `UTC`/`Z` → `UTC`；`GMT` → `GMT`；
/// - 其余（IANA 名，如 `Etc/GMT-1`、`America/New_York`）按原样返回。
pub fn java_time_zone_id(s: &str) -> String {
    let t = s.trim();
    let upper = t.to_ascii_uppercase();
    if upper == "UTC" || upper == "Z" {
        return "UTC".to_string();
    }
    if upper == "GMT" {
        return "GMT".to_string();
    }
    if let Some(rest) = upper.strip_prefix("GMT") {
        let (sign, rest) = match rest.chars().next() {
            Some('+') => (1i32, &rest[1..]),
            Some('-') => (-1i32, &rest[1..]),
            _ => return t.to_string(),
        };
        let (h, m) = if let Some(i) = rest.find(':') {
            match (rest[..i].parse::<i32>(), rest[i + 1..].parse::<i32>()) {
                (Ok(h), Ok(m)) => (h, m),
                _ => return t.to_string(),
            }
        } else if rest.len() == 4 {
            match (rest[..2].parse::<i32>(), rest[2..].parse::<i32>()) {
                (Ok(h), Ok(m)) => (h, m),
                _ => return t.to_string(),
            }
        } else {
            match rest.parse::<i32>() {
                Ok(h) => (h, 0),
                _ => return t.to_string(),
            }
        };
        return format!("GMT{}{:02}:{:02}", if sign < 0 { "-" } else { "+" }, h, m);
    }
    t.to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            locale: "en_US".to_string(),
            time_zone: TzSetting::Fixed(chrono::FixedOffset::east_opt(0).unwrap()),
            time_zone_id: "GMT+00:00".to_string(),
            number_format: "number".to_string(),
            boolean_format: "true,false".to_string(),
            // Java 默认 ""（Configurable.java:450-458："" 等价 medium，见 setDateFormat javadoc）
            date_format: String::new(),
            time_format: String::new(),
            date_time_format: String::new(),
            output_format: OutputFormatKind::PlainText,
            auto_escaping: AutoEscaping::Default,
            whitespace_stripping: true,
            strict_syntax: false,
            classic_compatible: false,
            incompatible_improvements: Version::V2_3_34,
            output_encoding: "UTF-8".to_string(),
            // Java 默认 null（Configurable.java:491 "outputEncoding and urlEscapingCharset defaults to null"）；
            // 空串 = 未设置（?url 回退 UTF-8，`.url_escaping_charset` 缺失）
            url_escaping_charset: String::new(),
            fallback_on_null_loop_variable: true,
            delay: 1,
            localized_lookup: true,
            lookup_strategy: LookupStrategyKind::Default020300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse_and_int() {
        let v = Version::parse("2.3.34").unwrap();
        assert_eq!(v, Version::V2_3_34);
        assert_eq!(v.to_int(), 2_003_034);
    }

    #[test]
    fn output_format_names() {
        assert_eq!(OutputFormatKind::Html.name(), "HTML");
        assert_eq!(
            OutputFormatKind::parse("html"),
            Some(OutputFormatKind::Html)
        );
        assert!(OutputFormatKind::Html.is_markup());
        assert!(!OutputFormatKind::PlainText.is_markup());
    }
}
