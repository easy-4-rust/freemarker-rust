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
//!
//! 实现文件按职责拆分（`#[path]` 聚合，参照 parser/grammar.rs 模式）：
//! - built_ins_for_sequences_sort.rs —— sort/sort_by/Collator 排序键/min/max
//! - built_ins_for_sequences_tests.rs —— 测试（#[cfg(test)]）

use crate::core::environment::{model_to_string, BodyCtx, LambdaValue, LocalEntry};
use crate::core::eval;
use crate::core::eval_util::{arg_count, check_arg_count};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::value::{DateType, TNumber};
use std::cmp::Ordering;
use std::rc::Rc;

#[path = "built_ins_for_sequences_sort.rs"]
mod built_ins_for_sequences_sort;
#[cfg(test)]
#[path = "built_ins_for_sequences_tests.rs"]
mod built_ins_for_sequences_tests;

pub use self::built_ins_for_sequences_sort::{max, min, sort, sort_by};

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
pub(crate) fn java_date_type_name(kind: DateType) -> &'static str {
    match kind {
        DateType::Date => "DATE",
        DateType::Time => "TIME",
        DateType::DateTime => "DATETIME",
        DateType::Unknown => "UNKNOWN",
    }
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
    let needle = crate::core::built_ins_for_sequences::eval_arg_lenient(env, args, 0)?;
    let items = crate::core::built_ins_for_sequences::seq_or_collection_items(&m, "seq_contains")?;
    for (i, item) in items.iter().enumerate() {
        if crate::core::built_ins_for_sequences::models_equal(i, item, &needle, Some(env))? {
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
