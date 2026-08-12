//! 序列内建 —— 对应 Java `BuiltInsForSequences.java`（本文件为 eval.rs 内建集未覆盖的
//! 子集：chunk/filter/map/take_while/drop_while/sort/sort_by/min/max/seq_index_of/
//! seq_last_index_of 等；join/reverse/seq_contains 自 built_in.rs 迁入（2026-08-04））。
//!
//! 语义要点（Java 对照）：
//! - filter/map/take_while/drop_while 消费 lambda（Java ElementTransformer →
//!   LocalLambdaExpression.invokeLambdaDefinedFunction：以参数为新局部上下文求值体）；
//! - seq_index_of/seq_last_index_of（Java seq_index_ofBI(findFirst)）：modelsEqual 宽松相等
//!   （EvalUtil.compare，参数 (true,true,true)：类型不匹配→false、null→false），
//!   支持 fromIndex（Java getNumberMethodArg(...).intValue() 向零截断）；未找到 → -1；
//!   目标为序列或集合（Java 2.3.x 序列优先，BIMethod :389-413）；
//! - sort/sort_by（Java sortBI/sort_byBI.sort :703-839）：键类型按首个元素定
//!   （字符串/数字/日期/布尔），后续键按该类型强转、不一致报错
//!   （newInconsistentSortKeyTypeException :670-688）；字符串按 Collator 排序
//!   （v1 用 UTF-16 码元序近似）；
//! - chunk(size[, filler])：子序列序列（Java chunkBI/ChunkedSequence）；size 按
//!   intValue() 截断（无整数检查）、<1 报错；filler 为 null（缺失变量）→ 不补齐；
//! - min/max（Java MinOrMaxBI）：null 元素跳过、空 → null（下游 InvalidReference）；
//!   字符串/布尔上的 > 运算报错（EvalUtil.compare :262-277）。

use crate::core::environment::{model_to_string, BodyCtx, LambdaValue, LocalEntry};
use crate::core::eval;
use crate::core::eval_util::{arg_count, check_arg_count};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::value::{DateType, DateValue, TNumber};
use std::cmp::Ordering;
use std::rc::Rc;

/// 求值 lambda 体（Java LocalLambdaExpression.invokeLambdaDefinedFunction：
/// 参数绑定为新局部上下文后求值体；?map/?filter 等的消费方）
fn invoke_lambda(env: &mut Environment, lam: &TModel, arg: TModel) -> Result<TModel> {
    let lv = lam
        .internal::<LambdaValue>()
        .ok_or_else(|| TemplateError::misc("The argument must be a lambda expression"))?;
    let mut vars = std::collections::HashMap::new();
    if let Some(p) = lv.params.first() {
        vars.insert(p.clone(), arg);
    }
    env.push_local(LocalEntry::Body(Rc::new(BodyCtx { vars })));
    let r = crate::core::eval::eval(env, &lv.body);
    env.pop_local();
    r
}

/// 求值参数为 lambda 模型（Java getElementTransformerExp 的求值）
fn arg_lambda(
    env: &mut Environment,
    args: Option<&[Expr]>,
    idx: usize,
    bi: &str,
) -> Result<TModel> {
    check_arg_count(bi, args, 1, 1)?;
    let e = args
        .and_then(|a| a.get(idx))
        .ok_or_else(|| TemplateError::misc(format!("The ?{bi} built-in expects one argument")))?;
    let m = crate::core::eval::eval(env, e)?;
    if !m.is_lambda() {
        return Err(TemplateError::misc(format!(
            "The argument to ?{bi} must be a lambda expression"
        )));
    }
    Ok(m)
}

/// 求值内建参数（Java 方法参数求值语义：缺失变量 → null 流入方法而非报错，
/// 由各内建决定后续处理——?seq_index_of(?noSuchVar) → -1、?chunk(?, noSuchVar) →
/// filler 为 null 不补齐；?chunk(noSuchVar) 则由 method_number_arg 报 "received a Null"）
pub(crate) fn eval_arg_lenient(
    env: &mut Environment,
    args: Option<&[Expr]>,
    idx: usize,
) -> Result<TModel> {
    let e = args
        .and_then(|a| a.get(idx))
        .ok_or_else(|| TemplateError::misc("Missing argument"))?;
    match crate::core::eval::eval(env, e) {
        Ok(m) => Ok(m),
        Err(TemplateError::InvalidReference { .. }) => Ok(TModel::nothing()),
        Err(e) => Err(e),
    }
}

/// 类型描述（Java `_DelayedAOrAn(_DelayedFTLTypeDescription)`；null → "a Null"；
/// wrapper 信息为 Java 特有，Rust 侧省略——与 compare_models 等处的约定一致）
fn ftl_type_desc(m: &TModel) -> String {
    if m.is_nothing() {
        "a Null".to_string()
    } else {
        format!("a {}", m.type_name)
    }
}

/// 方法数字参数（Java BuiltIn.getNumberMethodArg → _MessageUtil.newMethodArgMustBeNumberException：
/// `?{bi}(...) expects a number as argument #{idx+1}, but received {type}.`；
/// 缺失变量 → null → "a Null"）
fn method_number_arg(
    env: &mut Environment,
    args: Option<&[Expr]>,
    idx: usize,
    bi: &str,
) -> Result<TNumber> {
    let m = eval_arg_lenient(env, args, idx)?;
    if !m.is_number() {
        return Err(TemplateError::misc(format!(
            "?{bi}(...) expects a number as argument #{}, but received {}.",
            idx + 1,
            ftl_type_desc(&m)
        )));
    }
    m.get_number()
}

/// 序列元素枚举（仅序列；Java BuiltInForSequence.calculateResult 的强转语义——
/// ?chunk/?filter/?map/?sort 等目标必须是序列）
fn sequence_items(m: &TModel, bi: &str) -> Result<Vec<TModel>> {
    let seq = m.sequence.clone().ok_or_else(|| {
        TemplateError::misc(format!(
            "?{bi} is not applicable to a {} value",
            m.type_name
        ))
    })?;
    let n = seq.size()?;
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        v.push(seq.get(i)?);
    }
    Ok(v)
}

/// 序列或集合元素枚举（Java seq_index_ofBI.BIMethod :389-413：TemplateSequenceModel
/// 优先（2.3.x BC），否则 TemplateCollectionModel 迭代；两者皆非 → 报错。
/// 供 ?seq_index_of/?seq_last_index_of/?seq_contains/?min/?max 使用）
pub(crate) fn seq_or_collection_items(m: &TModel, bi: &str) -> Result<Vec<TModel>> {
    let mut v = Vec::new();
    if let Some(seq) = &m.sequence {
        let n = seq.size()?;
        v.reserve(n);
        for i in 0..n {
            v.push(seq.get(i)?);
        }
        return Ok(v);
    }
    if let Some(c) = &m.collection {
        for item in c.iterator()? {
            v.push(item?);
        }
        return Ok(v);
    }
    Err(TemplateError::misc(format!(
        "?{bi} is not applicable to a {} value",
        m.type_name
    )))
}

/// ?filter(lambda) —— Java filterBI：谓词为真的元素序列（急切版）
pub fn filter(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let lam = arg_lambda(env, args, 0, "filter")?;
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "filter")?;
    let mut out = Vec::new();
    for item in items {
        let r = invoke_lambda(env, &lam, item.clone())?;
        let b = r.eval_boolean().map_err(|_| {
            TemplateError::misc("The filter expression had to return a boolean value")
        })?;
        if b {
            out.push(item);
        }
    }
    Ok(Some(TModel::from_sequence(out)))
}

/// ?map(lambda) —— Java mapBI：元素映射序列（急切版）
pub fn map(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    let lam = arg_lambda(env, args, 0, "map")?;
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "map")?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let r = invoke_lambda(env, &lam, item)?;
        if r.is_nothing() {
            return Err(TemplateError::misc(
                "The element mapper function has returned no return value (has returned null).",
            ));
        }
        out.push(r);
    }
    Ok(Some(TModel::from_sequence(out)))
}

/// ?take_while(lambda) —— Java take_whileBI：谓词为真的前缀
pub fn take_while(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let lam = arg_lambda(env, args, 0, "take_while")?;
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "take_while")?;
    let mut out = Vec::new();
    for item in items {
        let r = invoke_lambda(env, &lam, item.clone())?;
        let b = r.eval_boolean().map_err(|_| {
            TemplateError::misc("The filter expression had to return a boolean value")
        })?;
        if b {
            out.push(item);
        } else {
            break;
        }
    }
    Ok(Some(TModel::from_sequence(out)))
}

/// ?drop_while(lambda) —— Java drop_whileBI：跳过谓词为真的前缀，其后全部保留
pub fn drop_while(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let lam = arg_lambda(env, args, 0, "drop_while")?;
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "drop_while")?;
    let mut out = Vec::new();
    let mut dropping = true;
    for item in items {
        if dropping {
            let r = invoke_lambda(env, &lam, item.clone())?;
            let b = r.eval_boolean().map_err(|_| {
                TemplateError::misc("The filter expression had to return a boolean value")
            })?;
            if b {
                continue;
            }
            dropping = false;
        }
        out.push(item);
    }
    Ok(Some(TModel::from_sequence(out)))
}

/// ?chunk(size[, filler]) —— Java chunkBI：子序列序列（ChunkedSequence）
pub fn chunk(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("chunk", args, 1, 2)?;
    let size = method_number_arg(env, args, 0, "chunk")?;
    // Java chunkBI.exec :70：getNumberMethodArg(args, 0).intValue() 向零截断
    // （非整数不报错；超出 i64 的极端值按 Java intValue 的"正数化"近似钳制）
    let chunk_size_i = crate::core::eval::trunc_i64(&size).unwrap_or(i64::MAX);
    if chunk_size_i < 1 {
        return Err(TemplateError::misc(
            "The 1st argument to ?chunk (...) must be at least 1.",
        ));
    }
    let chunk_size = chunk_size_i as usize;
    // Java :78：args.size() > 1 时取 args.get(1)；null（缺失变量）→ fillerItem null → 不补齐
    let filler: Option<TModel> = if arg_count(args) > 1 {
        let m = eval_arg_lenient(env, args, 1)?;
        if m.is_nothing() {
            None
        } else {
            Some(m)
        }
    } else {
        None
    };
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "chunk")?;
    let mut out = Vec::new();
    for c in items.chunks(chunk_size) {
        let mut chunk_items = c.to_vec();
        if let Some(f) = &filler {
            while chunk_items.len() < chunk_size {
                chunk_items.push(f.clone());
            }
        }
        out.push(TModel::from_sequence(chunk_items));
    }
    Ok(Some(TModel::from_sequence(out)))
}

/// ?seq_index_of(searched[, fromIndex]) —— Java seq_index_ofBI(true)
pub fn seq_index_of(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    seq_index_of_impl(env, target, args, true)
}

/// ?seq_last_index_of(searched[, fromIndex]) —— Java seq_index_ofBI(false)
pub fn seq_last_index_of(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    seq_index_of_impl(env, target, args, false)
}

fn seq_index_of_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    find_first: bool,
) -> Result<Option<TModel>> {
    let bi = if find_first {
        "seq_index_of"
    } else {
        "seq_last_index_of"
    };
    check_arg_count(bi, args, 1, 2)?;
    let t = crate::core::eval::eval(env, target)?;
    let items = seq_or_collection_items(&t, bi)?;
    // 被搜项（Java exec :430-431：缺失变量 → null → modelsEqual null → false → -1）
    let searched = eval_arg_lenient(env, args, 0)?;
    let from: i64 = if arg_count(args) > 1 {
        // Java exec :434-436：getNumberMethodArg(args, 1).intValue() 向零截断
        let n = method_number_arg(env, args, 1, bi)?;
        crate::core::eval::trunc_i64(&n).unwrap_or(0)
    } else if find_first {
        0
    } else {
        items.len() as i64 - 1
    };
    let found = if find_first {
        // Java findInSeq(searched, startIndex) :477-492：startIndex >= size → -1；
        // startIndex < 0 → 0
        let start = from.max(0) as usize;
        if start >= items.len() {
            -1
        } else {
            let mut f = -1;
            for (i, item) in items.iter().enumerate().skip(start) {
                if models_equal(i, item, &searched, Some(env))? {
                    f = i as i64;
                    break;
                }
            }
            f
        }
    } else {
        // Java findInSeq(searched, startIndex)（findFirst=false）：startIndex >= size
        // → size-1；startIndex < 0 → -1
        if from < 0 {
            -1
        } else {
            let start = from.min(items.len() as i64 - 1).max(0) as usize;
            let mut f = -1;
            for (i, item) in items.iter().enumerate().take(start + 1).rev() {
                if models_equal(i, item, &searched, Some(env))? {
                    f = i as i64;
                    break;
                }
            }
            f
        }
    };
    Ok(Some(TModel::from_number(TNumber::from_i64(found))))
}

/// 序列内建宽松相等（Java SequenceBuiltins.modelsEqual :937-954 → EvalUtil.compare
/// 参数 (typeMismatchMeansNotEqual, leftNullReturnsFalse, rightNullReturnsFalse) =
/// (true, true, true)）：null/缺失 → false；数字按值；字符串按 NFKC 归一化相等
/// （v1 近似：原文比较——非 ASCII 归一化差异属 P4，同 sort 的 Collator 近似）；
/// 日期同型比毫秒、异型/未知型报错（EvalUtil.compare :221-258）；布尔相同；
/// 其余类型组合 → false。比较异常按 :950-952 包装索引信息。
pub(crate) fn models_equal(
    seq_item_index: usize,
    seq_item: &TModel,
    searched: &TModel,
    env: Option<&mut Environment>,
) -> Result<bool> {
    models_equal_inner(seq_item, searched, env).map_err(|e| {
        TemplateError::misc(format!(
            "This error has occurred when comparing sequence item at 0-based index {seq_item_index} to the searched item:\n{e}"
        ))
    })
}

fn models_equal_inner(a: &TModel, b: &TModel, mut env: Option<&mut Environment>) -> Result<bool> {
    // Java EvalUtil.compare：classic-compatible 模式（FREEMARKER-227 seq_contains 宽松比较）
    let classic = env
        .as_ref()
        .map(|e| e.settings.classic_compatible)
        .unwrap_or(false);
    if a.is_nothing() || b.is_nothing() {
        // Java :192-223：left/rightNullReturnsFalse → false（不报错）；
        // classic 模式 null → EMPTY_STRING（EvalUtil.compare :193-205）再比较
        if classic {
            let e = env.as_deref_mut().expect("classic 模式必有 env");
            return Ok(model_to_string(e, a)? == model_to_string(e, b)?);
        }
        return Ok(false);
    }
    if a.is_number() && b.is_number() {
        return Ok(a
            .get_number()?
            .as_big_decimal()
            .cmp(&b.get_number()?.as_big_decimal())
            == Ordering::Equal);
    }
    if a.is_date() && b.is_date() {
        let ld = a.get_date()?;
        let rd = b.get_date()?;
        if ld.kind == DateType::Unknown || rd.kind == DateType::Unknown {
            // Java :227-238：未知日期类型 → 报错（sideName = left/right）
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
            // Java :240-250
            return Err(TemplateError::misc(format!(
                "Can't compare dates of different types. Left date type is {}, right date type is {}.",
                java_date_type_name(ld.kind),
                java_date_type_name(rd.kind)
            )));
        }
        return Ok(ld.dt.cmp(&rd.dt) == Ordering::Equal);
    }
    if a.is_scalar() && b.is_scalar() {
        // Java :282-286：ICI >= 2.3.33 按 NFKC 归一化后 compareTo（v1 近似：原文比较）
        return Ok(a.get_scalar()? == b.get_scalar()?);
    }
    if a.is_boolean() && b.is_boolean() {
        return Ok(a.get_boolean()? == b.get_boolean()?);
    }
    if classic {
        // Java :303-308：classic 兼容 → 双方转纯文本比较（coerceModelToPlainText）
        let e = env.as_mut().expect("classic 模式必有 env");
        return Ok(model_to_string(e, a)? == model_to_string(e, b)?);
    }
    // Java :303-326：typeMismatchMeansNotEqual → EQUALS → false（不报错）
    Ok(false)
}

/// Java TemplateDateModel.TYPE_NAMES（:58-63："UNKNOWN","TIME","DATE","DATETIME"）
fn java_date_type_name(kind: DateType) -> &'static str {
    match kind {
        DateType::Date => "DATE",
        DateType::Time => "TIME",
        DateType::DateTime => "DATETIME",
        DateType::Unknown => "UNKNOWN",
    }
}

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

/// 按键排序（Java sortBI.sort :703-839）：首键定类型（标量→字符串/数字/日期/布尔，
/// 其余 → "Values used for sorting must be..."），后续键按该类型强转、不一致 →
/// newInconsistentSortKeyTypeException :670-688（value/key value 措辞按 keyNamesLn）；
/// 排序稳定（Java Collections.sort 稳定）。字符串键按 Collator 排序（v1 用
/// UTF-16 码元序近似，同 eval.rs utf16_cmp）。
/// 强转后的排序键（Java sortBI.sort 的 KVP.key：按 keyType 定型，:628-636）
enum CastKey {
    Number(TNumber),
    String(String),
    Date(DateValue),
    Boolean(bool),
}

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
fn collator_cmp(a: &str, b: &str) -> Ordering {
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
fn utf16_cmp(a: &str, b: &str) -> Ordering {
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
    let items = seq_or_collection_items(&t, bi)?;
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

/// ?sequence —— Java BuiltInsForSequences.sequence：目标已是序列/集合 → 原样返回；
/// 字符串 → 字符序列（每字符一个单字符串）；其余 → 报错。
pub fn sequence(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("sequence", args, 0, 0)?;
    let t = crate::core::eval::eval(env, target)?;
    // 已是序列或集合 → 原样返回
    if t.is_sequence() || t.is_collection() {
        return Ok(Some(t));
    }
    // 字符串 → 字符序列（每字符一个单字符串）
    if t.is_scalar() {
        let s = t.get_scalar()?;
        let chars: Vec<TModel> = s
            .chars()
            .map(|c| TModel::from_scalar(c.to_string()))
            .collect();
        return Ok(Some(TModel::from_sequence(chars)));
    }
    Err(TemplateError::misc(format!(
        "?sequence is not applicable to a {} value",
        t.type_name
    )))
}

/// ?join —— Java joinBI（BuiltInsForSequences.java:191-265）：1-3 参数
/// （separator / whenEmpty / afterLast，checkMethodArgCount(args, 1, 3)）；
/// null（nothing）元素跳过（:225 `if (item != null)`，idx 仍递增）；
/// 逐项字符串转换错误包装失败索引（:230-238，EMBEDDED_MESSAGE_BEGIN/END）；
/// 右无界数值范围拒绝（:256 checkNotRightUnboundedNumericalRange，:929-935）
pub fn join(
    env: &mut crate::core::Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    // Java joinBI（BuiltInsForSequences.java:191-265）：1-3 参数
    // （separator / whenEmpty / afterLast，checkMethodArgCount(args, 1, 3)）；
    // null（nothing）元素跳过（:225 `if (item != null)`，idx 仍递增）；
    // 逐项字符串转换错误包装失败索引（:230-238，EMBEDDED_MESSAGE_BEGIN/END）；
    // 右无界数值范围拒绝（:256 checkNotRightUnboundedNumericalRange，:929-935）
    if let Some(a) = args {
        if a.is_empty() || a.len() > 3 {
            // Java _MessageUtil.newArgCntError（BuiltIn.java:450-452）：
            // "?join(...) expects 1 to 3 arguments but has received none./{n}."
            return Err(TemplateError::misc(format!(
                "?join(...) expects 1 to 3 arguments but has received {}.",
                if a.is_empty() {
                    "none".to_string()
                } else {
                    a.len().to_string()
                }
            )));
        }
    }
    let arg = args.and_then(|a| a.first()).ok_or_else(|| {
        TemplateError::misc("?join(...) expects 1 to 3 arguments but has received none.")
    })?;
    let m = eval(env, target)?;
    if m.range.as_ref().is_some_and(|r| r.unbounded) {
        return Err(TemplateError::misc(
                    "The input sequence is a right-unbounded numerical range, thus, it's infinitely long, and can't processed with this built-in.",
                ));
    }
    let sep = eval(env, arg)?.get_scalar()?;
    let when_empty = match args.and_then(|a| a.get(1)) {
        Some(a) => Some(eval(env, a)?.get_scalar()?),
        None => None,
    };
    let after_last = match args.and_then(|a| a.get(2)) {
        Some(a) => Some(eval(env, a)?.get_scalar()?),
        None => None,
    };
    let mut out = String::new();
    let mut had_item = false;
    let mut idx = 0usize;
    // Java :251-263：TemplateCollectionModel 优先 → 惰性迭代器；
    // 其次 TemplateSequenceModel → CollectionAndSequence 包装
    if let Some(c) = &m.collection {
        for v in c.iterator()? {
            join_append_item(env, &v?, &mut out, &sep, &mut had_item, idx)?;
            idx += 1;
        }
    } else if let Some(s) = &m.sequence {
        let n = s.size()?;
        for i in 0..n {
            let item = s.get(i)?;
            join_append_item(env, &item, &mut out, &sep, &mut had_item, idx)?;
            idx += 1;
        }
    } else {
        return Err(TemplateError::misc(format!(
            "?join is not applicable to a {} value",
            m.type_name
        )));
    }
    // Java :242-246：hadItem → afterLast；否则 → whenEmpty
    if had_item {
        if let Some(al) = after_last {
            out.push_str(&al);
        }
    } else if let Some(we) = when_empty {
        out.push_str(&we);
    }
    Ok(Some(TModel::from_scalar(out)))
}

/// ?reverse —— Java reverseBI（BuiltInsForSequences.java）：序列倒序/字符串倒序
pub fn reverse(
    env: &mut crate::core::Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = eval(env, target)?;
    if let Some(seq) = &m.sequence {
        let n = seq.size()?;
        let mut v = Vec::with_capacity(n);
        for i in (0..n).rev() {
            v.push(seq.get(i)?);
        }
        return Ok(Some(TModel::from_sequence(v)));
    }
    if let Some(sc) = &m.scalar {
        return Ok(Some(TModel::from_scalar(
            sc.as_string()?.chars().rev().collect(),
        )));
    }
    Err(TemplateError::misc(format!(
        "?reverse is not applicable to a {} value",
        m.type_name
    )))
}

/// ?seq_contains —— Java seq_containsBI（BuiltInsForSequences.java:308-380）：
/// checkMethodArgCount(1)；序列优先（2.3.x BC），否则集合迭代；
/// 参数缺失变量 → null → modelsEqual false
pub fn seq_contains(
    env: &mut crate::core::Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    // Java seq_containsBI（BuiltInsForSequences.java:308-380）：checkMethodArgCount(1)；
    // 序列优先（2.3.x BC），否则集合迭代；参数缺失变量 → null → modelsEqual false
    crate::core::eval_util::check_arg_count("seq_contains", args, 1, 1)?;
    let m = eval(env, target)?;
    let needle = crate::builtins::sequences::eval_arg_lenient(env, args, 0)?;
    let items = crate::builtins::sequences::seq_or_collection_items(&m, "seq_contains")?;
    for (i, item) in items.iter().enumerate() {
        if crate::builtins::sequences::models_equal(i, item, &needle, Some(env))? {
            return Ok(Some(TModel::from_boolean(true)));
        }
    }
    Ok(Some(TModel::from_boolean(false)))
}

fn join_append_item(
    env: &mut crate::core::Environment,
    item: &TModel,
    out: &mut String,
    sep: &str,
    had_item: &mut bool,
    idx: usize,
) -> Result<()> {
    if item.is_nothing() {
        return Ok(());
    }
    if *had_item {
        out.push_str(sep);
    } else {
        *had_item = true;
    }
    match model_to_string(env, item) {
        Ok(s) => {
            out.push_str(&s);
            Ok(())
        }
        Err(e) => Err(TemplateError::misc(format!(
            "\"?join\" failed at index {idx} with this error:\n\n---begin-message---\n{e}\n---end-message---"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
    use indexmap::IndexMap;
    use std::sync::Arc;

    /// 渲染 `${src}` 返回输出字符串（boolean_format=c、number_format=0.#########，
    /// 同 golden 用例设置）
    fn eval_out(root: DynValue, src: &str) -> Result<String> {
        let mut c = Configuration::new();
        c.settings.boolean_format = "c".to_string();
        c.settings.number_format = "0.#########".to_string();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", &format!("${{{src}}}"));
        let t = c.get_template("t.ftl")?;
        let root_model = SimpleObjectWrapper
            .wrap(&root)?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    /// 直接以 TModel 为根渲染（测试纯集合/未知日期类型等 DynValue 无法表达的模型）
    fn render_model(root_model: TModel, src: &str) -> Result<String> {
        let mut c = Configuration::new();
        c.settings.boolean_format = "c".to_string();
        c.settings.number_format = "0.#########".to_string();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", &format!("${{{src}}}"));
        let t = c.get_template("t.ftl")?;
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    /// 错误消息去位置/指令栈后缀（渲染层附加 "  [in template ...]" 位置段与
    /// "\n\n----\nFTL stack trace ..." 段——断言 Java 消息主体用）
    fn err_msg(e: &TemplateError) -> String {
        e.to_string()
            .split("  [in template")
            .next()
            .unwrap_or_default()
            .split("\n\n----\nFTL stack trace")
            .next()
            .unwrap_or_default()
            .to_string()
    }

    fn no_root() -> DynValue {
        DynValue::Map(vec![])
    }

    /// 含 null 项的序列（Java TemplateTestCase 的 listWithNull）
    fn list_with_null_root() -> DynValue {
        DynValue::Map(vec![(
            "listWithNull".into(),
            DynValue::List(vec![
                DynValue::Str("a".into()),
                DynValue::Null,
                DynValue::Str("c".into()),
            ]),
        )])
    }

    /// 1992-02-21 的日期模型（'yyyy-MM-dd'）
    fn date_1992() -> DateValue {
        DateValue {
            dt: chrono::DateTime::parse_from_str(
                "1992-02-21 00:00:00 +0000",
                "%Y-%m-%d %H:%M:%S %z",
            )
            .unwrap(),
            kind: DateType::Date,
            is_sql: false,
        }
    }

    #[test]
    fn utf16_order() {
        assert_eq!(utf16_cmp("a", "b"), Ordering::Less);
        assert_eq!(utf16_cmp("ab", "a"), Ordering::Greater);
    }

    #[test]
    fn collator_order() {
        // jar 实测（Locale.US）顺序
        assert_eq!(collator_cmp("aardvark", "Barbara"), Ordering::Less);
        assert_eq!(collator_cmp("Barbara", "beetroot"), Ordering::Less);
        assert_eq!(collator_cmp("barbara", "Barbara"), Ordering::Less);
        assert_eq!(collator_cmp("Barbara", "BARBARA"), Ordering::Less);
        assert_eq!(collator_cmp("aA", "Aa"), Ordering::Less);
        assert_eq!(collator_cmp("a", "A"), Ordering::Less);
        assert_eq!(collator_cmp("a", "ab"), Ordering::Less);
        assert_eq!(collator_cmp("ab", "a"), Ordering::Greater);
    }

    #[test]
    fn java_date_type_names() {
        assert_eq!(java_date_type_name(DateType::Date), "DATE");
        assert_eq!(java_date_type_name(DateType::Time), "TIME");
        assert_eq!(java_date_type_name(DateType::DateTime), "DATETIME");
        assert_eq!(java_date_type_name(DateType::Unknown), "UNKNOWN");
    }

    // ---- modelsEqual（Java SequenceBuiltins.modelsEqual :937-954）----

    #[test]
    fn models_equal_missing_and_mixed_types() {
        // null/缺失 → false（Java left/rightNullReturnsFalse）
        let nothing = TModel::nothing();
        let a = TModel::from_scalar("a".to_string());
        assert!(!models_equal(0, &nothing, &a, None).unwrap());
        assert!(!models_equal(0, &a, &nothing, None).unwrap());
        // 数字按值、字符串按内容、布尔相同
        assert!(models_equal(
            0,
            &TModel::from_number(TNumber::from_i64(1)),
            &TModel::from_number(TNumber::Decimal(bigdecimal::BigDecimal::from(1))),
            None
        )
        .unwrap());
        assert!(models_equal(
            0,
            &TModel::from_scalar("x".to_string()),
            &TModel::from_scalar("x".to_string()),
            None
        )
        .unwrap());
        assert!(models_equal(
            0,
            &TModel::from_boolean(true),
            &TModel::from_boolean(true),
            None
        )
        .unwrap());
        // 其余类型组合 → false（typeMismatchMeansNotEqual）
        assert!(!models_equal(
            0,
            &TModel::from_number(TNumber::from_i64(1)),
            &TModel::from_scalar("1".to_string()),
            None
        )
        .unwrap());
        assert!(!models_equal(
            0,
            &TModel::from_sequence(vec![]),
            &TModel::from_scalar("a".to_string()),
            None
        )
        .unwrap());
    }

    #[test]
    fn models_equal_dates() {
        // 同型日期比毫秒
        let d1 = date_1992();
        let d2 = DateValue {
            dt: d1.dt,
            kind: DateType::Date,
            is_sql: false,
        };
        assert!(models_equal(
            0,
            &TModel::from_date(d1.clone()),
            &TModel::from_date(d2),
            None
        )
        .unwrap());
        // 异型 → "Can't compare dates of different types"（Java EvalUtil.compare :240-250）
        let dtm = DateValue {
            dt: d1.dt,
            kind: DateType::DateTime,
            is_sql: false,
        };
        let err =
            models_equal(3, &TModel::from_date(d1), &TModel::from_date(dtm), None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "This error has occurred when comparing sequence item at 0-based index 3 to the searched item:\nCan't compare dates of different types. Left date type is DATE, right date type is DATETIME."
        );
    }

    #[test]
    fn seq_index_of_date_mismatch_message() {
        // golden 断言（assertFails "dates of different types"）：日期 vs 日期时间比较报错
        let root = DynValue::Map(vec![(
            "x".into(),
            DynValue::List(vec![
                DynValue::Date(date_1992()),
                DynValue::Str("foo".into()),
            ]),
        )]);
        assert_eq!(
            eval_out(root.clone(), "x?seq_index_of('foo')").unwrap(),
            "1"
        );
        assert_eq!(
            eval_out(
                root.clone(),
                "x?seq_index_of('1992-02-21'?date('yyyy-MM-dd'))"
            )
            .unwrap(),
            "0"
        );
        let err = eval_out(
            root.clone(),
            "x?seq_index_of('1992-02-21 00:00:00'?datetime('yyyy-MM-dd HH:mm:ss'))",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("dates of different types"),
            "{err}"
        );
    }

    #[test]
    fn seq_index_of_missing_var_arg_flows_as_null() {
        // golden "These should throw exception, but for BC they don't"：
        // 缺失变量参数 → null → 不报错（Java MethodCall 参数求值返回 null）
        let root = list_with_null_root();
        assert_eq!(
            eval_out(root.clone(), "listWithNull?seq_contains(noSuchVar)?c").unwrap(),
            "false"
        );
        assert_eq!(
            eval_out(root.clone(), "listWithNull?seq_index_of(noSuchVar)").unwrap(),
            "-1"
        );
        assert_eq!(
            eval_out(root.clone(), "listWithNull?seq_last_index_of(noSuchVar)").unwrap(),
            "-1"
        );
        // null 项跳过（Java leftNullReturnsFalse）
        assert_eq!(
            eval_out(root.clone(), "listWithNull?seq_contains('c')?c").unwrap(),
            "true"
        );
        assert_eq!(
            eval_out(root.clone(), "listWithNull?seq_index_of('c')").unwrap(),
            "2"
        );
        assert_eq!(
            eval_out(root.clone(), "listWithNull?seq_last_index_of('a')").unwrap(),
            "0"
        );
    }

    #[test]
    fn seq_index_of_from_index() {
        // 负数 fromIndex → 0；>= size → -1（Java findInSeq :477-492）
        let root = DynValue::Map(vec![(
            "names".into(),
            DynValue::List(vec![
                DynValue::Str("Joe".into()),
                DynValue::Str("Fred".into()),
                DynValue::Str("Joe".into()),
                DynValue::Str("Susan".into()),
            ]),
        )]);
        assert_eq!(
            eval_out(root.clone(), "names?seq_index_of('Joe', -2)").unwrap(),
            "0"
        );
        assert_eq!(
            eval_out(root.clone(), "names?seq_index_of('Joe', 1)").unwrap(),
            "2"
        );
        assert_eq!(
            eval_out(root.clone(), "names?seq_index_of('Joe', 4)").unwrap(),
            "-1"
        );
        // seq_last_index_of：fromIndex >= size → 从尾；< 0 → -1
        assert_eq!(
            eval_out(root.clone(), "names?seq_last_index_of('Joe', 1)").unwrap(),
            "0"
        );
        assert_eq!(
            eval_out(root.clone(), "names?seq_last_index_of('Joe', 4)").unwrap(),
            "2"
        );
        assert_eq!(
            eval_out(root.clone(), "names?seq_last_index_of('Susan', 2)").unwrap(),
            "-1"
        );
        // fromIndex 非整数 → intValue() 向零截断（jar 实测 2.5 → 2、-0.5 → 0）
        assert_eq!(
            eval_out(no_root(), "[1,2,3,4]?seq_index_of(4, 2.5)").unwrap(),
            "3"
        );
        assert_eq!(
            eval_out(no_root(), "[1,2,3,4]?seq_index_of(1, -0.5)").unwrap(),
            "0"
        );
        // fromIndex 缺失变量 → "expects a number as argument #2, but received a Null."
        let err = eval_out(no_root(), "[1,2,3]?seq_index_of(1, noSuchVar)").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?seq_index_of(...) expects a number as argument #2, but received a Null."
        );
    }

    #[test]
    fn seq_index_of_arg_count() {
        let err = eval_out(no_root(), "[1,2,3]?seq_index_of(1, 0, 0)").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?seq_index_of(...) expects 1 or 2 arguments but has received 3."
        );
        let err = eval_out(no_root(), "[1,2,3]?seq_contains(1, 2)").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?seq_contains(...) expects 1 argument but has received 2."
        );
    }

    #[test]
    fn seq_builtins_on_collection() {
        // Java seq_index_ofBI.BIMethod :389-413：序列优先，否则集合迭代
        let root_model = TModel::from_hash(IndexMap::from([(
            "coll".to_string(),
            TModel::from_collection(vec![
                TModel::from_scalar("a".to_string()),
                TModel::from_scalar("b".to_string()),
                TModel::from_scalar("c".to_string()),
            ]),
        )]));
        assert_eq!(
            render_model(root_model.clone(), "coll?seq_index_of('b')").unwrap(),
            "1"
        );
        assert_eq!(
            render_model(root_model.clone(), "coll?seq_index_of('a', 1)").unwrap(),
            "-1"
        );
        assert_eq!(
            render_model(root_model.clone(), "coll?seq_last_index_of('a', 2)").unwrap(),
            "0"
        );
        assert_eq!(
            render_model(root_model.clone(), "coll?seq_contains('a')?c").unwrap(),
            "true"
        );
        // ?first 支持集合（Java firstBI.calculateResultForColletion :180-187）
        assert_eq!(render_model(root_model.clone(), "coll?first").unwrap(), "a");
    }

    #[test]
    fn first_last_empty_return_null() {
        // Java firstBI/lastBI：空 → null → 下游 InvalidReferenceException
        let err = eval_out(no_root(), "[]?first").unwrap_err();
        assert!(
            err.to_string().contains("has evaluated to null or missing"),
            "{err}"
        );
        let err = eval_out(no_root(), "[]?last").unwrap_err();
        assert!(
            err.to_string().contains("has evaluated to null or missing"),
            "{err}"
        );
    }

    #[test]
    fn chunk_semantics() {
        // 非整数 size 截断（Java intValue()；jar 实测 2.9 → 2 块）
        assert_eq!(
            eval_out(no_root(), "[1,2,3,4]?chunk(2.9)?size").unwrap(),
            "2"
        );
        // 截断后 < 1 → "must be at least 1."
        let err = eval_out(no_root(), "[1,2,3]?chunk(0.5)?size").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "The 1st argument to ?chunk (...) must be at least 1."
        );
        // 非数字 size → 参数类型错误（Java newMethodArgMustBeNumberException）
        let err = eval_out(no_root(), "[1,2,3]?chunk('x')?size").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?chunk(...) expects a number as argument #1, but received a string."
        );
        // size 缺失变量 → "received a Null."（jar 实测）
        let err = eval_out(no_root(), "[1,2,3]?chunk(noSuchVar)?size").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?chunk(...) expects a number as argument #1, but received a Null."
        );
        // filler 缺失变量 → null → 不补齐（Java :78）
        assert_eq!(
            eval_out(no_root(), "[1,2]?chunk(1, noSuchVar)?size").unwrap(),
            "2"
        );
        // filler 补齐
        let root = DynValue::Map(vec![(
            "rows".into(),
            DynValue::List(vec![
                DynValue::List(vec![DynValue::Int(1)]),
                DynValue::List(vec![DynValue::Int(2), DynValue::Str("-".into())]),
            ]),
        )]);
        let _ = root;
        assert_eq!(
            eval_out(no_root(), "([1,2,3]?chunk(2, '-')?first)?size").unwrap(),
            "2"
        );
        assert_eq!(
            eval_out(no_root(), "([1,2,3]?chunk(2, '-')?last)?size").unwrap(),
            "2"
        );
        assert_eq!(
            eval_out(no_root(), "([1,2,3]?chunk(2)?last)?size").unwrap(),
            "1"
        );
    }

    #[test]
    fn sort_by_messages_match_java() {
        let err = eval_out(no_root(), "[{'a':1}]?sort_by()").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?sort_by(...) expects 1 argument but has received none."
        );
        let err = eval_out(no_root(), "[{'a':1}]?sort_by(42)").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "The argument to ?sort_by(key) must be a string (the name of the subvariable), or a sequence of strings (the \"path\" to the subvariable)."
        );
        let err = eval_out(no_root(), "[{'a':1}]?sort_by([1, 2])").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "The argument to ?sort_by(key), when it's a sequence, must be a sequence of strings, but the item at index 0 is not a string."
        );
        let err = eval_out(no_root(), "[{'a':1}]?sort_by('b')").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?sort_by(...) failed at sequence index 0: The \"b\" subvariable was null or missing."
        );
        let err = eval_out(no_root(), "[1,2]?sort_by('a')").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?sort_by(...) failed at sequence index 0: Sequence items must be hashes when using ?sort_by.  subvariable is not a hash, so ?sort_by can't proceed with getting the \"a\" subvariable."
        );
        // 键类型不一致（Java newInconsistentSortKeyTypeException :670-688）
        let err = eval_out(no_root(), "[1, 'a']?sort").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?sort failed at sequence index 1 (0-based): All values in the sequence must be numbers, because the first value was that. However, the value of the current item isn't a number but a string."
        );
        let err = eval_out(no_root(), "[{'a':'x'},{'a':1}]?sort_by('a')").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "?sort_by(...) failed at sequence index 1 (0-based): All key values in the sequence must be strings, because the first key value was that. However, the key value of the current item isn't a string but a number."
        );
    }

    #[test]
    fn sort_orders() {
        // 字符串序（en_US Collator 近似：忽略大小写的主强度，小写 < 大写的第三强度；
        // 与 golden 用例及 jar 实测一致）
        assert_eq!(
            eval_out(
                no_root(),
                "(['whale','Barbara','zeppelin','aardvark','beetroot']?sort)?join(',')"
            )
            .unwrap(),
            "aardvark,Barbara,beetroot,whale,zeppelin"
        );
        assert_eq!(
            eval_out(no_root(), "(['a','A','aa','aA','Aa','AA']?sort)?join(',')").unwrap(),
            "a,A,aa,aA,Aa,AA"
        );
        assert_eq!(
            eval_out(
                no_root(),
                "(['Barbara','barbara','BARBARA']?sort)?join(',')"
            )
            .unwrap(),
            "barbara,Barbara,BARBARA"
        );
        // 数字序（跨数值类型）
        assert_eq!(
            eval_out(no_root(), "[123?byte, 543, -324, -34?float, 0.11, 0, 111?int, 0.1?double, 1, 5]?sort?join(',')").unwrap(),
            "-324,-34,0,0.1,0.11,1,5,111,123,543"
        );
        // 布尔序（false < true，Java BooleanKVPComparator）
        assert_eq!(
            eval_out(no_root(), "([true,false,false,true]?sort)?first?c").unwrap(),
            "false"
        );
        // 日期序（DateKVPComparator 按毫秒）
        assert_eq!(
            eval_out(
                no_root(),
                "(['1999-01-20'?date('yyyy-MM-dd'), '1998-02-20'?date('yyyy-MM-dd')]?sort)?first?string('yyyy-MM-dd')"
            )
            .unwrap(),
            "1998-02-20"
        );
        // 空序列 → 原模型（?size == 0）
        assert_eq!(eval_out(no_root(), "([]?sort)?size").unwrap(), "0");
    }

    #[test]
    fn min_max_semantics() {
        assert_eq!(eval_out(no_root(), "[3,1,2]?max").unwrap(), "3");
        assert_eq!(eval_out(no_root(), "[3,1,2]?min").unwrap(), "1");
        // 空 → null → 下游 InvalidReferenceException（Java MinOrMaxBI :999-1011）
        let err = eval_out(no_root(), "[]?max").unwrap_err();
        assert!(
            err.to_string().contains("has evaluated to null or missing"),
            "{err}"
        );
        // null 元素跳过（jar 实测：跳过 null 后比较 'a' vs 'c' 字符串 → 报错）
        let err = eval_out(list_with_null_root(), "listWithNull?max").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "Can't use operator \"greater-than\" on string values."
        );
        // 字符串：大小比较报错（Java EvalUtil.compare :262-266，operatorString null
        // → "greater-than"/"less-than"）
        let err = eval_out(no_root(), "['a','b']?max").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "Can't use operator \"greater-than\" on string values."
        );
        let err = eval_out(no_root(), "['a','b']?min").unwrap_err();
        assert_eq!(
            err_msg(&err),
            "Can't use operator \"less-than\" on string values."
        );
        // 类型不匹配（Java :307-326，typeMismatchMeansNotEqual=false → 报错）
        let err = eval_out(no_root(), "[1,'a']?max").unwrap_err();
        assert!(
            err.to_string()
                .contains("Can't compare values of these types"),
            "{err}"
        );
        // 日期异型（Java :240-250，大写类型名）
        let err = eval_out(
            no_root(),
            "['1992-02-21'?date('yyyy-MM-dd'), '1992-02-21 00:00:00'?datetime('yyyy-MM-dd HH:mm:ss')]?max",
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("Left date type is DATETIME, right date type is DATE"),
            "{err}"
        );
        // 右无界范围拒绝（Java :975 checkNotRightUnboundedNumericalRange）
        let err = eval_out(no_root(), "(1..)?max").unwrap_err();
        assert!(err.to_string().contains("right-unbounded"), "{err}");
    }

    #[test]
    fn min_max_on_collection() {
        let root_model = TModel::from_hash(IndexMap::from([(
            "coll".to_string(),
            TModel::from_collection(vec![
                TModel::from_number(TNumber::from_i64(3)),
                TModel::from_number(TNumber::from_i64(1)),
                TModel::from_number(TNumber::from_i64(2)),
            ]),
        )]));
        assert_eq!(render_model(root_model.clone(), "coll?max").unwrap(), "3");
        assert_eq!(render_model(root_model.clone(), "coll?min").unwrap(), "1");
    }

    #[test]
    fn seq_contains_unknown_date_errors() {
        // Java EvalUtil.compare :227-238：未知日期类型比较报错
        let unknown = TModel::from_date(DateValue {
            dt: date_1992().dt,
            kind: DateType::Unknown,
            is_sql: false,
        });
        let root_model = TModel::from_hash(IndexMap::from([("u".to_string(), unknown)]));
        let err = render_model(root_model, "[u]?seq_contains(u)?c").unwrap_err();
        assert!(
            err.to_string()
                .contains("value of the comparison is a date-like value where it's not known"),
            "{err}"
        );
    }

    // ---- ?sequence ----

    #[test]
    fn sequence_on_sequence() {
        // 已是序列 → 原样返回
        assert_eq!(eval_out(no_root(), "[1,2,3]?sequence?size").unwrap(), "3");
        assert_eq!(eval_out(no_root(), "([1,2]?sequence)?first").unwrap(), "1");
    }

    #[test]
    fn sequence_on_string() {
        // 字符串 → 字符序列
        assert_eq!(eval_out(no_root(), "'abc'?sequence?size").unwrap(), "3");
        assert_eq!(
            eval_out(no_root(), "('abc'?sequence)?join(',')").unwrap(),
            "a,b,c"
        );
        assert_eq!(eval_out(no_root(), "('abc'?sequence)?first").unwrap(), "a");
        assert_eq!(eval_out(no_root(), "('abc'?sequence)?last").unwrap(), "c");
    }

    #[test]
    fn sequence_on_collection() {
        // 集合 → 原样返回
        let root_model = TModel::from_hash(IndexMap::from([(
            "coll".to_string(),
            TModel::from_collection(vec![
                TModel::from_scalar("x".to_string()),
                TModel::from_scalar("y".to_string()),
                TModel::from_scalar("z".to_string()),
            ]),
        )]));
        // ?seq_contains 适用于序列和集合
        assert_eq!(
            render_model(root_model.clone(), "coll?sequence?seq_contains('y')?c").unwrap(),
            "true"
        );
        assert_eq!(
            render_model(root_model.clone(), "coll?sequence?seq_contains('w')?c").unwrap(),
            "false"
        );
    }

    #[test]
    fn sequence_on_number_errors() {
        // 数字 → 报错
        let err = eval_out(no_root(), "42?sequence").unwrap_err();
        assert!(
            err.to_string()
                .contains("?sequence is not applicable to a number value"),
            "{err}"
        );
    }

    #[test]
    fn sequence_on_empty_string() {
        // 空字符串 → 空字符序列
        assert_eq!(eval_out(no_root(), "''?sequence?size").unwrap(), "0");
    }
}
