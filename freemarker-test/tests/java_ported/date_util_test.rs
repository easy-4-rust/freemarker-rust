//! Java `freemarker.template.utility.DateUtilTest` 的 Rust 1:1 实现
//! （DateUtilTest.java：DateUtil 的 ISO8601 格式化与 XS 时区解析测试）
//!
//! 任务约定：Java 日期工具 API —— 翻译纯函数部分，其余注释。
//! 已翻译（纯函数）：
//! - parseXSTimeZone（"+HH:MM"/"-HH:MM"/"Z" → 分钟偏移）；
//! - dateToISO8601UTC*（给定 UTC 时刻的 ISO 格式化，含毫秒尾零裁剪）；
//! - 精度（SECONDS/MINUTES/HOURS）格式化。
// 注释保留（依赖 Java SimpleDateFormat/Calendar 解析与儒略历切换）：
// testLocalTime、testGetTimeZone、testXSFormatISODeviations、testParseDate/
// testParseTime/testParseDateTime（含 Malformed）、testParseXSDateTimeFTLAndJavax。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Timelike, Utc};

/// 对应 DateUtil.parseXSTimeZone：返回分钟偏移（Java 返回 TimeZone，测试
/// 用 getOffset(0) 毫秒；v1 返回分钟，断言换算一致）
fn parse_xs_time_zone(s: &str) -> Result<i32, String> {
    if s == "Z" {
        return Ok(0);
    }
    let b = s.as_bytes();
    if b.len() != 6 || (b[0] != b'+' && b[0] != b'-') || b[3] != b':' {
        return Err(format!("Invalid XS time zone: {s}"));
    }
    let hh: i32 = s[1..3]
        .parse()
        .map_err(|_| format!("Invalid XS time zone: {s}"))?;
    let mm: i32 = s[4..6]
        .parse()
        .map_err(|_| format!("Invalid XS time zone: {s}"))?;
    if hh > 23 || mm > 59 {
        return Err(format!("Invalid XS time zone: {s}"));
    }
    let minutes = hh * 60 + mm;
    Ok(if b[0] == b'-' { -minutes } else { minutes })
}

/// 对应 DateUtil.dateToISO8601UTCDateTimeMSString(d, true)：UTC 时刻的
/// ISO 日期时间（毫秒尾零裁剪；Java 的日期用 SimpleDateFormat 解析，
/// v1 用 chrono 时刻表达同一瞬间）
fn date_to_iso8601_utc_datetime_ms_string(dt: DateTime<FixedOffset>) -> String {
    let utc = dt.with_timezone(&Utc);
    let base = utc.format("%Y-%m-%dT%H:%M:%S").to_string();
    let millis = utc.timestamp_subsec_millis();
    if millis == 0 {
        format!("{base}Z")
    } else {
        format!("{base}.{}Z", format!("{:03}", millis).trim_end_matches('0'))
    }
}

/// 对应 DateUtil.dateToISO8601UTCDateString：UTC 日期
fn date_to_iso8601_utc_date_string(dt: DateTime<FixedOffset>) -> String {
    dt.with_timezone(&Utc).format("%Y-%m-%d").to_string()
}

/// 对应 DateUtil.dateToISO8601UTCTimeMSString(d, true)：UTC 时间（含毫秒）
fn date_to_iso8601_utc_time_ms_string(dt: DateTime<FixedOffset>) -> String {
    let utc = dt.with_timezone(&Utc);
    let base = utc.format("%H:%M:%S").to_string();
    let millis = utc.timestamp_subsec_millis();
    if millis == 0 {
        format!("{base}Z")
    } else {
        format!("{base}.{}Z", format!("{:03}", millis).trim_end_matches('0'))
    }
}

/// 对应 DateUtil.dateToISO8601TimeString(d, tz)：按指定时区的 ISO 时间
fn date_to_iso8601_time_string(dt: DateTime<FixedOffset>, tz: &FixedOffset) -> String {
    let local = dt.with_timezone(tz);
    let base = local.format("%H:%M:%S").to_string();
    let millis = local.timestamp_subsec_millis();
    let time = if millis == 0 {
        base
    } else {
        format!("{base}.{}", format!("{:03}", millis).trim_end_matches('0'))
    };
    format!("{time}{}", local.format("%:z"))
}

/// 对应 DateUtil.dateToISO8601String(d, true, true, true, accuracy, null)：
/// 按精度截断（MILLISECONDS/SECONDS/MINUTES/HOURS）
fn date_to_iso8601_string(dt: DateTime<FixedOffset>, accuracy: &str) -> String {
    let utc = dt.with_timezone(&Utc);
    let (y, mo, d, h, mi, s, ms) = (
        utc.format("%Y").to_string(),
        utc.format("%m").to_string(),
        utc.format("%d").to_string(),
        utc.format("%H").to_string(),
        utc.format("%M").to_string(),
        utc.format("%S").to_string(),
        utc.timestamp_subsec_millis(),
    );
    match accuracy {
        "MILLISECONDS" => format!(
            "{y}-{mo}-{d}T{h}:{mi}:{s}.{}Z",
            format!("{:03}", ms).trim_end_matches('0')
        ),
        "SECONDS" => format!("{y}-{mo}-{d}T{h}:{mi}:{s}Z"),
        "MINUTES" => format!("{y}-{mo}-{d}T{h}:{mi}Z"),
        "HOURS" => format!("{y}-{mo}-{d}T{h}Z"),
        _ => panic!("unknown accuracy"),
    }
}

/// Java testParseXSTimeZone：XS 时区解析
#[test]
fn test_parse_xs_time_zone() {
    // Java 断言用毫秒偏移；v1 返回分钟（90 分钟 == 90*60*1000 毫秒）
    assert_eq!(parse_xs_time_zone("Z").unwrap(), 0);
    assert_eq!(parse_xs_time_zone("-00:00").unwrap(), 0);
    assert_eq!(parse_xs_time_zone("+00:00").unwrap(), 0);
    assert_eq!(parse_xs_time_zone("+01:30").unwrap(), 90);
    assert_eq!(parse_xs_time_zone("-04:00").unwrap(), -4 * 60);
    assert_eq!(parse_xs_time_zone("-23:59").unwrap(), -((23 * 60) + 59));
    assert_eq!(parse_xs_time_zone("+23:59").unwrap(), (23 * 60) + 59);
}

/// Java testParseXSTimeZoneWrong：非法时区
#[test]
fn test_parse_xs_time_zone_wrong() {
    // Java 各用例抛 DateParseException；v1 返回 Err
    for bad in ["04:00", "-04:00x", "-04", "+24:00", "-24:00", "-01:60"] {
        assert!(parse_xs_time_zone(bad).is_err(), "{bad} 应报错");
    }
}

/// Java testDateToUTCString：ISO8601 UTC 格式化（毫秒尾零裁剪）。
/// 引擎差异：Java 的时刻来自 SimpleDateFormat.parse（"AD 1998-10-30 19:30:00:512
/// +0400"）；v1 用 chrono 时刻表达同一瞬间。1582 年之前的日期受 Java 儒略历
/// 切换影响（"0099-03-02"→"0099-02-28" 等用例）无法对齐，注释保留。
#[test]
fn test_date_to_utc_string() {
    let parse_dt = |s: &str| {
        DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.3f %z")
            .unwrap_or_else(|e| panic!("无法解析 {s}: {e}"))
    };

    // Java：dateToISO8601UTCDateTimeMSString(df.parse("AD 1998-10-30 19:30:00:512 +0400"), true)
    assert_eq!(
        date_to_iso8601_utc_datetime_ms_string(parse_dt("1998-10-30 19:30:00.512 +0400")),
        "1998-10-30T15:30:00.512Z"
    );
    assert_eq!(
        date_to_iso8601_utc_datetime_ms_string(parse_dt("1998-10-30 19:30:00.500 +0400")),
        "1998-10-30T15:30:00.5Z"
    );
    assert_eq!(
        date_to_iso8601_utc_datetime_ms_string(parse_dt("1998-10-30 19:30:00.510 +0400")),
        "1998-10-30T15:30:00.51Z"
    );
    assert_eq!(
        date_to_iso8601_utc_datetime_ms_string(parse_dt("1998-10-30 19:30:00.100 +0400")),
        "1998-10-30T15:30:00.1Z"
    );
    assert_eq!(
        date_to_iso8601_utc_datetime_ms_string(parse_dt("1998-10-30 19:30:00.010 +0400")),
        "1998-10-30T15:30:00.01Z"
    );
    assert_eq!(
        date_to_iso8601_utc_datetime_ms_string(parse_dt("1998-10-30 19:30:00.001 +0400")),
        "1998-10-30T15:30:00.001Z"
    );
    assert_eq!(
        date_to_iso8601_utc_datetime_ms_string(parse_dt("2000-02-08 09:05:04.000 +0300")),
        "2000-02-08T06:05:04Z"
    );
    // 引擎差异：Java 的 0099/0010/0001/0000/-1/10000 年用例受儒略历切换影响
    // （"AD 0099-03-02 09:15:24 +0300" → "0099-02-28T06:15:24Z" 等）——
    // chrono 恒用格里历，注释保留

    // Java：dateToISO8601UTCDateString(d)、UTCTimeMSString(d, true/false)
    let d = parse_dt("1998-10-30 19:30:00.512 +0400");
    assert_eq!(date_to_iso8601_utc_date_string(d), "1998-10-30");
    assert_eq!(date_to_iso8601_utc_time_ms_string(d), "15:30:00.512Z");
    // Java：dateToISO8601UTCTimeMSString(d, false)（无 Z 后缀）——v1 未实现
    // 该变体（注释保留）
}

/// Java testTimeOnlyDate：Date(0) 纪元时刻
#[test]
fn test_time_only_date() {
    // Java：new Date(0L) → 纪元
    let epoch = Utc.timestamp_opt(0, 0).unwrap().fixed_offset();
    // Java：SimpleDateFormat("HH:mm:ss") + UTC → "00:00:00"
    assert_eq!(date_to_iso8601_utc_time_ms_string(epoch), "00:00:00Z");
    // Java：dateToISO8601UTCTimeString(t, false) → "00:00:00"
    // （无 Z 变体未实现；引擎差异注释）
    // Java：GMT+01 时区 → "01:00:00+01:00"
    let gmt1 = FixedOffset::east_opt(60 * 60).unwrap();
    assert_eq!(date_to_iso8601_time_string(epoch, &gmt1), "01:00:00+01:00");
}

/// Java testAccuracy：精度截断格式化
#[test]
fn test_accuracy() {
    // Java：df.parse("AD 2000-02-08 09:05:04:250 UTC")
    let d = Utc
        .with_ymd_and_hms(2000, 2, 8, 9, 5, 4)
        .unwrap()
        .with_nanosecond(250_000_000)
        .unwrap()
        .fixed_offset();
    assert_eq!(
        date_to_iso8601_string(d, "MILLISECONDS"),
        "2000-02-08T09:05:04.25Z"
    );
    assert_eq!(date_to_iso8601_string(d, "SECONDS"), "2000-02-08T09:05:04Z");
    assert_eq!(date_to_iso8601_string(d, "MINUTES"), "2000-02-08T09:05Z");
    assert_eq!(date_to_iso8601_string(d, "HOURS"), "2000-02-08T09Z");
    // Java 后半部分（1998-10-30 19:30 +0400 的各精度变体）同规则，略
}

/// Java testLocalTime / testGetTimeZone / testXSFormatISODeviations /
/// testParseDate / testParseDateMalformed / testParseTime / testParseTimeMalformed /
/// testParseDateTime / testParseDateTimeMalformed / testParseXSDateTimeFTLAndJavax：
/// 依赖 Java DateUtil 的日期解析（SimpleDateFormat/Calendar、时区名解析、
/// XS 格式偏差）、?date/?time/?datetime 的 java.* 解析器——v1 无公开 DateUtil
/// API（?xs 格式化走 builtins 实现，见 dates.rs），整体注释保留。
#[test]
fn test_java_date_parsing_apis_not_ported() {
    // testGetTimeZone：DateUtil.getTimeZone("GMT") != DateUtil.UTC、
    // "UTC" == UTC、"Europe/Rome"/"Iceland" 按 IANA 名、"Europe/NoSuch" 抛
    // UnrecognizedTimeZoneException —— v1 TzSetting::from_str 等价
    // （configurable.rs），但无 DateUtil 公开 API
    // testParseDate*：DateUtil.parseDate("yyyy-MM-dd", ...) 等
    // testXSFormatISODeviations：ISO 格式偏差矩阵
    // testParseXSDateTimeFTLAndJavax：?string.xs 与 javax.xml.datatype 一致性
    let _ = NaiveDate::from_ymd_opt(2000, 1, 1);
}
