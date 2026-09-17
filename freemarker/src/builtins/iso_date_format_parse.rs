//! ISO 8601/XS 日期解析与日历换算 —— 自 iso_date_format.rs 拆出。
//! （对应 ISOLikeTemplateDateFormat.parse 与 DateUtil 的字段解析、儒略日换算。）

use super::IsoSpec;
use crate::core::TzSetting;
use crate::error::{Result, TemplateError};
use crate::value::{DateType, DateValue};
use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};

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
pub(crate) fn fields_to_date(
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
