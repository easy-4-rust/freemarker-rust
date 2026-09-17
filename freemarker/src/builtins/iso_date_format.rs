//! ISO 8601 / XML Schema 日期格式 —— 对应 Java `ISOLikeTemplateDateFormat.java`（:33-261，
//! formatString 参数解析）+ `ISOTemplateDateFormat.java` / `XSTemplateDateFormat.java`
//! + `DateUtil.dateToISO8601String/dateToXSString`（DateUtil.java:243-405）与
//!   `parseISO8601*/parseXS*`（DateUtil.java:431-805）。
//!
//! 语义要点（Java 对照）：
//!
//! - 格式串分派（Environment.getTemplateDateFormatWithoutCache :2304-2333）：
//!   `xs...` → XS 模式（XML Schema）、`iso...` → ISO 模式、其余 → Java 模式（java_date_format.rs）；
//! - 参数解析（ISOLikeTemplateDateFormat :66-169）：`_`/空格分隔的 h/m/s/ms（精度）、
//!   nz/fz（时区偏移可见性）、u/fu（强制 UTC）；
//! - 格式化（DateUtil.dateToString :264-405）：datePart/timePart/offsetPart/accuracy，
//!   毫秒分数最少位数（MS_FORCED 固定 3 位），偏移 `±HH:MM[:SS]`，UTC → "Z"；
//! - 解析（DateUtil.parseISO8601Date/Time/DateTime、parseXSDate/Time/DateTime）：
//!   扩展（2010-05-15T15:30:44,512+04:00）与基本（20100515T153044,512Z）两种 ISO 形式、
//!   XS 强制 "HH:MM:SS(.f)±HH:MM"；年份 ≤ 0 → BC 纪元，y<1582 用儒略历换算
//!   （Java GregorianCalendar 在 change date 前按儒略历，600-01-01 → 0600-01-03）；
//! - `is_sql`（java.sql.*）值 → zonelessInput：默认不显示时区偏移。

use crate::core::TzSetting;
use crate::error::{Result, TemplateError};
use crate::value::{DateType, DateValue};
use chrono::{DateTime, Datelike, FixedOffset, Timelike, Utc};

// parse/日历换算与测试按 #[path] 聚合拆分（参照 grammar.rs 模式）：
// - iso_date_format_parse.rs —— parse_iso_like 与字段解析、日历换算辅助
// - iso_date_format_tests.rs —— 测试（#[cfg(test)]）
#[path = "iso_date_format_parse.rs"]
mod iso_date_format_parse;
#[cfg(test)]
#[path = "iso_date_format_tests.rs"]
mod iso_date_format_tests;

pub use self::iso_date_format_parse::{fields_to_utc, parse_iso_like};

/// ISO 精度（对应 DateUtil.ACCURACY_*；DateUtil.java:39-59）
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]

pub enum Accuracy {
    Hours,
    Minutes,
    Seconds,
    Milliseconds,
    /// 毫秒强制显示 3 位（参数 `ms`；DateUtil.ACCURACY_MILLISECONDS_FORCED）
    MillisecondsForced,
}

/// ISO/XS 格式参数 —— 对应 ISOLikeTemplateDateFormat 的字段
/// （showZoneOffset/forceUTC/accuracy，ISOLikeTemplateDateFormat.java:39-44）
#[derive(Clone, Copy, Debug)]
pub struct IsoSpec {
    pub accuracy: Accuracy,
    /// None = 默认（非 SQL 值显示偏移；date-only 在 ISO 模式下恒不显示）
    pub show_zone_offset: Option<bool>,
    /// None = 参数 `u`（非 zonelessInput 时用 UTC）；Some(false) = 环境时区（默认）；
    /// Some(true) = 参数 `fu`（恒 UTC）
    pub force_utc: Option<bool>,
}

impl Default for IsoSpec {
    fn default() -> Self {
        IsoSpec {
            accuracy: Accuracy::Milliseconds,
            show_zone_offset: None,
            force_utc: Some(false),
        }
    }
}

/// 解析 ISO/XS 格式串的参数部分 —— 对应 ISOLikeTemplateDateFormat 构造器 :66-169。
/// `format_string` 为完整格式串（"iso nz"、"xs_fz" 等），`prefix_len` 为前缀长度
/// （ISO=3、"iso"；XS=2、"xs"）。
pub fn parse_iso_params(format_string: &str, prefix_len: usize, xs_mode: bool) -> Result<IsoSpec> {
    let mut accuracy = Accuracy::Milliseconds;
    let mut show_zone_offset: Option<bool> = None;
    let mut force_utc: Option<bool> = Some(false);
    let chars: Vec<char> = format_string.chars().collect();
    let ln = chars.len();
    let mut after_separator = false;
    let mut i = prefix_len;
    while i < ln {
        let c = chars[i];
        i += 1;
        if c == '_' || c == ' ' {
            after_separator = true;
        } else {
            if !after_separator {
                return Err(TemplateError::misc(format!(
                    "Missing space or \"_\" before \"{c}\" (at char pos. {i})."
                )));
            }
            match c {
                'h' | 'm' | 's' => {
                    if accuracy != Accuracy::Milliseconds {
                        return Err(TemplateError::misc(format!(
                            "Character \"{c}\" is unexpected as accuracy was already specified earlier (at char pos. {i})."
                        )));
                    }
                    match c {
                        'h' => {
                            if xs_mode {
                                return Err(TemplateError::misc(
                                    "Less than seconds accuracy isn't allowed by the XML Schema format",
                                ));
                            }
                            accuracy = Accuracy::Hours;
                        }
                        'm' => {
                            if i < ln && chars[i] == 's' {
                                i += 1;
                                accuracy = Accuracy::MillisecondsForced;
                            } else {
                                if xs_mode {
                                    return Err(TemplateError::misc(
                                        "Less than seconds accuracy isn't allowed by the XML Schema format",
                                    ));
                                }
                                accuracy = Accuracy::Minutes;
                            }
                        }
                        's' => accuracy = Accuracy::Seconds,
                        _ => unreachable!(),
                    }
                }
                'f' | 'n' => {
                    // 'fu'/'fz' 或 'nz'
                    if c == 'f' && i < ln && chars[i] == 'u' {
                        if force_utc != Some(false) {
                            return Err(TemplateError::misc(
                                "The UTC usage option was already set earlier.",
                            ));
                        }
                        i += 1;
                        force_utc = Some(true);
                        after_separator = false;
                        continue;
                    }
                    if show_zone_offset.is_some() {
                        return Err(TemplateError::misc(format!(
                            "Character \"{c}\" is unexpected as zone offset visibility was already specified earlier. (at char pos. {i})."
                        )));
                    }
                    match c {
                        'n' => {
                            if i < ln && chars[i] == 'z' {
                                i += 1;
                                show_zone_offset = Some(false);
                            } else {
                                return Err(TemplateError::misc(format!(
                                    "\"n\" must be followed by \"z\" (at char pos. {i})."
                                )));
                            }
                        }
                        'f' => {
                            if i < ln && chars[i] == 'z' {
                                i += 1;
                                show_zone_offset = Some(true);
                            } else {
                                return Err(TemplateError::misc(format!(
                                    "\"f\" must be followed by \"z\" (at char pos. {i})."
                                )));
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                'u' => {
                    if force_utc != Some(false) {
                        return Err(TemplateError::misc(
                            "The UTC usage option was already set earlier.",
                        ));
                    }
                    force_utc = None;
                }
                other => {
                    return Err(TemplateError::misc(format!(
                        "Unexpected character, \"{other}\". Expected the beginning of one of: h, m, s, ms, nz, fz, u (at char pos. {i})."
                    )));
                }
            }
            after_separator = false;
        }
    }
    Ok(IsoSpec {
        accuracy,
        show_zone_offset,
        force_utc,
    })
}

/// 是否为 ISO/XS 格式串（"xs..." / "iso..." 前缀；对应 Environment.java:2311-2322）
pub fn is_iso_like(format_string: &str) -> Option<(usize, bool)> {
    let c: Vec<char> = format_string.chars().collect();
    if c.len() >= 2 && c[0] == 'x' && c[1] == 's' {
        Some((2, true))
    } else if c.len() >= 3 && c[0] == 'i' && c[1] == 's' && c[2] == 'o' {
        Some((3, false))
    } else {
        None
    }
}

/// 目标时区 —— 对应 ISOLikeTemplateDateFormat.formatToPlainText :189 的
/// `(forceUTC == null ? !zonelessInput : forceUTC) ? UTC : timeZone`
fn format_tz(spec: &IsoSpec, zoneless_input: bool, env_tz: &TzSetting) -> TzSetting {
    let use_utc = match spec.force_utc {
        Some(true) => true,
        None => !zoneless_input,
        Some(false) => false,
    };
    if use_utc {
        TzSetting::Fixed(FixedOffset::east_opt(0).unwrap())
    } else {
        *env_tz
    }
}

/// 格式化 —— 对应 ISOLikeTemplateDateFormat.formatToPlainText :178-191 +
/// DateUtil.dateToString :264-405（xs_mode 决定 date-only 是否可带偏移）
pub fn format_iso_like(
    d: &DateValue,
    spec: &IsoSpec,
    xs_mode: bool,
    env_tz: &TzSetting,
) -> Result<String> {
    format_iso_like_with_tz(d, spec, xs_mode, &format_tz(spec, d.is_sql, env_tz))
}

/// 显式时区版本（?iso(tz) 内建路径：时区已由调用方解析）
pub fn format_iso_like_with_tz(
    d: &DateValue,
    spec: &IsoSpec,
    xs_mode: bool,
    tz: &TzSetting,
) -> Result<String> {
    let date_part = d.kind != DateType::Time && d.kind != DateType::Unknown;
    let time_part = d.kind != DateType::Date && d.kind != DateType::Unknown;
    // showZoneOffset == null ? !zonelessInput : showZoneOffset（:185-187）
    let offset_part = match spec.show_zone_offset {
        Some(b) => b,
        None => !d.is_sql,
    };
    let utc: DateTime<Utc> = d.dt.with_timezone(&Utc);
    let local = utc.with_timezone(&tz.offset_at(&utc.naive_utc()));
    date_to_string(
        &local,
        date_part,
        time_part,
        // ISO：timePart && offsetPart（ISO 8601:2004 不允许 date-only 带偏移）；
        // XS：offsetPart 原样（DateUtil.dateToString :270-274——jar 实测
        // ?string.xs 对 date-only 输出带 Z，如 "2003-04-05Z"）
        offset_part && (xs_mode || time_part),
        spec.accuracy,
        xs_mode,
    )
}

/// DateUtil.dateToString :264-405 的复刻：字段 → 字符串
fn date_to_string(
    local: &DateTime<FixedOffset>,
    date_part: bool,
    time_part: bool,
    offset_part: bool,
    accuracy: Accuracy,
    xs_mode: bool,
) -> Result<String> {
    if !xs_mode && !time_part && offset_part {
        return Err(TemplateError::misc(
            "ISO 8601:2004 doesn't specify any formats where the offset is shown but the time isn't.",
        ));
    }
    let mut out = String::new();
    if date_part {
        // Java DateUtil.dateToString :296-299：BC 纪元（chrono 年 ≤ 0 = BC 1 起）显示为
        // x = -YEAR + (xsMode ? 0 : 1) —— chrono 年 0 = BC 1 → ISO 显示 0、XS 显示 -1
        let raw = local.year();
        let x = if raw <= 0 && xs_mode { raw - 1 } else { raw };
        if (0..9999).contains(&x) {
            out.push_str(&format!("{x:04}"));
        } else {
            out.push_str(&x.to_string());
        }
        out.push('-');
        out.push_str(&format!("{:02}", local.month()));
        out.push('-');
        out.push_str(&format!("{:02}", local.day()));
        if time_part {
            out.push('T');
        }
    }
    if time_part {
        out.push_str(&format!("{:02}", local.hour()));
        if accuracy >= Accuracy::Minutes {
            out.push(':');
            out.push_str(&format!("{:02}", local.minute()));
            if accuracy >= Accuracy::Seconds {
                out.push(':');
                out.push_str(&format!("{:02}", local.second()));
                if accuracy >= Accuracy::Milliseconds {
                    let ms = local.timestamp_subsec_millis();
                    let forced_digits = if accuracy == Accuracy::MillisecondsForced {
                        3usize
                    } else {
                        0usize
                    };
                    if ms != 0 || forced_digits != 0 {
                        out.push('.');
                        // 最少位数：逐位输出直至 x==0 且强制位数用尽（DateUtil :348-363）
                        let mut x = ms;
                        let mut fd = forced_digits;
                        let mut wrote = 0usize;
                        while x != 0 || fd > 0 {
                            out.push(char::from(b'0' + (x / 100) as u8));
                            x = x % 100 * 10;
                            fd = fd.saturating_sub(1);
                            wrote += 1;
                        }
                        debug_assert!(wrote <= 3);
                    }
                }
            }
        }
    }
    if offset_part {
        let off = local.offset();
        let secs = off.local_minus_utc();
        if secs == 0 {
            out.push('Z');
        } else {
            let sign = if secs < 0 { '-' } else { '+' };
            let s = secs.unsigned_abs();
            let h = s / 3600;
            let m = (s % 3600) / 60;
            let sec = s % 60;
            out.push(sign);
            out.push_str(&format!("{h:02}:{m:02}"));
            if sec != 0 {
                out.push_str(&format!(":{sec:02}"));
            }
        }
    }
    Ok(out)
}
