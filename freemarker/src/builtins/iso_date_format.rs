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
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeZone, Timelike, Utc};

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
        // XS：offsetPart 原样（DateUtil.dateToString :270-274）
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

/// 解析 —— 对应 ISOLikeTemplateDateFormat.parse :199-218：按 dateType 分派
/// DateUtil.parseISO8601Date/Time/DateTime 或 parseXSDate/Time/DateTime
pub fn parse_iso_like(
    s: &str,
    kind: DateType,
    spec: &IsoSpec,
    env_tz: &TzSetting,
    xs_mode: bool,
) -> Result<DateValue> {
    // tz = forceUTC != FALSE ? UTC : timeZone（:204）
    let tz = match spec.force_utc {
        Some(true) | None => TzSetting::Fixed(FixedOffset::east_opt(0).unwrap()),
        Some(false) => *env_tz,
    };
    let mut parsed = match kind {
        DateType::Date => parse_date(s, &tz, xs_mode)?,
        DateType::Time => parse_time(s, &tz, xs_mode)?,
        DateType::DateTime | DateType::Unknown => parse_date_time(s, &tz, xs_mode)?,
    };
    // 内部解析产出 DateTime 类型；按请求的 dateType 修正（Java SimpleDate(date, dateType)）
    if parsed.kind != kind && kind != DateType::Unknown {
        parsed.kind = kind;
    }
    Ok(parsed)
}

/// 时区偏移解析（DateUtil.parseMatchingTimeZone :763-792；xs 要求 ±HH:MM，iso 允许 ±HH[:MM]）
fn parse_offset(s: &str, xs_mode: bool) -> Result<FixedOffset> {
    if s == "Z" {
        return Ok(FixedOffset::east_opt(0).unwrap());
    }
    let b = s.as_bytes();
    if b.is_empty() || (b[0] != b'+' && b[0] != b'-') {
        return Err(TemplateError::misc("The time zone offset is malformed."));
    }
    let sign: i32 = if b[0] == b'-' { -1 } else { 1 };
    if xs_mode {
        // ±HH:MM
        if b.len() != 6 || b[3] != b':' {
            return Err(TemplateError::misc(
                "The time zone offset didn't match the expected pattern: Z|(?:[-+][0-9]{2}:[0-9]{2})",
            ));
        }
        let h: i32 = s[1..3]
            .parse()
            .map_err(|_| TemplateError::misc("The offset-hours part is a malformed integer."))?;
        let m: i32 = s[4..6]
            .parse()
            .map_err(|_| TemplateError::misc("The offset-minutes part is a malformed integer."))?;
        check_offset(h, m)?;
        Ok(FixedOffset::east_opt(sign * (h * 3600 + m * 60)).unwrap())
    } else {
        // ±HH(?:MM)? / ±HH:MM
        let (h, m) = match b.len() {
            3 => (s[1..3].parse::<i32>().map_err(|_| {
                TemplateError::misc("The offset-hours part is a malformed integer.")
            })?, 0),
            5 => (s[1..3].parse::<i32>().map_err(|_| {
                TemplateError::misc("The offset-hours part is a malformed integer.")
            })?, s[3..5].parse::<i32>().map_err(|_| {
                TemplateError::misc("The offset-minutes part is a malformed integer.")
            })?),
            6 if b[3] == b':' => (s[1..3].parse::<i32>().map_err(|_| {
                TemplateError::misc("The offset-hours part is a malformed integer.")
            })?, s[4..6].parse::<i32>().map_err(|_| {
                TemplateError::misc("The offset-minutes part is a malformed integer.")
            })?),
            _ => {
                return Err(TemplateError::misc(
                    "The time zone offset didn't match the expected pattern: Z|(?:[-+][0-9]{2}(?::?[0-9]{2})?)",
                ))
            }
        };
        check_offset(h, m)?;
        Ok(FixedOffset::east_opt(sign * (h * 3600 + m * 60)).unwrap())
    }
}

fn check_offset(h: i32, m: i32) -> Result<()> {
    if !(0..=23).contains(&h) {
        return Err(TemplateError::misc(
            "The offset-hours part must be at least 0 and can't be more than 23.",
        ));
    }
    if !(0..=59).contains(&m) {
        return Err(TemplateError::misc(
            "The offset-minutes part must be at least 0 and can't be more than 59.",
        ));
    }
    Ok(())
}

/// 小数秒 → 毫秒（DateUtil.groupToMillisecond :794-805：1 位 ×100、2 位 ×10、3 位截断）
fn frac_to_ms(g: &str) -> Result<u32> {
    let digits = if g.len() > 3 { &g[..3] } else { g };
    let v: u32 = digits
        .parse()
        .map_err(|_| TemplateError::misc("The partial-seconds part is a malformed integer."))?;
    Ok(match g.len() {
        1 => v * 100,
        2 => v * 10,
        _ => v,
    })
}

fn parse_int(s: &str, name: &str, min: i64, max: i64) -> Result<i64> {
    let (neg, start) = if let Some(r) = s.strip_prefix('-') {
        (true, r)
    } else {
        (false, s)
    };
    // 去前导零（保留至少 1 位）
    let trimmed = start.trim_start_matches('0');
    let t = if trimmed.is_empty() { "0" } else { trimmed };
    let mut v: i64 = t
        .parse()
        .map_err(|_| TemplateError::misc(format!("The {name} part is a malformed integer.")))?;
    if neg {
        v = -v;
    }
    if v < min {
        return Err(TemplateError::misc(format!(
            "The {name} part must be at least {min}."
        )));
    }
    if v > max {
        return Err(TemplateError::misc(format!(
            "The {name} part can't be more than {max}."
        )));
    }
    Ok(v)
}

/// 日历字段 → UTC 时刻（Java GregorianCalendar 语义：y<1582 用儒略历换算；
/// 供 java_date_format.rs 的 SimpleDateFormat 解析复用）
pub fn fields_to_utc(
    year: i64,
    month: u32,
    day: u32,
    h: u32,
    mi: u32,
    s: u32,
    ms: u32,
) -> Result<DateTime<Utc>> {
    if year >= 1582 {
        let naive = NaiveDate::from_ymd_opt(year as i32, month, day)
            .ok_or_else(|| TemplateError::misc("Date calculation faliure."))?
            .and_hms_milli_opt(h, mi, s, ms)
            .ok_or_else(|| TemplateError::misc("Date calculation faliure."))?;
        Ok(Utc.from_utc_datetime(&naive))
    } else {
        let jdn = julian_day_number(year, month as i64, day as i64);
        let epoch_days = jdn - 2440588;
        let secs = epoch_days * 86400 + h as i64 * 3600 + mi as i64 * 60 + s as i64;
        DateTime::<Utc>::from_timestamp(secs, ms * 1_000_000)
            .ok_or_else(|| TemplateError::misc("Date calculation faliure."))
    }
}

/// 日历字段 → 日期值。年份 ≤ 0（BC）或 y<1582 → 儒略历换算（Java GregorianCalendar
/// 在 change date 1582-10-15 前按儒略历；DateUtil.parseDate_parseMatcher :476-491）
/// 日期字段构造器（8 参数）；豁免 too_many_arguments
#[allow(clippy::too_many_arguments)]
fn fields_to_date(
    year: i64,
    month: u32,
    day: u32,
    h: u32,
    mi: u32,
    s: u32,
    ms: u32,
    tz: FixedOffset,
) -> Result<DateValue> {
    // 字段按墙钟时间解读：时刻 = 字段(UTC) − 偏移（Java Calendar 语义；
    // 修正：带 +02:00 的 "22:38" → 20:38Z）
    let epoch = fields_to_utc(year, month, day, h, mi, s, ms)? - tz;
    Ok(DateValue::new(epoch.with_timezone(&tz), DateType::DateTime))
}

/// 儒略日数（儒略历；仅 y<1582 时调用）。公式：JDN = 367Y − (7(Y+5001+(M−9)/7))/4
/// + (275M)/9 + D + 1729777
fn julian_day_number(y: i64, m: i64, d: i64) -> i64 {
    367 * y - (7 * (y + 5001 + (m - 9) / 7)) / 4 + (275 * m) / 9 + d + 1729777
}

/// DateUtil.parseDate_parseMatcher :463-499（xsMode：年份语义 + 时区可选）
fn parse_date(s: &str, tz: &TzSetting, xs_mode: bool) -> Result<DateValue> {
    // XS：(-?[0-9]+)-([0-9]{2})-([0-9]{2})(Z|±HH:MM)?
    // ISO 扩展：(-?[0-9]{4,})-([0-9]{2})-([0-9]{2})  基本：(-?[0-9]{4,}?)([0-9]{2})([0-9]{2})
    if xs_mode {
        let (date_part, zone) = split_xs_zone(s)?;
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() != 3 {
            return Err(TemplateError::misc(
                "The value didn't match the expected pattern: (-?[0-9]+)-([0-9]{2})-([0-9]{2})(Z|(?:[-+][0-9]{2}:[0-9]{2}))?",
            ));
        }
        let year = parse_int(parts[0], "year", i64::MIN, i64::MAX)?;
        let month = parse_int(parts[1], "month", 1, 12)? as u32;
        let day = parse_int(parts[2], "day-of-month", 1, 31)? as u32;
        let year = xs_year_to_era(year, xs_mode)?;
        let off = match zone {
            Some(z) => parse_offset(z, true)?,
            None => tz.offset_at(&Utc::now().naive_utc()),
        };
        fields_to_date(year, month, day, 0, 0, 0, 0, off)
    } else {
        let m = match split_iso_date(s) {
            Some(m) => m,
            None => {
                return Err(TemplateError::misc(
                    "The value didn't match the expected pattern: (-?[0-9]{4,})-([0-9]{2})-([0-9]{2}) or (-?[0-9]{4,}?)([0-9]{2})([0-9]{2})",
                ))
            }
        };
        let off = tz.offset_at(&Utc::now().naive_utc());
        let year = xs_year_to_era(m.year, xs_mode)?;
        fields_to_date(year, m.month as u32, m.day as u32, 0, 0, 0, 0, off)
    }
}

/// 年份 → 纪元换算（DateUtil.parseDate_parseMatcher :476-484：ISO 0000 = BC 1，
/// XS 无 0 年）；返回调整后的儒略年（BC 1 → 0、BC 2 → -1，XSD 0 年报错）
fn xs_year_to_era(year: i64, xs_mode: bool) -> Result<i64> {
    if year <= 0 {
        let adj = -year + if xs_mode { 0 } else { 1 };
        if adj == 0 {
            return Err(TemplateError::misc(
                "Year 0 is not allowed in XML schema dates. BC 1 is -1, AD 1 is 1.",
            ));
        }
        Ok(adj)
    } else {
        Ok(year)
    }
}

struct IsoDateMatch {
    year: i64,
    month: i64,
    day: i64,
}

/// ISO 日期匹配（扩展优先，再基本；无时区——DateUtil PATTERN_ISO8601_*_DATE :93-96）
/// 扩展：(-?[0-9]{4,})-([0-9]{2})-([0-9]{2})；基本：(-?[0-9]{4,}?)([0-9]{2})([0-9]{2})
fn split_iso_date(s: &str) -> Option<IsoDateMatch> {
    let b = s.as_bytes();
    let neg = b.first() == Some(&b'-');
    let start = if neg { 1 } else { 0 };
    let body = &s[start..];
    let year_s;
    let rest;
    if let Some(d1) = body.find('-') {
        // 扩展形式
        if d1 == 0 {
            return None;
        }
        year_s = &body[..d1];
        rest = &body[d1 + 1..];
        if year_s.len() < 4 || !year_s.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let (month_s, day_s) = rest.split_once('-')?;
        if month_s.len() != 2 || day_s.len() != 2 {
            return None;
        }
        let year = signed_parse(neg, year_s)?;
        let month: i64 = month_s.parse().ok()?;
        let day: i64 = day_s.parse().ok()?;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        Some(IsoDateMatch { year, month, day })
    } else {
        // 基本形式：YYYYMMDD（4+ 位年 + 2 位月 + 2 位日）
        let b2 = body.as_bytes();
        let n = b2.len();
        if n < 8 || !b2.iter().all(|c| c.is_ascii_digit()) {
            return None;
        }
        year_s = &body[..n - 4];
        if year_s.len() < 4 {
            return None;
        }
        let year = signed_parse(neg, year_s)?;
        let month = ((b2[n - 4] - b'0') * 10 + (b2[n - 3] - b'0')) as i64;
        let day = ((b2[n - 2] - b'0') * 10 + (b2[n - 1] - b'0')) as i64;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        Some(IsoDateMatch { year, month, day })
    }
}

fn signed_parse(neg: bool, s: &str) -> Option<i64> {
    let v: i64 = s.parse().ok()?;
    Some(if neg { -v } else { v })
}

/// 拆分 XS 可选时区（...)(Z|±HH:MM)?）
fn split_xs_zone(s: &str) -> Result<(&str, Option<&str>)> {
    if let Some(z) = s.strip_suffix('Z') {
        return Ok((z, Some("Z")));
    }
    if s.len() > 6 {
        let b = s.as_bytes();
        let c = b[b.len() - 6];
        if (c == b'+' || c == b'-') && b[b.len() - 3] == b':' {
            return Ok((&s[..s.len() - 6], Some(&s[s.len() - 6..])));
        }
    }
    Ok((s, None))
}

/// XS 时间：([0-9]{2}):([0-9]{2}):([0-9]{2})(?:\.([0-9]+))?(Z|±HH:MM)?
/// ISO 扩展：([0-9]{2})(?::([0-9]{2})(?::([0-9]{2})(?:[\\.,]([0-9]+))?)?)?(Z|±HH(:MM)?)?
/// ISO 基本：([0-9]{2})(?:([0-9]{2})(?:([0-9]{2})(?:[\\.,]([0-9]+))?)?)?(Z|±HH(?:[0-9]{2})?)?
fn parse_time(s: &str, tz: &TzSetting, xs_mode: bool) -> Result<DateValue> {
    if xs_mode {
        let (body, zone) = split_xs_zone(s)?;
        let parts: Vec<&str> = body.split(':').collect();
        if parts.len() != 3 {
            return Err(TemplateError::misc(
                "The value didn't match the expected pattern: ([0-9]{2}):([0-9]{2}):([0-9]{2})(?:\\.([0-9]+))?(Z|(?:[-+][0-9]{2}:[0-9]{2}))?",
            ));
        }
        let (hms, frac) = match parts[2].split_once('.') {
            Some((h, f)) => (h, Some(f)),
            None => (parts[2], None),
        };
        let h = parse_int(parts[0], "hour-of-day", 0, 24)? as u32;
        let mi = parse_int(parts[1], "minute", 0, 59)? as u32;
        let sec = parse_int(hms, "second", 0, 60)? as u32;
        let ms = match frac {
            Some(f) => frac_to_ms(f)?,
            None => 0,
        };
        let off = match zone {
            Some(z) => parse_offset(z, true)?,
            None => tz.offset_at(&Utc::now().naive_utc()),
        };
        time_fields_to_value(h, mi, sec, ms, off)
    } else {
        parse_iso_time(s, tz)
    }
}

/// ISO 时间解析：扩展/基本两种形式
fn parse_iso_time(s: &str, tz: &TzSetting) -> Result<DateValue> {
    // 从尾部切时区（Z / ±HH / ±HHMM / ±HH:MM）
    let (body, zone) = split_iso_zone(s);
    let (h, mi, sec, ms) = parse_iso_time_fields(body)?;
    let off = match zone {
        Some(z) => parse_offset(z, false)?,
        None => tz.offset_at(&Utc::now().naive_utc()),
    };
    time_fields_to_value(h, mi, sec, ms, off)
}

fn check_hms(h: u32, mi: u32, sec: u32) -> Result<()> {
    if h > 24 {
        return Err(TemplateError::misc(
            "The hour-of-day part can't be more than 24.",
        ));
    }
    if mi > 59 {
        return Err(TemplateError::misc(
            "The minute part can't be more than 59.",
        ));
    }
    if sec > 60 {
        return Err(TemplateError::misc(
            "The second part can't be more than 60.",
        ));
    }
    if h == 24 && (mi != 0 || sec != 0) {
        return Err(TemplateError::misc(
            "Hour 24 is only allowed in the case of midnight.",
        ));
    }
    Ok(())
}

/// 时间字段 → 1970-01-01 基准的 DateValue（DateUtil.parseTime_parseMatcher :534-586）
fn time_fields_to_value(h: u32, mi: u32, sec: u32, ms: u32, off: FixedOffset) -> Result<DateValue> {
    let day = if h == 24 { 2 } else { 1 };
    let h = h % 24;
    let naive = NaiveDate::from_ymd_opt(1970, 1, day)
        .ok_or_else(|| TemplateError::misc("Date calculation faliure."))?
        .and_hms_milli_opt(h, mi, sec, ms)
        .ok_or_else(|| TemplateError::misc("Date calculation faliure."))?;
    let utc = Utc.from_utc_datetime(&naive) - off;
    Ok(DateValue::new(utc.with_timezone(&off), DateType::Time))
}

/// 从 ISO 时间/日期时间串尾部切时区（Z / ±HH / ±HHMM / ±HH:MM）
fn split_iso_zone(s: &str) -> (&str, Option<&str>) {
    if let Some(z) = s.strip_suffix('Z') {
        return (z, Some("Z"));
    }
    let b = s.as_bytes();
    if b.len() >= 3 {
        let c = b[b.len() - 3];
        if c == b'+' || c == b'-' {
            // ±HH
            return (&s[..s.len() - 3], Some(&s[s.len() - 3..]));
        }
    }
    if b.len() >= 5 {
        let c = b[b.len() - 5];
        if c == b'+' || c == b'-' {
            // ±HHMM
            return (&s[..s.len() - 5], Some(&s[s.len() - 5..]));
        }
    }
    if b.len() >= 6 {
        let c = b[b.len() - 6];
        if (c == b'+' || c == b'-') && b[b.len() - 3] == b':' {
            // ±HH:MM
            return (&s[..s.len() - 6], Some(&s[s.len() - 6..]));
        }
    }
    (s, None)
}

/// 日期时间：XS = XS 日期 + "T" + XS 时间 + XS 时区；
/// ISO 扩展/基本 = 日期 + "T" + 时间 + 时区（时区仅出现在时间部分之后）
fn parse_date_time(s: &str, tz: &TzSetting, xs_mode: bool) -> Result<DateValue> {
    let Some((date_part, time_part)) = s.split_once('T') else {
        return Err(TemplateError::misc(
            "The value didn't match the expected date-time pattern (missing \"T\" separator).",
        ));
    };
    if xs_mode {
        let (time_body, zone) = split_xs_zone(time_part)?;
        let parts: Vec<&str> = time_body.split(':').collect();
        if parts.len() != 3 {
            return Err(TemplateError::misc(
                "The value didn't match the expected pattern: (-?[0-9]+)-([0-9]{2})-([0-9]{2})T([0-9]{2}):([0-9]{2}):([0-9]{2})(?:\\.([0-9]+))?(Z|(?:[-+][0-9]{2}:[0-9]{2}))?",
            ));
        }
        let (hms, frac) = match parts[2].split_once('.') {
            Some((h, f)) => (h, Some(f)),
            None => (parts[2], None),
        };
        let h = parse_int(parts[0], "hour-of-day", 0, 24)? as u32;
        let mi = parse_int(parts[1], "minute", 0, 59)? as u32;
        let sec = parse_int(hms, "second", 0, 60)? as u32;
        let ms = match frac {
            Some(f) => frac_to_ms(f)?,
            None => 0,
        };
        check_hms(h, mi, sec)?;
        let off = match zone {
            Some(z) => parse_offset(z, true)?,
            None => tz.offset_at(&Utc::now().naive_utc()),
        };
        date_time_fields(date_part, h, mi, sec, ms, off)
    } else {
        // ISO：时区在时间尾部
        let (time_body, zone) = split_iso_zone(time_part);
        let hms = parse_iso_time_fields(time_body)?;
        let off = match zone {
            Some(z) => parse_offset(z, false)?,
            None => tz.offset_at(&Utc::now().naive_utc()),
        };
        date_time_fields(date_part, hms.0, hms.1, hms.2, hms.3, off)
    }
}

/// ISO 时间字段（扩展/基本；无时区部分）。扩展 `HH:MM:SS(.f)?`、基本 `HHMMSS(.f)?`，
/// 分/秒可省略（DateUtil PATTERN_ISO8601_*_TIME_BASE :87-89）
fn parse_iso_time_fields(s: &str) -> Result<(u32, u32, u32, u32)> {
    let b = s.as_bytes();
    if b.len() < 2 || !b[..2].iter().all(|c| c.is_ascii_digit()) {
        return Err(TemplateError::misc(
            "The value didn't match the expected ISO 8601 time pattern.",
        ));
    }
    let h = ((b[0] - b'0') * 10 + (b[1] - b'0')) as u32;
    let mut rest = &s[2..];
    if let Some(r) = rest.strip_prefix(':') {
        rest = r;
    }
    let (mi, sec, ms) = if rest.is_empty() {
        (0, 0, 0)
    } else {
        if rest.len() < 2 || !rest[..2].bytes().all(|c| c.is_ascii_digit()) {
            return Err(TemplateError::misc(
                "The value didn't match the expected ISO 8601 time pattern.",
            ));
        }
        let mi = ((rest.as_bytes()[0] - b'0') * 10 + (rest.as_bytes()[1] - b'0')) as u32;
        rest = &rest[2..];
        if let Some(r) = rest.strip_prefix(':') {
            rest = r;
        }
        if rest.is_empty() {
            (mi, 0, 0)
        } else {
            let (sec_part, frac) = match rest.split_once([',', '.']) {
                Some((a, f)) => (a, Some(f)),
                None => (rest, None),
            };
            if sec_part.is_empty() {
                // 只有小数部分（秒省略）
                let ms = match frac {
                    Some(f) => frac_to_ms(f)?,
                    None => 0,
                };
                (mi, 0, ms)
            } else {
                if sec_part.len() != 2 || !sec_part.bytes().all(|c| c.is_ascii_digit()) {
                    return Err(TemplateError::misc(
                        "The value didn't match the expected ISO 8601 time pattern.",
                    ));
                }
                let sec =
                    ((sec_part.as_bytes()[0] - b'0') * 10 + (sec_part.as_bytes()[1] - b'0')) as u32;
                let ms = match frac {
                    Some(f) => frac_to_ms(f)?,
                    None => 0,
                };
                (mi, sec, ms)
            }
        }
    };
    check_hms(h, mi, sec)?;
    Ok((h, mi, sec, ms))
}

/// 日期 + 时间字段 → DateValue（DateUtil.parseDateTime_parseMatcher :629-698）
fn date_time_fields(
    date_part: &str,
    h: u32,
    mi: u32,
    sec: u32,
    ms: u32,
    off: FixedOffset,
) -> Result<DateValue> {
    let m = split_iso_date(date_part)
        .ok_or_else(|| TemplateError::misc("The date part didn't match the expected pattern."))?;
    let year = xs_year_to_era(m.year, false)?;
    let instant = if year >= 1582 {
        let naive = NaiveDate::from_ymd_opt(year as i32, m.month as u32, m.day as u32)
            .ok_or_else(|| TemplateError::misc("Date calculation faliure."))?
            .and_hms_milli_opt(h, mi, sec, ms)
            .ok_or_else(|| TemplateError::misc("Date calculation faliure."))?;
        Utc.from_utc_datetime(&naive)
    } else {
        // 儒略历（year<1582；BC 由 julian_day_number 处理）
        let jdn = julian_day_number(year, m.month, m.day);
        let epoch_days = jdn - 2440588;
        let secs = epoch_days * 86400 + h as i64 * 3600 + mi as i64 * 60 + sec as i64;
        DateTime::<Utc>::from_timestamp(secs, ms * 1_000_000)
            .ok_or_else(|| TemplateError::misc("Date calculation faliure."))?
    };
    Ok(DateValue::new(
        (instant - off).with_timezone(&off),
        DateType::DateTime,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TzSetting;

    fn tz_gmt2() -> TzSetting {
        "GMT+02".parse().unwrap()
    }

    #[test]
    fn iso_params_parse() {
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(spec.accuracy, Accuracy::Milliseconds);
        assert_eq!(spec.show_zone_offset, None);
        let spec = parse_iso_params("iso ms nz", 3, false).unwrap();
        assert_eq!(spec.accuracy, Accuracy::MillisecondsForced);
        assert_eq!(spec.show_zone_offset, Some(false));
        let spec = parse_iso_params("xs_fz", 2, true).unwrap();
        assert_eq!(spec.show_zone_offset, Some(true));
        let spec = parse_iso_params("iso_s_u", 3, false).unwrap();
        assert_eq!(spec.accuracy, Accuracy::Seconds);
        assert_eq!(spec.force_utc, None);
        let spec = parse_iso_params("iso_h", 3, false).unwrap();
        assert_eq!(spec.accuracy, Accuracy::Hours);
        // XS 不允许 h/m 精度
        assert!(parse_iso_params("xs_h", 2, true).is_err());
        assert!(parse_iso_params("iso x", 3, false).is_err());
    }

    fn d(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, ms: u32) -> DateValue {
        let naive = NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_milli_opt(h, mi, s, ms)
            .unwrap();
        DateValue::new(
            Utc.from_utc_datetime(&naive)
                .with_timezone(&FixedOffset::east_opt(0).unwrap()),
            DateType::DateTime,
        )
    }

    #[test]
    fn format_iso_variants() {
        let dt = d(2010, 5, 15, 20, 38, 5, 23);
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dt, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.023+02:00"
        );
        let spec = parse_iso_params("iso", 3, false).unwrap();
        assert_eq!(
            format_iso_like(&dt, &spec, false, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.023+02:00"
        );
        // 毫秒为 0 → 不输出分数
        let dt0 = d(2010, 5, 15, 20, 38, 5, 0);
        assert_eq!(
            format_iso_like(&dt0, &spec, false, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05+02:00"
        );
        // ms 强制 3 位
        let spec_ms = parse_iso_params("iso ms", 3, false).unwrap();
        let dt10 = d(2010, 5, 15, 20, 38, 5, 10);
        assert_eq!(
            format_iso_like(&dt10, &spec_ms, false, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.010+02:00"
        );
        // 最小位数
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dt10, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.01+02:00"
        );
        let dt100 = d(2010, 5, 15, 20, 38, 5, 100);
        assert_eq!(
            format_iso_like(&dt100, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.1+02:00"
        );
    }

    #[test]
    fn format_date_only_variants() {
        let dt = d(2010, 5, 15, 20, 38, 5, 23);
        let mut dd = dt.clone();
        dd.kind = DateType::Date;
        // XS date-only 默认带偏移（非 SQL），iso 恒不带
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15+02:00"
        );
        let spec_fz = parse_iso_params("xs_fz", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec_fz, true, &tz_gmt2()).unwrap(),
            "2010-05-15+02:00"
        );
        let spec_iso = parse_iso_params("iso", 3, false).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec_iso, false, &tz_gmt2()).unwrap(),
            "2010-05-15"
        );
        let spec_iso_fz = parse_iso_params("iso_fz", 3, false).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec_iso_fz, false, &tz_gmt2()).unwrap(),
            "2010-05-15"
        );
        // SQL date：xs 默认不带偏移，xs_fz 带
        dd.is_sql = true;
        assert_eq!(
            format_iso_like(&dd, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15"
        );
        assert_eq!(
            format_iso_like(&dd, &spec_fz, true, &tz_gmt2()).unwrap(),
            "2010-05-15+02:00"
        );
    }

    #[test]
    fn format_time_only_variants() {
        let t = DateValue {
            dt: DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(1970, 1, 1)
                    .unwrap()
                    .and_hms_milli_opt(20, 38, 5, 23)
                    .unwrap(),
                Utc,
            )
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
            kind: DateType::Time,
            is_sql: true,
        };
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&t, &spec, true, &tz_gmt2()).unwrap(),
            "22:38:05.023"
        );
        let spec_fz = parse_iso_params("xs_fz", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&t, &spec_fz, true, &tz_gmt2()).unwrap(),
            "22:38:05.023+02:00"
        );
        // 非 SQL 时间默认带偏移
        let mut t2 = t.clone();
        t2.is_sql = false;
        assert_eq!(
            format_iso_like(&t2, &spec, true, &tz_gmt2()).unwrap(),
            "22:38:05.023+02:00"
        );
    }

    #[test]
    fn parse_iso_and_xs() {
        let tz = tz_gmt2();
        let spec = parse_iso_params("xs", 2, true).unwrap();
        // 扩展
        let v = parse_iso_like(
            "2010-05-15T22:38:05.023+02:00",
            DateType::DateTime,
            &spec,
            &tz,
            true,
        )
        .unwrap();
        assert_eq!(v.dt, d(2010, 5, 15, 20, 38, 5, 23).dt);
        // 无时区 → 默认时区
        let v = parse_iso_like(
            "2010-05-15T22:38:05.023",
            DateType::DateTime,
            &spec,
            &tz,
            true,
        )
        .unwrap();
        assert_eq!(v.dt, d(2010, 5, 15, 20, 38, 5, 23).dt);
        // ISO 基本形式 + 逗号小数 + +01 偏移
        let spec_iso = parse_iso_params("iso", 3, false).unwrap();
        let v = parse_iso_like(
            "19981030T153044,512+01",
            DateType::DateTime,
            &spec_iso,
            &tz,
            false,
        )
        .unwrap();
        assert_eq!(v.dt, d(1998, 10, 30, 14, 30, 44, 512).dt);
        // 基本日期（GMT+02 默认时区 → 1998-10-29T22:00Z）
        let v = parse_iso_like("19981030", DateType::Date, &spec_iso, &tz, false).unwrap();
        assert_eq!(
            v.dt.with_timezone(&Utc),
            d(1998, 10, 29, 22, 0, 0, 0).dt.with_timezone(&Utc)
        );
        // XS 日期带 Z
        let v = parse_iso_like("1998-10-30Z", DateType::Date, &spec, &tz, true).unwrap();
        assert_eq!(v.dt, d(1998, 10, 30, 0, 0, 0, 0).dt);
        // 时间基本形式
        let v = parse_iso_like("153044,512Z", DateType::Time, &spec_iso, &tz, false).unwrap();
        assert_eq!(v.dt, d(1970, 1, 1, 15, 30, 44, 512).dt);
    }

    #[test]
    fn julian_year_600() {
        // Java GregorianCalendar：600-01-01（儒略历）→ 公历 0600-01-03
        let v = fields_to_date(
            600,
            1,
            1,
            23,
            59,
            59,
            123,
            FixedOffset::east_opt(0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            v.dt.format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
            "0600-01-03T23:59:59.123"
        );
        // 与 Java 基准一致：?iso_utc_ms → 0600-01-03T23:59:59.123Z（UTC）
        let spec = parse_iso_params("iso ms", 3, false).unwrap();
        let utc = TzSetting::Fixed(FixedOffset::east_opt(0).unwrap());
        assert_eq!(
            format_iso_like(&v, &spec, false, &utc).unwrap(),
            "0600-01-03T23:59:59.123Z"
        );
    }
}
