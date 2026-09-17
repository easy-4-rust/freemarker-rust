//! 排序与极值内建 —— sort/sort_by（Java sortBI/sort_byBI.sort :703-839）、Collator
//! 近似排序键、min/max（Java MinOrMaxBI）。

use super::{eval_arg_lenient, java_date_type_name, sequence_items};
use crate::core::eval_util::{arg_count, check_arg_count};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::value::{DateType, DateValue, TNumber};
use std::cmp::Ordering;

/// ?sort —— Java sortBI：直接按元素排序（键 = 元素本身）
pub fn sort(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("sort", args, 0, 0)?;
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "sort")?;
    if items.is_empty() {
        // Java sortBI.sort :706：空序列返回原模型
        return Ok(Some(t));
    }
    let keys: Vec<TModel> = items.clone();
    Ok(Some(TModel::from_sequence(sort_by_keys(&items, 0, &keys)?)))
}

/// ?sort_by(key[, key2, ...]) —— Java sort_byBI：按子变量键排序（键路径）
pub fn sort_by(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    // Java sort_byBI.BIMethod.exec :559-562：BC 下只查 args.size() < 1
    // （_MessageUtil.newArgCntError("?" + key, 0, 1) → "expects 1 argument but has received none."）
    if arg_count(args) < 1 {
        return Err(TemplateError::misc(
            "?sort_by(...) expects 1 argument but has received none.",
        ));
    }
    let mut key_names: Vec<String> = Vec::new();
    for i in 0..arg_count(args) {
        let a = args.unwrap()[i].clone();
        let m = eval_arg_lenient(env, Some(&[a]), 0)?;
        if let Some(s) = &m.scalar {
            key_names.push(s.as_string()?);
        } else if m.is_sequence() {
            // Java :569-580：序列参数必须全为字符串（按项报错含索引）
            let seq = m.sequence.clone().unwrap();
            let n = seq.size()?;
            for j in 0..n {
                let item = seq.get(j)?;
                match &item.scalar {
                    Some(s) => key_names.push(s.as_string()?),
                    None => {
                        return Err(TemplateError::misc(format!(
                            "The argument to ?sort_by(key), when it's a sequence, must be a sequence of strings, but the item at index {j} is not a string."
                        )));
                    }
                }
            }
        } else {
            return Err(TemplateError::misc(
                "The argument to ?sort_by(key) must be a string (the name of the subvariable), or a sequence of strings (the \"path\" to the subvariable).",
            ));
        }
    }
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "sort_by")?;
    if items.is_empty() {
        return Ok(Some(t));
    }
    // 取每个元素的键（Java sort :716-741：逐级取子变量；缺失 → "The \"{name}\"
    // subvariable was null or missing."；非哈希 → 哈希错误消息）
    let mut keys = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let prefix = format!(
            "?sort_by(...) failed at sequence index {i}{}: ",
            if i == 0 { "" } else { " (0-based)" }
        );
        let mut key = item.clone();
        for (kn, name) in key_names.iter().enumerate() {
            if !key.is_hash() {
                // Java :721-734：keyNameI == 0 → "Sequence items must be hashes when
                // using ?sort_by. "（后接 " subvariable is not a hash, ..."，形成双空格）；
                // keyNameI > 0 → "The \"{prev}\" subvariable is not a hash, ..."
                return Err(TemplateError::misc(if kn == 0 {
                    format!(
                        "{prefix}Sequence items must be hashes when using ?sort_by.  subvariable is not a hash, so ?sort_by can't proceed with getting the \"{name}\" subvariable."
                    )
                } else {
                    format!(
                        "{prefix}The \"{}\" subvariable is not a hash, so ?sort_by can't proceed with getting the \"{name}\" subvariable.",
                        key_names[kn - 1]
                    )
                }));
            }
            let h = key.get_hash()?;
            key = h.get(name)?.ok_or_else(|| {
                // Java :736-740
                TemplateError::misc(format!(
                    "{prefix}The \"{name}\" subvariable was null or missing."
                ))
            })?;
        }
        keys.push(key);
    }
    Ok(Some(TModel::from_sequence(sort_by_keys(
        &items,
        key_names.len(),
        &keys,
    )?)))
}

/// 强转后的排序键（Java sortBI.sort 的 KVP.key：按 keyType 定型，:628-636）
enum CastKey {
    Number(TNumber),
    String(String),
    Date(DateValue),
    Boolean(bool),
}

/// 按键排序（Java sortBI.sort :703-839）：首键定类型（标量→字符串/数字/日期/布尔，
/// 其余 → "Values used for sorting must be..."），后续键按该类型强转、不一致 →
/// newInconsistentSortKeyTypeException :670-688（value/key value 措辞按 keyNamesLn）；
/// 排序稳定（Java Collections.sort 稳定）。字符串键按 Collator 排序（v1 用
/// UTF-16 码元序近似，同 eval.rs utf16_cmp）。
fn sort_by_keys(items: &[TModel], key_names_ln: usize, keys: &[TModel]) -> Result<Vec<TModel>> {
    // Java :674-680：keyNamesLn == 0 → "value"/"values"，否则 "key value"/"key values"
    let (value_word, values_word) = if key_names_ln == 0 {
        ("value", "values")
    } else {
        ("key value", "key values")
    };
    let bi_name = if key_names_ln == 0 {
        "?sort"
    } else {
        "?sort_by(...)"
    };
    // Java startErrorMessage :845-850：index == 0 → ": "，其余 → " (0-based): "
    let start_err = |i: usize| {
        format!(
            "{bi_name} failed at sequence index {i}{}: ",
            if i == 0 { "" } else { " (0-based)" }
        )
    };
    let mut key_type: Option<KeyType> = None;
    let mut keyed: Vec<(CastKey, usize)> = Vec::with_capacity(items.len());
    for (i, key) in keys.iter().enumerate() {
        let t = match key_type {
            Some(t) => t,
            None => {
                let t = classify_key(key).ok_or_else(|| {
                    // Java :759-762：首键类型不受支持
                    TemplateError::misc(format!(
                        "{}{}",
                        start_err(i),
                        "Values used for sorting must be numbers, strings, date/times or booleans."
                    ))
                })?;
                key_type = Some(t);
                t
            }
        };
        let ck = cast_key(key, t).ok_or_else(|| {
            // Java newInconsistentSortKeyTypeException :670-688
            let (first_type, first_plural) = sort_type_words(t);
            TemplateError::misc(format!(
                "{}All {values_word} in the sequence must be {first_plural}, because the first {value_word} was that. However, the {value_word} of the current item isn't a {first_type} but a {}.",
                start_err(i),
                key.type_name
            ))
        })?;
        keyed.push((ck, i));
    }
    // Java Collections.sort(res, keyComparator)（稳定）：类型已按首键统一，比较不会失败
    keyed.sort_by(|(a, _), (b, _)| compare_cast_keys(a, b));
    Ok(keyed.into_iter().map(|(_, i)| items[i].clone()).collect())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyType {
    Number,
    String,
    Date,
    Boolean,
}

/// 首键类型判定（Java :743-763：标量 → 字符串、数字 → 数字、日期 → 日期、布尔 → 布尔）
fn classify_key(k: &TModel) -> Option<KeyType> {
    if k.is_scalar() {
        Some(KeyType::String)
    } else if k.is_number() {
        Some(KeyType::Number)
    } else if k.is_date() {
        Some(KeyType::Date)
    } else if k.is_boolean() {
        Some(KeyType::Boolean)
    } else {
        None
    }
}

/// 键强转（Java :765-822：按 keyType 强制转换，失败 → 不一致类型异常）
fn cast_key(key: &TModel, t: KeyType) -> Option<CastKey> {
    match t {
        KeyType::Number => {
            if key.is_number() {
                key.get_number().ok().map(CastKey::Number)
            } else {
                None
            }
        }
        KeyType::String => {
            if key.is_scalar() {
                key.get_scalar().ok().map(CastKey::String)
            } else {
                None
            }
        }
        KeyType::Date => {
            if key.is_date() {
                key.get_date().ok().map(CastKey::Date)
            } else {
                None
            }
        }
        KeyType::Boolean => {
            if key.is_boolean() {
                key.get_boolean().ok().map(CastKey::Boolean)
            } else {
                None
            }
        }
    }
}

/// 类型名（Java newInconsistentSortKeyTypeException 的 firstType 措辞）
fn sort_type_words(t: KeyType) -> (&'static str, &'static str) {
    match t {
        KeyType::Number => ("number", "numbers"),
        KeyType::String => ("string", "strings"),
        KeyType::Date => ("date/time", "date/times"),
        KeyType::Boolean => ("boolean", "booleans"),
    }
}

/// 同型键比较（Java 各 KVPComparator：数字 → ArithmeticEngine.compareNumbers、
/// 字符串 → Collator（v1 近似）、日期 → Date.compareTo、布尔 → 自定义）
fn compare_cast_keys(a: &CastKey, b: &CastKey) -> Ordering {
    match (a, b) {
        (CastKey::Number(x), CastKey::Number(y)) => crate::core::eval::compare_numbers(x, y),
        (CastKey::String(x), CastKey::String(y)) => collator_cmp(x, y),
        (CastKey::Date(x), CastKey::Date(y)) => x.dt.cmp(&y.dt),
        (CastKey::Boolean(x), CastKey::Boolean(y)) => x.cmp(y),
        // 不可能：类型已按首键统一
        _ => Ordering::Equal,
    }
}

/// 近似 Java `Collator`（Locale.US，TERTIARY 强度）的字符串排序比较（对应 Java
/// LexicalKVPComparator :637-649 的 `Collator.compare`）：
///
/// **核心原理**：标点符号在 Collator 中有非零的主权重（primary weight），不能像
/// `?sort` 的简单码元序那样直接剥离。本实现为每个 ASCII 标点分配一个排序键代理字符，
/// 其相对顺序由 jar 实测 Java Collator.getInstance(Locale.US).setStrength(TERTIARY)
/// 确定。字母数字使用自身的小写码元。
///
/// **代理顺序**（jar ProbeCollator 实测，左=先排序）：
/// `_` < `:` < `!` < `/` < `.` < `'` < `"` < `-` < `@`
///
/// **已知限制（P4）**：
/// - 非 ASCII 标点与带重音字符使用码元序作为近似
/// - 次强度（secondary）的重音差异不区分
/// - 完整 Collator 需 ICU/CLDR 数据表（>100KB），留待 P6 对齐
pub(crate) fn collator_cmp(a: &str, b: &str) -> Ordering {
    let ka = collation_sort_key(a);
    let kb = collation_sort_key(b);
    // 主强度比较（使用排序键 = 代理字符替换标点后的序列）
    match ka.cmp(&kb) {
        Ordering::Equal => {
            // 第三强度：逐字符比较大小写与标点权重
            let au: Vec<u16> = a.encode_utf16().collect();
            let bu: Vec<u16> = b.encode_utf16().collect();
            for (x, y) in au.iter().zip(bu.iter()) {
                if x == y {
                    continue;
                }
                if let (Some(xc), Some(yc)) = (char::from_u32(*x as u32), char::from_u32(*y as u32))
                {
                    // 同字母异大小写 → 小写在前（Java Collator TERTIARY）
                    let xl = xc.to_lowercase().next();
                    let yl = yc.to_lowercase().next();
                    if xl.is_some() && xl == yl && xc.is_lowercase() != yc.is_lowercase() {
                        return if xc.is_lowercase() {
                            Ordering::Less
                        } else {
                            Ordering::Greater
                        };
                    }
                    // 标点第三强度权重（jar 实测 TERTIARY 标点顺序）
                    let xw = collation_weight(xc);
                    let yw = collation_weight(yc);
                    if xw != yw {
                        return xw.cmp(&yw);
                    }
                }
                return x.cmp(y);
            }
            au.len().cmp(&bu.len())
        }
        o => o,
    }
}

/// 构建 Collator 排序键：将每个字符映射为其主权重代理。
/// 标点代理在 U+0001‥U+0009 区段，保证标点 < 数字 < 字母的顺序。
fn collation_sort_key(s: &str) -> Vec<u16> {
    s.chars()
        .flat_map(|c| {
            // 小写折叠
            let lc = c.to_lowercase().next().unwrap_or(c);
            match lc {
                '_' => vec![0x0001],
                ':' => vec![0x0002],
                '!' => vec![0x0003],
                '/' => vec![0x0004],
                '.' => vec![0x0005],
                '\'' => vec![0x0006],
                '"' => vec![0x0007],
                '-' => vec![0x0008],
                '@' => vec![0x0009],
                // 其他 ASCII 标点：码元偏移到 0x000A-0x002F（保持相对顺序）
                c if c.is_ascii_punctuation() => {
                    vec![0x000A + (c as u32 as u16).saturating_sub(0x21)]
                }
                // 字母数字：小写码元（>= 0x30）
                c => vec![c as u16],
            }
        })
        .collect()
}

/// Java Collator（Locale.US，TERTIARY）的 ASCII 标点第三强度权重（jar ProbeCollator
/// 实测；权重越大在排序中越靠后）。
fn collation_weight(c: char) -> u32 {
    match c {
        '_' => 1,
        ':' => 2,
        '!' => 3,
        '/' => 4,
        '.' => 5,
        '\'' => 6,
        '"' => 7,
        '-' => 8,
        '@' => 9,
        ' ' => 10,
        ',' => 11,
        ';' => 12,
        '?' => 13,
        '`' => 14,
        '^' => 15,
        '~' => 16,
        '(' => 17,
        ')' => 18,
        '[' => 19,
        ']' => 20,
        '{' => 21,
        '}' => 22,
        '$' => 23,
        '*' => 24,
        '\\' => 25,
        '&' => 26,
        '#' => 27,
        '%' => 28,
        '+' => 29,
        '<' => 30,
        '=' => 31,
        '>' => 32,
        '|' => 33,
        _ => u32::from(c), // 非 ASCII：码点兜底
    }
}

/// UTF-16 码元字典序（保留供其他模块使用）
#[allow(dead_code)]
pub(crate) fn utf16_cmp(a: &str, b: &str) -> Ordering {
    let au: Vec<u16> = a.encode_utf16().collect();
    let bu: Vec<u16> = b.encode_utf16().collect();
    for (x, y) in au.iter().zip(bu.iter()) {
        match x.cmp(y) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    au.len().cmp(&bu.len())
}

/// ?min / ?max —— Java MinOrMaxBI：序列/集合元素极值；null 元素跳过、空 → null
/// （下游 InvalidReferenceException）；比较按 EvalUtil.compare
/// （参数 (quoteOperandsInErrors=true, typeMismatchMeansNotEqual=false, nullReturnsFalse=false)：
/// 字符串/布尔上的大小比较报错、类型不匹配报错）
fn min_max_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    want_max: bool,
) -> Result<Option<TModel>> {
    check_arg_count(if want_max { "max" } else { "min" }, args, 0, 0)?;
    let t = crate::core::eval::eval(env, target)?;
    if t.range.as_ref().is_some_and(|r| r.unbounded) {
        // Java MinOrMaxBI._eval :975：checkNotRightUnboundedNumericalRange
        return Err(TemplateError::misc(
            "The input sequence is a right-unbounded numerical range, thus, it's infinitely long, and can't processed with this built-in.",
        ));
    }
    let bi = if want_max { "max" } else { "min" };
    let items = super::seq_or_collection_items(&t, bi)?;
    // Java calculateResultForSequence :999-1011：cur == null 跳过；空 → null
    let mut best: Option<TModel> = None;
    for item in &items {
        if item.is_nothing() {
            continue;
        }
        match &best {
            None => best = Some(item.clone()),
            Some(b) => {
                if compare_for_min_max(item, b, want_max)? {
                    best = Some(item.clone());
                }
            }
        }
    }
    Ok(Some(best.unwrap_or_else(TModel::nothing)))
}

/// 极值比较（Java MinOrMaxBI → EvalUtil.compare(cur, null, op, null, best, null,
/// this, true, false, false, false, env)：操作符名取 cmpOpToString 的
/// "greater-than"/"less-than"（operatorString == null，:339-353））
fn compare_for_min_max(a: &TModel, b: &TModel, want_max: bool) -> Result<bool> {
    let op_str = if want_max {
        "greater-than"
    } else {
        "less-than"
    };
    let cmp = if a.is_number() && b.is_number() {
        crate::core::eval::compare_numbers(&a.get_number()?, &b.get_number()?)
    } else if a.is_date() && b.is_date() {
        let ld = a.get_date()?;
        let rd = b.get_date()?;
        if ld.kind == DateType::Unknown || rd.kind == DateType::Unknown {
            let side = if ld.kind == DateType::Unknown {
                "left"
            } else {
                "right"
            };
            return Err(TemplateError::misc(format!(
                "The {side} value of the comparison is a date-like value where it's not known if it's a date (no time part), time, or date-time, and thus can't be used in a comparison."
            )));
        }
        if ld.kind != rd.kind {
            return Err(TemplateError::misc(format!(
                "Can't compare dates of different types. Left date type is {}, right date type is {}.",
                java_date_type_name(ld.kind),
                java_date_type_name(rd.kind)
            )));
        }
        ld.dt.cmp(&rd.dt)
    } else if a.is_scalar() && b.is_scalar() {
        // Java :262-266：字符串只支持 ==/!=（min/max 恒用大小比较 → 必报错）
        return Err(TemplateError::misc(format!(
            "Can't use operator \"{op_str}\" on string values."
        )));
    } else if a.is_boolean() && b.is_boolean() {
        return Err(TemplateError::misc(format!(
            "Can't use operator \"{op_str}\" on boolean values."
        )));
    } else {
        // Java :307-326：typeMismatchMeansNotEqual=false → 报错（左右操作数类型描述）
        return Err(TemplateError::misc(format!(
            "Can't compare values of these types. Allowed comparisons are between two numbers, two strings, two dates, or two booleans.\nLeft hand operand is a {}.\nRight hand operand is a {}.",
            a.type_name, b.type_name
        )));
    };
    Ok(if want_max {
        cmp == Ordering::Greater
    } else {
        cmp == Ordering::Less
    })
}

pub fn min(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    min_max_impl(env, target, args, false)
}

pub fn max(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    min_max_impl(env, target, args, true)
}
