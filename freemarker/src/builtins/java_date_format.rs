//! Java 日期格式（SimpleDateFormat 子集）—— 对应 Java `JavaTemplateDateFormat.java` +
//! `JavaTemplateDateFormatFactory.java`（命名模式 short/medium/long/full 按 `_` 拆分，
//! :66-146）+ `java.text.SimpleDateFormat` 的常用模式子集。
//!
//! v1 支持的模式元素（Java SimpleDateFormat 字母）：`yyyy`/`yy`/`y`/`yyyyy`（年）、
//! `MM`/`M`/`MMM`/`MMMM`（月）、`dd`/`d`（日）、`HH`/`H`（24 小时）、`hh`/`h`（12 小时 +
//! `a` AM/PM）、`mm`/`m`、`ss`/`s`、`SSS`/`S`（毫秒）、`E`/`EEE`/`EEEE`（星期）、
//! `z`（GMT±HH:MM）、`Z`（±HHMM）、`X`/`XX`/`XXX`（ISO 偏移）、`G`（AD/BC 纪元）、
//! `'...'` 引号字面量；其余字符原样输出/匹配。不支持的元素 → InvalidFormatParametersException。

use crate::error::{Result, TemplateError};
use crate::value::{DateType, DateValue};
use chrono::{DateTime, Datelike, FixedOffset, Timelike, Utc};

/// 解析命名模式（"short"/"medium"/"long"/"full" 及 "date_time" 组合，`_` 拆分；
/// JavaTemplateDateFormatFactory.parseDateStyleToken :140-146；空串 → DEFAULT=medium）。
/// 返回具体模式串；含非风格 token 则原样返回（视为模式）。
pub fn resolve_named_style(
    name_or_pattern: &str,
    date_type: DateType,
    locale: &str,
) -> Option<String> {
    // 空串：无风格 token → DateFormat.DEFAULT（= MEDIUM）；Java StringTokenizer 无 token
    if name_or_pattern.is_empty() {
        return Some(default_pattern(date_type, locale));
    }
    let mut toks = name_or_pattern.split('_');
    let style1 = parse_style_token(toks.next().unwrap_or(""));
    let style2 = toks.next().and_then(parse_style_token);
    // 全部 token 均为风格名 → 命名模式；否则是普通模式串
    if style1.is_none() || (style2.is_none() && toks.next().is_some()) {
        return None;
    }
    let style1 = style1.unwrap_or(0); // DEFAULT = MEDIUM
    let pattern = match date_type {
        DateType::Date => date_pattern(style1, locale),
        DateType::Time => time_pattern(style1, locale),
        DateType::DateTime | DateType::Unknown => {
            let style2 = style2.unwrap_or(style1);
            format!(
                "{} {}",
                date_pattern(style1, locale),
                time_pattern(style2, locale)
            )
        }
    };
    Some(pattern)
}

/// 空格式串的默认模式（DateFormat.DEFAULT = MEDIUM）
fn default_pattern(date_type: DateType, locale: &str) -> String {
    match date_type {
        DateType::Date => date_pattern(0, locale),
        DateType::Time => time_pattern(0, locale),
        DateType::DateTime | DateType::Unknown => {
            format!("{} {}", date_pattern(0, locale), time_pattern(0, locale))
        }
    }
}

fn parse_style_token(token: &str) -> Option<i32> {
    match token {
        "short" => Some(1),  // SHORT
        "medium" => Some(2), // MEDIUM
        "long" => Some(3),   // LONG
        "full" => Some(4),   // FULL
        _ => None,
    }
}

/// 命名模式的 locale 模式表（JDK DateFormat 各 locale 的 getDateInstance/getTimeInstance；
/// v1 支持套件用到的 en_US 与 hu_hu，其余 locale 回退 en_US）
fn date_pattern(style: i32, locale: &str) -> String {
    let lang = locale.split('_').next().unwrap_or("en");
    if lang == "hu" {
        match style {
            1 => "yyyy. M. d.".to_string(),
            2 | 0 => "yyyy. MMM d.".to_string(),
            3 => "yyyy. MMMM d.".to_string(),
            _ => "yyyy. MMMM d., EEEE".to_string(),
        }
    } else {
        match style {
            1 => "M/d/yy".to_string(),
            2 | 0 => "MMM d, yyyy".to_string(), // DEFAULT = MEDIUM
            3 => "MMMM d, yyyy".to_string(),
            _ => "EEEE, MMMM d, yyyy".to_string(),
        }
    }
}

fn time_pattern(style: i32, locale: &str) -> String {
    let lang = locale.split('_').next().unwrap_or("en");
    if lang == "hu" {
        match style {
            1 => "H:mm".to_string(),
            2 | 0 => "H:mm:ss".to_string(),
            3 => "H:mm:ss z".to_string(),
            _ => "H:mm:ss zzzz".to_string(),
        }
    } else {
        match style {
            1 => "h:mm a".to_string(),
            2 | 0 => "h:mm:ss a".to_string(), // DEFAULT = MEDIUM
            3 => "h:mm:ss a z".to_string(),
            _ => "h:mm:ss a zzzz".to_string(),
        }
    }
}

/// 模式元素（格式字母 + 长度）
#[derive(Debug, Clone, PartialEq)]
enum Fld {
    Lit(String),
    Year(usize),
    Month(usize),
    Day(usize),
    Hour24(usize),
    Hour12(usize),
    Minute(usize),
    Second(usize),
    Millis(usize),
    AmPm,
    Weekday(usize),
    ZoneZ,   // z
    ZoneRFC, // Z
    ZoneX(usize),
    Era,
}

/// 模式 → 元素序列（SimpleDateFormat 模式扫描；不支持的模式字母报错）
fn scan_pattern(pattern: &str) -> Result<Vec<Fld>> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            // 引号字面量（'' 表示单个 '）
            let mut lit = String::new();
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        lit.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                lit.push(chars[i]);
                i += 1;
            }
            out.push(Fld::Lit(lit));
            continue;
        }
        if c.is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < chars.len() && chars[j] == c {
                j += 1;
            }
            let len = j - i;
            let f = match c {
                'y' => Fld::Year(len),
                'M' => Fld::Month(len),
                'd' => Fld::Day(len),
                'H' => Fld::Hour24(len),
                'h' => Fld::Hour12(len),
                'm' => Fld::Minute(len),
                's' => Fld::Second(len),
                'S' => Fld::Millis(len),
                'a' => Fld::AmPm,
                'E' => Fld::Weekday(len),
                'z' => Fld::ZoneZ,
                'Z' => Fld::ZoneRFC,
                'X' => Fld::ZoneX(len),
                'G' => Fld::Era,
                other => {
                    return Err(TemplateError::misc(format!(
                        "Illegal pattern character '{other}'"
                    )))
                }
            };
            out.push(f);
            i = j;
        } else {
            // 非字母 → 字面量
            let mut lit = String::new();
            while i < chars.len() && !chars[i].is_ascii_alphabetic() && chars[i] != '\'' {
                lit.push(chars[i]);
                i += 1;
            }
            out.push(Fld::Lit(lit));
        }
    }
    Ok(out)
}

// ---- locale 文本表（月份/星期/AMPM；v1 支持 en_US 与 hu_hu）----

fn month_names(locale: &str, short: bool) -> Vec<&'static str> {
    if locale.split('_').next().unwrap_or("en") == "hu" {
        if short {
            vec![
                "jan.", "febr.", "márc.", "ápr.", "máj.", "jún.", "júl.", "aug.", "szept.", "okt.",
                "nov.", "dec.",
            ]
        } else {
            vec![
                "január",
                "február",
                "március",
                "április",
                "május",
                "június",
                "július",
                "augusztus",
                "szeptember",
                "október",
                "november",
                "december",
            ]
        }
    } else if short {
        vec![
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ]
    } else {
        vec![
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ]
    }
}

fn weekday_names(locale: &str, full: bool) -> Vec<&'static str> {
    // 索引：0=Monday（ISO）——转 SimpleDateFormat 的 1=Sunday 语义由调用处处理
    if locale.split('_').next().unwrap_or("en") == "hu" {
        if full {
            vec![
                "hétfő",
                "kedd",
                "szerda",
                "csütörtök",
                "péntek",
                "szombat",
                "vasárnap",
            ]
        } else {
            vec!["h", "k", "sze", "cs", "p", "szo", "v"]
        }
    } else if full {
        vec![
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ]
    } else {
        vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    }
}

fn ampm_names(locale: &str) -> (&'static str, &'static str) {
    if locale.split('_').next().unwrap_or("en") == "hu" {
        ("de", "du")
    } else {
        ("AM", "PM")
    }
}

/// 日期格式化（JavaTemplateDateFormat.formatToPlainText；env 时区）
pub fn format_java(
    pattern: &str,
    d: &DateValue,
    locale: &str,
    env_tz: &crate::core::TzSetting,
) -> Result<String> {
    let fields = scan_pattern(pattern)?;
    let utc: DateTime<Utc> = d.dt.with_timezone(&Utc);
    let local = utc.with_timezone(&env_tz.offset_at(&utc.naive_utc()));
    let mut out = String::new();
    let month_short = month_names(locale, true);
    let month_full = month_names(locale, false);
    let wd_short = weekday_names(locale, false);
    let wd_full = weekday_names(locale, true);
    let (am, pm) = ampm_names(locale);
    let hour12 = {
        let h = local.hour();
        let h12 = h % 12;
        if h12 == 0 {
            12
        } else {
            h12
        }
    };
    for f in &fields {
        match f {
            Fld::Lit(s) => out.push_str(s),
            Fld::Year(n) => {
                let y = local.year();
                let s = y.abs().to_string();
                match n {
                    1..=2 => {
                        // yy → 两位年份（Java SimpleDateFormat 2 位：截断）
                        let two = if s.len() >= 2 { &s[s.len() - 2..] } else { &s };
                        out.push_str(two);
                    }
                    5 => out.push_str(&format!("{y:05}")),
                    _ => out.push_str(&format!("{y:04}")),
                }
            }
            Fld::Month(n) => match n {
                3 => out.push_str(month_short[(local.month() - 1) as usize]),
                4 => out.push_str(month_full[(local.month() - 1) as usize]),
                // 1 位（M）不补零；≥2 位补零（SimpleDateFormat）
                _ => out.push_str(&format!(
                    "{:0width$}",
                    local.month(),
                    width = if *n == 1 { 1 } else { 2 }
                )),
            },
            Fld::Day(n) => {
                out.push_str(&format!(
                    "{:0width$}",
                    local.day(),
                    width = if *n == 1 { 1 } else { 2 }
                ));
            }
            Fld::Hour24(n) => {
                out.push_str(&format!(
                    "{:0width$}",
                    local.hour(),
                    width = if *n == 1 { 1 } else { 2 }
                ));
            }
            Fld::Hour12(n) => {
                out.push_str(&format!(
                    "{hour12:0width$}",
                    width = if *n == 1 { 1 } else { 2 }
                ));
            }
            Fld::Minute(n) => {
                out.push_str(&format!(
                    "{:0width$}",
                    local.minute(),
                    width = if *n == 1 { 1 } else { 2 }
                ));
            }
            Fld::Second(n) => {
                out.push_str(&format!(
                    "{:0width$}",
                    local.second(),
                    width = if *n == 1 { 1 } else { 2 }
                ));
            }
            Fld::Millis(n) => {
                let ms = local.timestamp_subsec_millis();
                if *n == 1 {
                    // S：最少位数（去尾零）
                    let s = ms.to_string();
                    out.push_str(s.trim_end_matches('0'));
                    if s.trim_end_matches('0').is_empty() && ms == 0 {
                        out.push('0');
                    }
                } else {
                    out.push_str(&format!("{ms:0width$}", width = n));
                }
            }
            Fld::AmPm => out.push_str(if local.hour() < 12 { am } else { pm }),
            Fld::Weekday(n) => {
                // chrono weekday(): Mon=0..Sun=6；SimpleDateFormat E 用 Sun=1..Sat=7
                let wd = local.weekday().num_days_from_monday() as usize;
                if *n <= 3 {
                    out.push_str(wd_short[wd]);
                } else {
                    out.push_str(wd_full[wd]);
                }
            }
            Fld::ZoneZ => {
                let off = local.offset().local_minus_utc();
                if off == 0 {
                    out.push_str("GMT");
                } else {
                    let sign = if off < 0 { '-' } else { '+' };
                    let s = off.unsigned_abs();
                    out.push_str(&format!("GMT{sign}{:02}:{:02}", s / 3600, (s % 3600) / 60));
                }
            }
            Fld::ZoneRFC => {
                let off = local.offset().local_minus_utc();
                let sign = if off < 0 { '-' } else { '+' };
                let s = off.unsigned_abs();
                out.push_str(&format!("{sign}{:02}{:02}", s / 3600, (s % 3600) / 60));
            }
            Fld::ZoneX(n) => {
                let off = local.offset().local_minus_utc();
                if off == 0 {
                    out.push('Z');
                } else {
                    let sign = if off < 0 { '-' } else { '+' };
                    let s = off.unsigned_abs();
                    let h = s / 3600;
                    let m = (s % 3600) / 60;
                    match n {
                        1 => out.push_str(&format!("{sign}{h:02}")),
                        2 => out.push_str(&format!("{sign}{h:02}{m:02}")),
                        _ => out.push_str(&format!("{sign}{h:02}:{m:02}")),
                    }
                }
            }
            Fld::Era => out.push_str(if local.year() >= 0 { "AD" } else { "BC" }),
        }
    }
    Ok(out)
}

/// 日期解析（JavaTemplateDateFormat.parse；SimpleDateFormat 子集的宽松位置匹配）
pub fn parse_java(
    pattern: &str,
    s: &str,
    date_type: DateType,
    locale: &str,
    env_tz: &crate::core::TzSetting,
) -> Result<DateValue> {
    let fields = scan_pattern(pattern)?;
    let mut pos = 0usize;
    let mut year: Option<i64> = None;
    let mut month = 1i64;
    let mut day = 1i64;
    let mut hour = 0i64;
    let mut minute = 0i64;
    let mut second = 0i64;
    let mut millis = 0i64;
    let mut ampm: Option<bool> = None;
    let mut bc_era = false;
    let mut zone: Option<FixedOffset> = None;
    let chars: Vec<char> = s.chars().collect();
    let month_short = month_names(locale, true);
    let month_full = month_names(locale, false);
    let wd_short = weekday_names(locale, false);
    let wd_full = weekday_names(locale, true);
    let (am, pm) = ampm_names(locale);

    for f in &fields {
        match f {
            Fld::Lit(lit) => {
                // 字面量：模式中的空白（未引号）匹配任意空白；非空白字面量跳过前导空白后匹配
                // （SimpleDateFormat 语义）
                if !lit.chars().all(|c| c.is_whitespace()) {
                    skip_ws(&chars, &mut pos);
                    if !s[pos..].starts_with(lit.as_str()) {
                        return Err(parse_err(pattern, s));
                    }
                    pos += lit.len();
                } else {
                    skip_ws(&chars, &mut pos);
                }
            }
            Fld::Year(n) => {
                skip_ws(&chars, &mut pos);
                let start = pos;
                let max = if *n <= 2 { 2 } else { 5 };
                while pos < chars.len() && pos - start < max && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                if pos == start {
                    return Err(parse_err(pattern, s));
                }
                let v: i64 = s[start..pos].parse().map_err(|_| parse_err(pattern, s))?;
                year = Some(if *n <= 2 {
                    // SimpleDateFormat 2 位年：<80 → 2000s，否则 1900s（Java 语义）
                    if v < 80 {
                        2000 + v
                    } else {
                        1900 + v
                    }
                } else {
                    v
                });
            }
            Fld::Month(n) => {
                skip_ws(&chars, &mut pos);
                if *n >= 3 {
                    // 文本月份
                    let names: Vec<&str> = if *n == 3 {
                        month_short.clone()
                    } else {
                        month_full.clone()
                    };
                    let rest: String = chars[pos..].iter().take(12).collect();
                    let mut found = false;
                    for (idx, name) in names.iter().enumerate() {
                        if rest.to_lowercase().starts_with(&name.to_lowercase()) {
                            month = idx as i64 + 1;
                            pos += name.chars().count();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(parse_err(pattern, s));
                    }
                } else {
                    let v = parse_digits(&chars, &mut pos, 2)?;
                    month = v.ok_or_else(|| parse_err(pattern, s))?;
                }
            }
            Fld::Day(_) => {
                skip_ws(&chars, &mut pos);
                let v = parse_digits(&chars, &mut pos, 2)?;
                day = v.ok_or_else(|| parse_err(pattern, s))?;
            }
            Fld::Hour24(_) | Fld::Hour12(_) => {
                skip_ws(&chars, &mut pos);
                let v = parse_digits(&chars, &mut pos, 2)?;
                hour = v.ok_or_else(|| parse_err(pattern, s))?;
            }
            Fld::Minute(_) => {
                skip_ws(&chars, &mut pos);
                let v = parse_digits(&chars, &mut pos, 2)?;
                minute = v.ok_or_else(|| parse_err(pattern, s))?;
            }
            Fld::Second(_) => {
                skip_ws(&chars, &mut pos);
                let v = parse_digits(&chars, &mut pos, 2)?;
                second = v.ok_or_else(|| parse_err(pattern, s))?;
            }
            Fld::Millis(_) => {
                skip_ws(&chars, &mut pos);
                // S：1-3 位数字按毫秒
                let v = parse_digits(&chars, &mut pos, 3)?;
                millis = v.ok_or_else(|| parse_err(pattern, s))?;
            }
            Fld::AmPm => {
                skip_ws(&chars, &mut pos);
                let rest: String = chars[pos..].iter().take(2).collect();
                if rest.eq_ignore_ascii_case(am) {
                    ampm = Some(false);
                    pos += am.chars().count();
                } else if rest.eq_ignore_ascii_case(pm) {
                    ampm = Some(true);
                    pos += pm.chars().count();
                } else {
                    return Err(parse_err(pattern, s));
                }
            }
            Fld::Weekday(n) => {
                // 星期名仅跳过（SimpleDateFormat 解析时忽略）
                skip_ws(&chars, &mut pos);
                let names: Vec<&str> = if *n <= 3 {
                    wd_short.clone()
                } else {
                    wd_full.clone()
                };
                let rest: String = chars[pos..].iter().take(12).collect();
                let mut matched = false;
                for name in names {
                    if rest.to_lowercase().starts_with(&name.to_lowercase()) {
                        pos += name.chars().count();
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return Err(parse_err(pattern, s));
                }
            }
            Fld::ZoneZ => {
                skip_ws(&chars, &mut pos);
                let rest = &s[pos..];
                // z 模式：GMT / GMT±HH:MM / GMT±HHMM（SimpleDateFormat z 的 GMT 前缀形式）
                if rest.to_uppercase().starts_with("GMT") {
                    let after = &rest[3..];
                    let before = pos;
                    pos += 3;
                    let off = parse_gmt_offset(after, &mut pos)?;
                    zone = off.or(Some(FixedOffset::east_opt(0).unwrap()));
                    if pos == before + 3 && rest.len() > 3 {
                        return Err(parse_err(pattern, s));
                    }
                } else if let Some(z) = parse_plain_offset(rest) {
                    zone = Some(z);
                    pos += 5;
                } else {
                    return Err(parse_err(pattern, s));
                }
            }
            Fld::ZoneRFC => {
                skip_ws(&chars, &mut pos);
                let z = parse_plain_offset(&s[pos..]).ok_or_else(|| parse_err(pattern, s))?;
                zone = Some(z);
                pos += 5;
            }
            Fld::ZoneX(n) => {
                skip_ws(&chars, &mut pos);
                let rest = &s[pos..];
                if rest.starts_with('Z') {
                    zone = Some(FixedOffset::east_opt(0).unwrap());
                    pos += 1;
                } else {
                    let sign: i32 = if rest.starts_with('-') { -1 } else { 1 };
                    let body = &rest[1..];
                    let h: i32 = body
                        .get(..2)
                        .and_then(|x| x.parse().ok())
                        .ok_or_else(|| parse_err(pattern, s))?;
                    let m: i32 = if body.len() >= 5 && body.as_bytes()[2] == b':' {
                        body.get(3..5).and_then(|x| x.parse().ok()).unwrap_or(0)
                    } else if body.len() >= 4 {
                        body.get(2..4).and_then(|x| x.parse().ok()).unwrap_or(0)
                    } else {
                        0
                    };
                    zone = Some(FixedOffset::east_opt(sign * (h * 3600 + m * 60)).unwrap());
                    let n = *n;
                    pos += 1 + if n >= 3 && body.as_bytes().get(2) == Some(&b':') {
                        5
                    } else if n >= 2 && body.len() >= 4 {
                        4
                    } else {
                        2
                    };
                }
            }
            Fld::Era => {
                skip_ws(&chars, &mut pos);
                let rest: String = chars[pos..].iter().take(2).collect();
                if rest.eq_ignore_ascii_case("AD") {
                    pos += 2;
                } else if rest.eq_ignore_ascii_case("BC") {
                    bc_era = true;
                    pos += 2;
                } else {
                    return Err(parse_err(pattern, s));
                }
            }
        }
    }
    // 解析结束：允许尾部空白
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }
    if pos != chars.len() {
        return Err(parse_err(pattern, s));
    }

    if let Some(ap) = ampm {
        if hour > 12 {
            return Err(parse_err(pattern, s));
        }
        if ap && hour != 12 {
            hour += 12;
        } else if !ap && hour == 12 {
            hour = 0;
        }
    }
    let off = match zone {
        Some(z) => z,
        None => env_tz.offset_at(&Utc::now().naive_utc()),
    };
    let mut year = year.unwrap_or(1970);
    if bc_era {
        // BC n → 儒略年 1-n（BC 1 = 儒略年 0；Java GregorianCalendar ERA/YEAR 语义）
        year = 1 - year;
    }
    // 年份 <1582 儒略历换算（与 iso_date_format 同语义）；时刻 = 字段(UTC) − 偏移
    let utc = crate::builtins::iso_date_format::fields_to_utc(
        year,
        month as u32,
        day as u32,
        hour as u32,
        minute as u32,
        second as u32,
        millis as u32,
    )?;
    let d = DateValue::new((utc - off).with_timezone(&off), date_type);
    Ok(d)
}

fn parse_err(pattern: &str, s: &str) -> TemplateError {
    TemplateError::misc(format!(
        "The string doesn't match the expected date/time/date-time format. The string to parse was: \"{s}\". The expected format was: \"{pattern}\"."
    ))
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_digits(chars: &[char], pos: &mut usize, max: usize) -> Result<Option<i64>> {
    skip_ws(chars, pos);
    let start = *pos;
    let mut n = 0usize;
    while *pos < chars.len() && n < max && chars[*pos].is_ascii_digit() {
        *pos += 1;
        n += 1;
    }
    if n == 0 {
        return Ok(None);
    }
    let v: i64 = chars[start..*pos]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| TemplateError::misc("The date part is a malformed integer."))?;
    Ok(Some(v))
}

/// ±HHMM（Z 模式 / RFC822 偏移）
fn parse_plain_offset(s: &str) -> Option<FixedOffset> {
    let b = s.as_bytes();
    if b.len() < 5 || (b[0] != b'+' && b[0] != b'-') {
        return None;
    }
    let sign: i32 = if b[0] == b'-' { -1 } else { 1 };
    let h: i32 = s[1..3].parse().ok()?;
    let m: i32 = s[3..5].parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(FixedOffset::east_opt(sign * (h * 3600 + m * 60)).unwrap())
}
/// "GMT±HH:MM" / "GMT±HHMM" / "GMT"（z 模式）
fn parse_gmt_offset(after: &str, pos: &mut usize) -> Result<Option<FixedOffset>> {
    let b = after.as_bytes();
    if b.is_empty() || (b[0] != b'+' && b[0] != b'-') {
        return Ok(None); // 裸 GMT
    }
    let sign: i32 = if b[0] == b'-' { -1 } else { 1 };
    if b.len() < 5 {
        return Ok(None);
    }
    let h: i32 = after[1..3]
        .parse()
        .map_err(|_| TemplateError::misc("The offset-hours part is a malformed integer."))?;
    let (m, consumed) = if b.len() >= 6 && b[3] == b':' {
        (
            after[4..6].parse::<i32>().map_err(|_| {
                TemplateError::misc("The offset-minutes part is a malformed integer.")
            })?,
            6,
        )
    } else if b.len() >= 5 && b[3].is_ascii_digit() {
        (
            after[3..5].parse::<i32>().map_err(|_| {
                TemplateError::misc("The offset-minutes part is a malformed integer.")
            })?,
            5,
        )
    } else {
        (0, 3)
    };
    *pos += consumed;
    Ok(Some(
        FixedOffset::east_opt(sign * (h * 3600 + m * 60)).unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TzSetting;
    use chrono::NaiveDate;
    use chrono::TimeZone as _; // from_utc_datetime

    fn gmt() -> TzSetting {
        TzSetting::Fixed(FixedOffset::east_opt(0).unwrap())
    }

    fn d2002() -> DateValue {
        let naive = NaiveDate::from_ymd_opt(2002, 11, 15)
            .unwrap()
            .and_hms_opt(14, 54, 13)
            .unwrap();
        DateValue::new(
            Utc.from_utc_datetime(&naive)
                .with_timezone(&FixedOffset::east_opt(0).unwrap()),
            DateType::DateTime,
        )
    }

    #[test]
    fn named_styles_en_us() {
        // dateformat-java.txt 期望值（2002-11-15 14:54:13 GMT）
        let d = d2002();
        assert_eq!(
            format_java(
                &resolve_named_style("", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "Nov 15, 2002 2:54:13 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "11/15/02 2:54 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("medium", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "Nov 15, 2002 2:54:13 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("long", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "November 15, 2002 2:54:13 PM GMT"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short_medium", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "11/15/02 2:54:13 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short_long", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "11/15/02 2:54:13 PM GMT"
        );
        // date-only / time-only
        assert_eq!(
            format_java(
                &resolve_named_style("medium", DateType::Date, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "Nov 15, 2002"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short", DateType::Time, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "2:54 PM"
        );
        // hu_hu（dateformat-java.txt：long_long）
        assert_eq!(
            format_java(
                &resolve_named_style("long_long", DateType::DateTime, "hu_hu").unwrap(),
                &d,
                "hu_hu",
                &gmt()
            )
            .unwrap(),
            "2002. november 15. 14:54:13 GMT"
        );
    }

    #[test]
    fn java_patterns() {
        let d = d2002();
        assert_eq!(
            format_java("EEE, dd MMM yyyyy HH:mm:ss z", &d, "en_US", &gmt()).unwrap(),
            "Fri, 15 Nov 02002 14:54:13 GMT"
        );
        assert_eq!(
            format_java("EEE, dd MMM yyyy HH:mm:ss z", &d, "en_US", &gmt()).unwrap(),
            "Fri, 15 Nov 2002 14:54:13 GMT"
        );
        assert_eq!(format_java("yyyy", &d, "en_US", &gmt()).unwrap(), "2002");
        assert_eq!(format_java("MM", &d, "en_US", &gmt()).unwrap(), "11");
    }

    #[test]
    fn parse_java_patterns() {
        // dateparsing：'AD 1998-10-30 19:30:44.512 +0400'?datetime（G yyyy-MM-dd HH:mm:ss.S Z）
        let d = parse_java(
            "G yyyy-MM-dd HH:mm:ss.S Z",
            "AD 1998-10-30 19:30:44.512 +0400",
            DateType::DateTime,
            "en_US",
            &gmt(),
        )
        .unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string(),
            "1998-10-30 15:30:44.512"
        );
        // '10/30/1998 19:30:44:512 GMT+04:00'?datetime("MM/dd/yyyy HH:mm:ss:S z")
        let d = parse_java(
            "MM/dd/yyyy HH:mm:ss:S z",
            "10/30/1998 19:30:44:512 GMT+04:00",
            DateType::DateTime,
            "en_US",
            &gmt(),
        )
        .unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string(),
            "1998-10-30 15:30:44.512"
        );
        // '2010-05-15 22:38:05:23 +0200'?datetime("yyyy-MM-dd HH:mm:ss:S Z")
        let d = parse_java(
            "yyyy-MM-dd HH:mm:ss:S Z",
            "2010-05-15 22:38:05:23 +0200",
            DateType::DateTime,
            "en_US",
            &gmt(),
        )
        .unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string(),
            "2010-05-15 20:38:05.023"
        );
        // date-only 解析（dateformat-iso-bi-common：GMT+02 时区）
        let tz2 = TzSetting::Fixed(FixedOffset::east_opt(7200).unwrap());
        let d = parse_java("yyyy-MM-dd", "2010-05-15", DateType::Date, "en_US", &tz2).unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            "2010-05-14 22:00:00"
        );
    }
}
