//! 字符串基础内建 —— 对应 Java `BuiltInsForStringsBasic.java`（本文件为 eval.rs 内建集
//! 未覆盖的子集：capitalize/uncap_first/c_*_case/chop_linebreak/keep_*/remove_*/ensure_*/
//! pad/last_index_of）。?replace/?split/?matches 见 strings_regexp.rs（需要 flags）。
//!
//! 语义要点（Java 对照）：
//! - capitalize → StringUtil.capitalize（按 ` \t\r\n` 分词，每词首字母大写其余小写）；
//! - chop_linebreak → StringUtil.chomp（去掉尾部单个 \r\n/\r/\n）；
//! - keep_* 的 flags：'i' 大小写不敏感（两侧 toLowerCase）；'r' 正则；'m'/'s'/'c'
//!   非正则模式下报错（RegexpHelper.checkOnlyHasNonRegexpFlags strict=true）；
//! - pad 按 UTF-16 码元计数（Java String.length 语义），filling 循环填充；
//! - 目标强制转字符串（EvalUtil.coerceModelToStringOrMarkup：数字/布尔可转）。

use crate::builtins::eval_util::{arg_count, arg_string, check_arg_count, target_string};
use crate::builtins::strings_regexp::{compile_pattern, parse_flags, FlagSet};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::utility::java_trim;

/// 目标字符串（数字/布尔按输出规则强制转换）
fn get_string(env: &mut Environment, target: &Expr) -> Result<String> {
    target_string(env, target)
}

/// ?capitalize —— Java `StringUtil.capitalize`（StringUtil.java:684）：
/// 按 ` \t\r\n` 分词（分隔符保留为独立 token），每 token 首字符大写 + 其余小写
pub fn capitalize(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = get_string(env, target)?;
    let mut out = String::with_capacity(s.len());
    let mut token = String::new();
    let push_token = |tok: &str, out: &mut String| {
        if tok.is_empty() {
            return;
        }
        let mut chars = tok.chars();
        if let Some(c) = chars.next() {
            out.push_str(&c.to_uppercase().collect::<String>());
            out.push_str(&chars.as_str().to_lowercase());
        }
    };
    for c in s.chars() {
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
            push_token(&token, &mut out);
            token.clear();
            out.push(c);
        } else {
            token.push(c);
        }
    }
    push_token(&token, &mut out);
    Ok(Some(TModel::from_scalar(out)))
}

/// ?uncap_first —— 首字母小写（Java BuiltInsForStringsBasic.java:817）
pub fn uncap_first(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = get_string(env, target)?;
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => Ok(Some(TModel::from_scalar(
            c.to_lowercase().collect::<String>() + chars.as_str(),
        ))),
        None => Ok(Some(TModel::from_scalar(String::new()))),
    }
}

/// ?c_lower_case / ?c_upper_case —— 仅 ASCII 的大小写转换（Java BuiltInsForStringsBasic.java:441）
pub fn c_lower_case(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = get_string(env, target)?;
    Ok(Some(TModel::from_scalar(
        s.chars().map(|c| c.to_ascii_lowercase()).collect(),
    )))
}

/// ?c_upper_case
pub fn c_upper_case(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = get_string(env, target)?;
    Ok(Some(TModel::from_scalar(
        s.chars().map(|c| c.to_ascii_uppercase()).collect(),
    )))
}

/// ?chop_linebreak —— Java `StringUtil.chomp`（StringUtil.java:853）：
/// 去掉尾部单个 \r\n/\r/\n
pub fn chop_linebreak(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = get_string(env, target)?;
    let out = if let Some(rest) = s.strip_suffix("\r\n") {
        rest.to_string()
    } else if let Some(rest) = s.strip_suffix('\r') {
        rest.to_string()
    } else if let Some(rest) = s.strip_suffix('\n') {
        rest.to_string()
    } else {
        s
    };
    Ok(Some(TModel::from_scalar(out)))
}

/// 解析 flags + 非正则模式下检查 m/s/c 非法（Java checkOnlyHasNonRegexpFlags strict=true）
fn parse_flags_strict(
    bi: &str,
    args: Option<&[Expr]>,
    env: &mut Environment,
    idx: usize,
) -> Result<FlagSet> {
    let flags = if arg_count(args) > idx {
        parse_flags(&arg_string(env, args, idx)?)?
    } else {
        FlagSet::default()
    };
    flags.check_non_regexp_strict(bi)?;
    Ok(flags)
}

/// 正则重叠扫描取最后匹配（Java `matcher.find(start + 1)` 循环：
/// 从上一匹配 start + 1 字符继续搜索——fancy_regex 的 find_iter 不重叠，
/// 会漏掉从上一匹配内部开始的后续匹配，如 "aaabb" 上 `[ab]{3}`：
/// 非重叠只给 "aaa"@0，Java 重叠扫描给 "aaa"@0/"aab"@1/"abb"@2）
fn last_regexp_match(re: &fancy_regex::Regex, s: &str) -> Option<(usize, usize)> {
    let mut pos = 0;
    let mut last = None;
    while let Ok(Some(m)) = re.find_from_pos(s, pos) {
        last = Some((m.start(), m.end()));
        // 下一扫描起点 = 上一匹配 start 后一个字符（Java find(stopIndex + 1)；
        // 多字节字符按 UTF-8 字符边界前进）
        let next = m.start()
            + s[m.start()..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        if next <= m.start() {
            break; // 字符串尾部的空匹配：无可前进位置
        }
        pos = next;
    }
    last
}

/// ?keep_before(sep[, flags]) —— Java BuiltInsForStringsBasic.java:331：
/// 字面量/正则首次匹配起点之前；未找到 → 整串
fn keep_before_impl(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<String> {
    check_arg_count("keep_before", args, 1, 2)?;
    let s = get_string(env, target)?;
    let sep = arg_string(env, args, 0)?;
    let flags = parse_flags_strict("keep_before", args, env, 1)?;
    let stop = if !flags.regexp {
        if flags.case_insensitive {
            s.to_lowercase().find(&sep.to_lowercase())
        } else {
            s.find(&sep)
        }
    } else if sep.is_empty() {
        Some(0)
    } else {
        let re = compile_pattern(&sep, &flags)?;
        re.find(&s).ok().flatten().map(|m| m.start())
    };
    Ok(match stop {
        Some(i) => s[..i].to_string(),
        None => s,
    })
}

pub fn keep_before(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_scalar(keep_before_impl(
        env, target, args,
    )?)))
}

/// ?keep_before_last(sep[, flags]) —— Java BuiltInsForStringsBasic.java:375
fn keep_before_last_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<String> {
    check_arg_count("keep_before_last", args, 1, 2)?;
    let s = get_string(env, target)?;
    let sep = arg_string(env, args, 0)?;
    let flags = parse_flags_strict("keep_before_last", args, env, 1)?;
    let stop = if !flags.regexp {
        if flags.case_insensitive {
            s.to_lowercase().rfind(&sep.to_lowercase())
        } else {
            s.rfind(&sep)
        }
    } else if sep.is_empty() {
        Some(s.len())
    } else {
        let re = compile_pattern(&sep, &flags)?;
        last_regexp_match(&re, &s).map(|(start, _)| start)
    };
    Ok(match stop {
        Some(i) => s[..i].to_string(),
        None => s,
    })
}

pub fn keep_before_last(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_scalar(keep_before_last_impl(
        env, target, args,
    )?)))
}

/// ?keep_after(sep[, flags]) —— Java BuiltInsForStringsBasic.java:232：
/// 首次匹配终点之后；未找到 → 空串
fn keep_after_impl(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<String> {
    check_arg_count("keep_after", args, 1, 2)?;
    let s = get_string(env, target)?;
    let sep = arg_string(env, args, 0)?;
    let flags = parse_flags_strict("keep_after", args, env, 1)?;
    let start = if !flags.regexp {
        let idx = if flags.case_insensitive {
            s.to_lowercase().find(&sep.to_lowercase())
        } else {
            s.find(&sep)
        };
        idx.map(|i| i + sep.len())
    } else if sep.is_empty() {
        Some(0)
    } else {
        let re = compile_pattern(&sep, &flags)?;
        re.find(&s).ok().flatten().map(|m| m.end())
    };
    Ok(match start {
        Some(i) if i <= s.len() => s[i..].to_string(),
        _ => String::new(),
    })
}

pub fn keep_after(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_scalar(keep_after_impl(
        env, target, args,
    )?)))
}

/// ?keep_after_last(sep[, flags]) —— Java BuiltInsForStringsBasic.java:278
fn keep_after_last_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<String> {
    check_arg_count("keep_after_last", args, 1, 2)?;
    let s = get_string(env, target)?;
    let sep = arg_string(env, args, 0)?;
    let flags = parse_flags_strict("keep_after_last", args, env, 1)?;
    let start = if !flags.regexp {
        let idx = if flags.case_insensitive {
            s.to_lowercase().rfind(&sep.to_lowercase())
        } else {
            s.rfind(&sep)
        };
        idx.map(|i| i + sep.len())
    } else if sep.is_empty() {
        Some(s.len())
    } else {
        let re = compile_pattern(&sep, &flags)?;
        last_regexp_match(&re, &s).map(|(_, end)| end)
    };
    Ok(match start {
        Some(i) if i <= s.len() => s[i..].to_string(),
        _ => String::new(),
    })
}

pub fn keep_after_last(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_scalar(keep_after_last_impl(
        env, target, args,
    )?)))
}

/// ?remove_beginning(prefix) —— Java BuiltInsForStringsBasic.java:499：
/// 精确前缀移除（非正则；无 flags 参数）
pub fn remove_beginning(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("remove_beginning", args, 1, 1)?;
    let s = get_string(env, target)?;
    let pre = arg_string(env, args, 0)?;
    let out = if let Some(rest) = s.strip_prefix(&pre) {
        rest.to_string()
    } else {
        s
    };
    Ok(Some(TModel::from_scalar(out)))
}

/// ?remove_ending(suffix) —— Java BuiltInsForStringsBasic.java:522：
/// 仅在整串以该后缀结束时移除
pub fn remove_ending(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("remove_ending", args, 1, 1)?;
    let s = get_string(env, target)?;
    let suf = arg_string(env, args, 0)?;
    let out = if let Some(rest) = s.strip_suffix(&suf) {
        rest.to_string()
    } else {
        s
    };
    Ok(Some(TModel::from_scalar(out)))
}

/// ?ensure_starts_with(prefix[, replacement[, flags]]) —— Java BuiltInsForStringsBasic.java:146：
/// 若不以 prefix（正则）开头，则把 replacement 拼到前面
pub fn ensure_starts_with(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("ensure_starts_with", args, 1, 3)?;
    let s = get_string(env, target)?;
    let prefix = arg_string(env, args, 0)?;
    let replacement = if arg_count(args) > 1 {
        arg_string(env, args, 1)?
    } else {
        prefix.clone()
    };
    let flags = if arg_count(args) > 2 {
        parse_flags(&arg_string(env, args, 2)?)?
    } else if arg_count(args) > 1 {
        // Java BuiltInsForStringsBasic.java:163-166：2 参数且无显式 flags 时
        // 默认 RE_FLAG_REGEXP（checkedPrefix 按正则前缀解释）
        FlagSet {
            regexp: true,
            ..FlagSet::default()
        }
    } else {
        FlagSet::default()
    };
    let matches = if !flags.regexp {
        let (ls, lp) = if flags.case_insensitive {
            (s.to_lowercase(), prefix.to_lowercase())
        } else {
            (s.clone(), prefix.clone())
        };
        ls.starts_with(&lp)
    } else if prefix.is_empty() {
        true
    } else {
        // Java matcher.lookingAt()：锚定字符串开头（不要求整串匹配）
        let re = compile_pattern(&prefix, &flags)?;
        re.find(&s).ok().flatten().is_some_and(|m| m.start() == 0)
    };
    let out = if matches {
        s
    } else {
        format!("{replacement}{s}")
    };
    Ok(Some(TModel::from_scalar(out)))
}

/// ?ensure_ends_with(suffix) —— Java BuiltInsForStringsBasic.java:123-145：
/// 仅 1 参数（checkMethodArgCount(args, 1)）；endsWith 判定，不匹配 → 追加 suffix
pub fn ensure_ends_with(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("ensure_ends_with", args, 1, 1)?;
    let s = get_string(env, target)?;
    let suffix = arg_string(env, args, 0)?;
    let out = if s.ends_with(&suffix) {
        s
    } else {
        format!("{s}{suffix}")
    };
    Ok(Some(TModel::from_scalar(out)))
}

/// ?left_pad / ?right_pad（Java padBI，BuiltInsForStringsBasic.java；StringUtil.leftPad/rightPad）：
/// 按 UTF-16 码元计数；filling 循环填充（剩余位数取 filling 前几个码元）
fn pad_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    left: bool,
) -> Result<String> {
    check_arg_count(if left { "left_pad" } else { "right_pad" }, args, 1, 2)?;
    let s = get_string(env, target)?;
    let width = crate::builtins::eval_util::arg_number(env, args, 0)?;
    let width = crate::core::eval::trunc_i64(&width)
        .ok_or_else(|| TemplateError::misc("The padding length must be an integer"))?
        .max(0) as usize;
    let s_units: Vec<u16> = s.encode_utf16().collect();
    if width <= s_units.len() {
        return Ok(s);
    }
    let filling: Vec<u16> = if arg_count(args) > 1 {
        let f = arg_string(env, args, 1)?;
        f.encode_utf16().collect()
    } else {
        vec![0x20]
    };
    if filling.is_empty() {
        return Err(TemplateError::misc(
            "The \"filling\" argument can't be 0 length string.",
        ));
    }
    let dif = width - s_units.len();
    // Java StringUtil：leftPad 从 filling 起始循环；rightPad 从 `ln % fln` 继续循环
    // （视被填充串为模式的一部分，StringUtil.rightPad 的 start = ln % fln）
    let start = if left {
        0
    } else {
        s_units.len() % filling.len()
    };
    let mut pad: Vec<u16> = Vec::with_capacity(dif);
    let mut i = 0;
    while pad.len() < dif {
        pad.push(filling[(start + i) % filling.len()]);
        i += 1;
    }
    let pad = String::from_utf16_lossy(&pad);
    Ok(if left {
        format!("{pad}{s}")
    } else {
        format!("{s}{pad}")
    })
}

pub fn left_pad(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_scalar(pad_impl(
        env, target, args, true,
    )?)))
}

pub fn right_pad(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_scalar(pad_impl(
        env, target, args, false,
    )?)))
}

/// ?last_index_of(sub[, fromIndex]) —— Java `index_ofBI(true)`（BuiltInsForStringsBasic.java）：
/// 从 fromIndex（含）向左找；未找到 → -1
pub fn last_index_of(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("last_index_of", args, 1, 2)?;
    let s = get_string(env, target)?;
    let sub = arg_string(env, args, 0)?;
    let from: usize = if arg_count(args) > 1 {
        let n = crate::builtins::eval_util::arg_number(env, args, 1)?;
        crate::core::eval::trunc_i64(&n).unwrap_or(0).max(0) as usize
    } else {
        usize::MAX
    };
    let idx: Option<usize> = if sub.is_empty() {
        // Java String.lastIndexOf("")：fromIndex 处（含）向前限位
        Some(from.min(s.len()))
    } else if from >= s.len() {
        s.rfind(&sub)
    } else {
        // 在 [0..=from] 内查找（char 下标；v1 UTF-16 差异见 docs/05 §3）
        let limit = char_index_limit(&s, from);
        limit.rfind(&sub)
    };
    Ok(Some(TModel::from_number(crate::value::TNumber::from_i64(
        match idx {
            Some(i) => i as i64,
            None => -1,
        },
    ))))
}

/// 截取到第 from 个字符（char 计数；Java 为 UTF-16 下标，v1 近似见 docs/05 §3）
fn char_index_limit(s: &str, from: usize) -> &str {
    match s.char_indices().nth(from) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

/// ?trim —— 与 eval.rs 相同语义（java_trim）；保留在此便于回归测试引用
#[allow(dead_code)]
fn trim_impl(s: &str) -> String {
    java_trim(s).to_string()
}

// ---------------------------------------------------------------------------
// truncate 家族 —— 对应 Java BuiltInsForStringsBasic.java（truncate/truncate_w/
// truncate_c 及其 markup-aware 变体）。_m 变体 v1 暂不支持。
// ---------------------------------------------------------------------------

/// 字符串的 UTF-16 码元数（对应 Java `String.length()`）
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// 从开头取 `max_units` 个 UTF-16 码元，返回截断位置（字节偏移）
fn utf16_cut_point(s: &str, max_units: usize) -> usize {
    let mut total = 0usize;
    for (i, c) in s.char_indices() {
        let cu = c.len_utf16();
        if total + cu > max_units {
            return i;
        }
        total += cu;
        if total == max_units {
            return i + c.len_utf8();
        }
    }
    s.len()
}

/// 核心截断实现（UTF-16 码元计数）：
/// - `max_len <= 0` → 返回空串
/// - 原串 UTF-16 长度 ≤ max_len → 原样返回
/// - `max_len <= terminator_utf16_len` → 仅返回能装下的 terminator 前缀（或空）
/// - 否则：在 `max_len - term_units` 码元处截断 + 拼接 terminator
fn truncate_impl(s: &str, max_len: i64, terminator: &str) -> String {
    if max_len <= 0 {
        return String::new();
    }
    let max_len = max_len as usize;
    let s_units = utf16_len(s);
    if s_units <= max_len {
        return s.to_string();
    }
    let term_units = utf16_len(terminator);
    if max_len <= term_units {
        // terminator 装不下 → 返回空；若正好装下 → 返回 terminator
        if max_len == term_units {
            return terminator.to_string();
        }
        return String::new();
    }
    let keep_units = max_len - term_units;
    let cut = utf16_cut_point(s, keep_units);
    let mut out = String::with_capacity(cut + terminator.len());
    out.push_str(&s[..cut]);
    out.push_str(terminator);
    out
}

/// `?truncate(length)` 或 `?truncate(length, terminator)`
/// 按 UTF-16 码元计数截断；默认 terminator = "..."
pub fn truncate(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("truncate", args, 1, 2)?;
    let s = get_string(env, target)?;
    let max_len = crate::builtins::eval_util::arg_number(env, args, 0)?;
    let max_len = crate::core::eval::trunc_i64(&max_len).unwrap_or(0);
    let terminator = if arg_count(args) > 1 {
        arg_string(env, args, 1)?
    } else {
        "...".to_string()
    };
    Ok(Some(TModel::from_scalar(truncate_impl(
        &s, max_len, &terminator,
    ))))
}

/// `?truncate_w(maxWords)` 或 `?truncate_w(maxWords, terminator)`
/// 按词数截断；默认 terminator = "..."
pub fn truncate_w(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("truncate_w", args, 1, 2)?;
    let s = get_string(env, target)?;
    let max_words = crate::builtins::eval_util::arg_number(env, args, 0)?;
    let max_words = crate::core::eval::trunc_i64(&max_words).unwrap_or(0);
    let terminator = if arg_count(args) > 1 {
        arg_string(env, args, 1)?
    } else {
        "...".to_string()
    };
    if max_words <= 0 {
        return Ok(Some(TModel::from_scalar(String::new())));
    }
    let max_words = max_words as usize;
    // 按空白分词（保留原词间空白信息用于重组）
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() <= max_words {
        return Ok(Some(TModel::from_scalar(s)));
    }
    // 截取前 max_words 个词：找到第 max_words 个词的结尾
    let mut word_count = 0usize;
    let mut in_word = false;
    let mut cut = 0usize;
    for (i, c) in s.char_indices() {
        if c.is_whitespace() {
            if in_word {
                word_count += 1;
                in_word = false;
                cut = i;
                if word_count >= max_words {
                    break;
                }
            }
        } else {
            in_word = true;
        }
    }
    if in_word {
        // 最后一个词
        cut = s.len();
    }
    // 移除尾部空白后追加 terminator
    let trimmed = s[..cut].trim_end();
    let mut out = String::with_capacity(trimmed.len() + terminator.len());
    out.push_str(trimmed);
    out.push_str(&terminator);
    Ok(Some(TModel::from_scalar(out)))
}

/// `?truncate_c(maxChars)` 或 `?truncate_c(maxChars, terminator)`
/// 按 Unicode 字符（char/code point）计数截断；默认 terminator = "..."
pub fn truncate_c(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("truncate_c", args, 1, 2)?;
    let s = get_string(env, target)?;
    let max_chars = crate::builtins::eval_util::arg_number(env, args, 0)?;
    let max_chars = crate::core::eval::trunc_i64(&max_chars).unwrap_or(0);
    let terminator = if arg_count(args) > 1 {
        arg_string(env, args, 1)?
    } else {
        "...".to_string()
    };
    if max_chars <= 0 {
        return Ok(Some(TModel::from_scalar(String::new())));
    }
    let max_chars = max_chars as usize;
    let s_chars = s.chars().count();
    if s_chars <= max_chars {
        return Ok(Some(TModel::from_scalar(s)));
    }
    let term_chars = terminator.chars().count();
    if max_chars <= term_chars {
        if max_chars == term_chars {
            return Ok(Some(TModel::from_scalar(terminator)));
        }
        return Ok(Some(TModel::from_scalar(String::new())));
    }
    let keep_chars = max_chars - term_chars;
    // 找到第 keep_chars 个 Unicode 字符之后的字节偏移
    let cut: usize = s
        .char_indices()
        .nth(keep_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let mut out = String::with_capacity(cut + terminator.len());
    out.push_str(&s[..cut]);
    out.push_str(&terminator);
    Ok(Some(TModel::from_scalar(out)))
}

/// `?truncate_m` —— markup-aware truncate（v1 不支持）
pub fn truncate_m(
    _env: &mut Environment,
    _target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Err(TemplateError::misc(
        "The \"truncate_m\" built-in requires markup/node infrastructure which isn't supported yet.",
    ))
}

/// `?truncate_w_m` —— markup-aware word truncate（v1 不支持）
pub fn truncate_w_m(
    _env: &mut Environment,
    _target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Err(TemplateError::misc(
        "The \"truncate_w_m\" built-in requires markup/node infrastructure which isn't supported yet.",
    ))
}

/// `?truncate_c_m` —— markup-aware character truncate（v1 不支持）
pub fn truncate_c_m(
    _env: &mut Environment,
    _target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Err(TemplateError::misc(
        "The \"truncate_c_m\" built-in requires markup/node infrastructure which isn't supported yet.",
    ))
}

#[cfg(test)]
mod tests {

    #[test]
    fn capitalize_and_chomp() {
        assert_eq!(
            capitalize_simple("dieBugsDie! * vazzZE"),
            "Diebugsdie! * Vazzze"
        );
        assert_eq!(chop_linebreak_simple("qwe\r\n\r\n"), "qwe\r\n");
        assert_eq!(chop_linebreak_simple("qwe\n"), "qwe");
        assert_eq!(chop_linebreak_simple("qwe"), "qwe");
    }

    fn capitalize_simple(s: &str) -> String {
        let mut out = String::new();
        let mut token = String::new();
        for c in s.chars() {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                if !token.is_empty() {
                    let mut chars = token.chars();
                    out.push_str(&chars.next().unwrap().to_uppercase().collect::<String>());
                    out.push_str(&chars.as_str().to_lowercase());
                    token.clear();
                }
                out.push(c);
            } else {
                token.push(c);
            }
        }
        if !token.is_empty() {
            let mut chars = token.chars();
            out.push_str(&chars.next().unwrap().to_uppercase().collect::<String>());
            out.push_str(&chars.as_str().to_lowercase());
        }
        out
    }

    fn chop_linebreak_simple(s: &str) -> String {
        if let Some(rest) = s.strip_suffix("\r\n") {
            rest.to_string()
        } else if let Some(rest) = s.strip_suffix('\r') {
            rest.to_string()
        } else if let Some(rest) = s.strip_suffix('\n') {
            rest.to_string()
        } else {
            s.to_string()
        }
    }

    #[test]
    fn pad_utf16_semantics() {
        // 非 BMP 字符占 2 个 UTF-16 码元（Java String.length）
        let s = "\u{10000}".to_string();
        assert_eq!(s.encode_utf16().count(), 2);
        // left_pad("\u{10000}", 4, " ") → 2 个空格 + 字符
        let pad = pad_impl_pure(&s, 4, " ", true);
        assert_eq!(pad.encode_utf16().count(), 4);
        assert_eq!(pad, "  \u{10000}");
    }

    fn pad_impl_pure(s: &str, width: usize, filling: &str, left: bool) -> String {
        let s_units: Vec<u16> = s.encode_utf16().collect();
        if width <= s_units.len() {
            return s.to_string();
        }
        let filling: Vec<u16> = filling.encode_utf16().collect();
        let dif = width - s_units.len();
        let mut pad: Vec<u16> = Vec::with_capacity(dif);
        let mut i = 0;
        while pad.len() < dif {
            pad.push(filling[i % filling.len()]);
            i += 1;
        }
        let pad = String::from_utf16_lossy(&pad);
        if left {
            format!("{pad}{s}")
        } else {
            format!("{s}{pad}")
        }
    }

    /// 渲染辅助（keep/ensure/substring 家族矩阵，断言源自 Java templatesuite
    /// string-builtins3.ftl + jar 探针）
    fn render_out(src: &str) -> crate::error::Result<String> {
        use crate::cache::StringLoader;
        use crate::template::{Configuration, ObjectWrapper, SimpleObjectWrapper, TModel};
        let mut c = Configuration::new();
        let loader = std::sync::Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", src);
        let t = c.get_template("t.ftl")?;
        let root = SimpleObjectWrapper
            .wrap(&crate::template::DynValue::Map(vec![]))?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn keep_regexp_overlapping_scan() {
        // Java matcher.find(start+1) 重叠扫描：非重叠 find_iter 漏匹配
        // （string-builtins3.ftl:37-40）
        assert_eq!(
            render_out("${'aaabb'?keep_before_last('[ab]{3}', 'r')}").unwrap(),
            "aa",
            "正则 [ab]{{3}} 在 aaabb 上的最后匹配 start=2（aaa@0/aab@1/abb@2）"
        );
        assert_eq!(
            render_out("${'aaabbxbabe'?keep_before_last('[ab]{3}', 'r')}").unwrap(),
            "aaabbx",
            "babe 上的 abb@… 后移"
        );
        assert_eq!(
            render_out("${'aaabb12345'?keep_after_last('[ab]{3}', 'r')}").unwrap(),
            "12345"
        );
        // 无重叠歧义时与 find_iter 一致
        assert_eq!(
            render_out("${'xxxaaayyy'?keep_before_last('a+', 'r')}").unwrap(),
            "xxxaa"
        );
        assert_eq!(
            render_out("${'xxxaaayyy'?keep_after_last('a+', 'r')}").unwrap(),
            "yyy"
        );
    }

    #[test]
    fn ensure_starts_with_default_regexp() {
        // Java BuiltInsForStringsBasic.java:163-166：2 参数无显式 flags → 默认 REGEXP
        assert_eq!(
            render_out("${'bacdef'?ensure_starts_with('[ab]{2}', 'ab')}").unwrap(),
            "bacdef",
            "默认正则前缀 [ab]{{2}} 匹配 ba → 不加前缀"
        );
        assert_eq!(
            render_out("${'cacdef'?ensure_starts_with('[ab]{2}', 'ab')}").unwrap(),
            "abcacdef",
            "ca 不匹配 [ab]{{2}} → 加前缀 ab"
        );
        assert_eq!(
            render_out("${'HTTP://example.com'?ensure_starts_with('[a-z]+://', 'http://', 'ir')}")
                .unwrap(),
            "HTTP://example.com",
            "显式 ir flags"
        );
        // Java ensure_ends_with 仅 1 参数（checkMethodArgCount(args, 1)）
        let err = render_out("${'x'?ensure_ends_with('x', 'x')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?ensure_ends_with(...) expects 1 argument but has received 2."),
            "{err}"
        );
    }

    #[test]
    fn arg_count_messages_match_java() {
        // _MessageUtil.newArgCntError：max-min==1 → "expects 1 or 2 arguments"；
        // argCnt==0 → "but has received none."
        let err = render_out("${'x'?keep_before()}").unwrap_err().to_string();
        assert!(
            err.contains("?keep_before(...) expects 1 or 2 arguments but has received none."),
            "{err}"
        );
        let err = render_out("${'x'?keep_before('x', 'i', 'x')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?keep_before(...) expects 1 or 2 arguments but has received 3."),
            "{err}"
        );
        let err = render_out("${'x'?ensure_starts_with('x', 'x', 'x', 'x')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?ensure_starts_with(...) expects 1 to 3 arguments but has received 4."),
            "{err}"
        );
        // "m" flag 无正则 → 报错（RegexpHelper.checkOnlyHasNonRegexpFlags）
        let err = render_out("${'x'?keep_before('x', 'm')}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("\"m\" flag"), "{err}");
    }

    // -----------------------------------------------------------------------
    // truncate 家族单元测试
    // -----------------------------------------------------------------------

    #[test]
    fn truncate_no_truncation_needed() {
        // 串长 ≤ max_len → 原样返回
        assert_eq!(render_out("${'hello'?truncate(10)}").unwrap(), "hello");
        assert_eq!(render_out("${'hello'?truncate(5)}").unwrap(), "hello");
    }

    #[test]
    fn truncate_basic() {
        // "hello world", truncate(8) → "hello..."
        // UTF-16: 'hello world' = 11 码元；8 码元预算；terminator "..." = 3 码元；
        // 保留 8-3=5 码元 → "hello" + "..." → "hello..."
        assert_eq!(render_out("${'hello world'?truncate(8)}").unwrap(), "hello...");
    }

    #[test]
    fn truncate_zero_length() {
        // max_len <= 0 → 空串
        assert_eq!(render_out("${'hello'?truncate(0)}").unwrap(), "");
        assert_eq!(render_out("${'hello'?truncate(-1)}").unwrap(), "");
    }

    #[test]
    fn truncate_custom_terminator() {
        assert_eq!(
            render_out("${'hello world'?truncate(7, '!')}").unwrap(),
            "hello !"
        );
    }

    #[test]
    fn truncate_terminator_wont_fit() {
        // max_len=2, terminator="..." (3 UTF-16) → max_len < terminator len → 空串
        assert_eq!(render_out("${'hello'?truncate(2)}").unwrap(), "");
    }

    #[test]
    fn truncate_non_bmp() {
        // 😀 (U+1F600) 占 2 个 UTF-16 码元；"a😀b😀c" → UTF-16 长度=7
        // truncate(5) → 保留 5-3=2 码元 → "a" + "..." → "a..."
        assert_eq!(
            render_out("${'a\u{1F600}b\u{1F600}c'?truncate(5)}").unwrap(),
            "a..."
        );
    }

    #[test]
    fn truncate_w_basic() {
        assert_eq!(
            render_out("${'one two three four'?truncate_w(2)}").unwrap(),
            "one two..."
        );
    }

    #[test]
    fn truncate_w_no_truncation() {
        assert_eq!(
            render_out("${'one two'?truncate_w(5)}").unwrap(),
            "one two"
        );
    }

    #[test]
    fn truncate_w_custom_terminator() {
        assert_eq!(
            render_out("${'a b c d'?truncate_w(2, ' [more]')}").unwrap(),
            "a b [more]"
        );
    }

    #[test]
    fn truncate_c_basic() {
        // "hello world" = 11 char; truncate_c(8 char) → 保留 8-3=5 char → "hello..."
        assert_eq!(
            render_out("${'hello world'?truncate_c(8)}").unwrap(),
            "hello..."
        );
    }

    #[test]
    fn truncate_c_non_bmp() {
        // 😀 (U+1F600) 是一个 Unicode char（code point）
        // "a😀b😀c" = 5 Unicode chars
        // truncate_c(4) → 保留 4-3=1 char → "a..."
        assert_eq!(
            render_out("${'a\u{1F600}b\u{1F600}c'?truncate_c(4)}").unwrap(),
            "a..."
        );
    }

    #[test]
    fn truncate_c_custom_terminator() {
        assert_eq!(
            render_out("${'hello world'?truncate_c(6, '!')}").unwrap(),
            "hello!"
        );
    }

    #[test]
    fn truncate_c_no_truncation() {
        assert_eq!(
            render_out("${'hi'?truncate_c(5)}").unwrap(),
            "hi"
        );
    }

    #[test]
    fn truncate_m_not_supported() {
        let err = render_out("${'hello'?truncate_m(5)}").unwrap_err().to_string();
        assert!(err.contains("truncate_m"), "{err}");
        assert!(err.contains("isn't supported yet"), "{err}");
    }

    #[test]
    fn truncate_w_m_not_supported() {
        let err = render_out("${'hello'?truncate_w_m(5)}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("truncate_w_m"), "{err}");
    }

    #[test]
    fn truncate_c_m_not_supported() {
        let err = render_out("${'hello'?truncate_c_m(5)}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("truncate_c_m"), "{err}");
    }

    #[test]
    fn truncate_arg_count_validation() {
        let err = render_out("${'x'?truncate()}").unwrap_err().to_string();
        assert!(
            err.contains("?truncate(...) expects 1 or 2 arguments but has received none."),
            "{err}"
        );
        let err = render_out("${'x'?truncate(1, '...', 2)}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?truncate(...) expects 1 or 2 arguments but has received 3."),
            "{err}"
        );
    }
}
