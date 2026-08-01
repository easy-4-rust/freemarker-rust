//! 序列内建 —— 对应 Java `BuiltInsForSequences.java`（本文件为 eval.rs 内建集未覆盖的
//! 子集：chunk/filter/map/take_while/drop_while/sort/sort_by/min/max/seq_index_of/
//! seq_last_index_of；first/last/join/reverse/seq_contains 在 eval.rs）。
//!
//! 语义要点（Java 对照）：
//! - filter/map/take_while/drop_while 消费 lambda（Java ElementTransformer →
//!   LocalLambdaExpression.invokeLambdaDefinedFunction：以参数为新局部上下文求值体）；
//! - seq_index_of/seq_last_index_of（Java seq_index_ofBI(findFirst)）：modelsEqual 宽松相等，
//!   支持 fromIndex 参数；未找到 → -1；
//! - sort/sort_by（Java sortBI/sort_byBI）：键类型按首个元素定（数字/字符串/日期/布尔），
//!   跨类型报错；字符串按 Collator 排序（v1 用 UTF-16 码元序近似）；
//! - chunk(size[, filler])：子序列序列（Java ChunkedSequence）。

use crate::builtins::eval_util::{arg_count, arg_number, check_arg_count, models_equal};
use crate::core::environment::{BodyCtx, LambdaValue, LocalEntry};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::value::TNumber;
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

/// 序列元素枚举（Java TemplateSequenceModel；非序列报错）
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

/// ?chunk(size[, filler]) —— Java chunkBI：子序列序列
pub fn chunk(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("chunk", args, 1, 2)?;
    let size = arg_number(env, args, 0)?;
    let chunk_size = crate::core::eval::trunc_i64(&size).ok_or_else(|| {
        TemplateError::misc("The 1st argument to ?chunk (...) must be an integer")
    })?;
    if chunk_size < 1 {
        return Err(TemplateError::misc(
            "The 1st argument to ?chunk (...) must be at least 1.",
        ));
    }
    let chunk_size = chunk_size as usize;
    let filler: Option<TModel> = if arg_count(args) > 1 {
        let e = args.and_then(|a| a.get(1)).unwrap();
        Some(crate::core::eval::eval(env, e)?)
    } else {
        None
    };
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "chunk")?;
    let mut out = Vec::new();
    for c in items.chunks(chunk_size) {
        let mut chunk_items = c.to_vec();
        if filler.is_some() {
            while chunk_items.len() < chunk_size {
                chunk_items.push(filler.clone().unwrap());
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
    let items = sequence_items(&t, bi)?;
    let searched = {
        let e = args.and_then(|a| a.first()).unwrap();
        crate::core::eval::eval(env, e)?
    };
    let from: i64 = if arg_count(args) > 1 {
        let n = arg_number(env, args, 1)?;
        crate::core::eval::trunc_i64(&n).unwrap_or(0)
    } else if find_first {
        0
    } else {
        items.len() as i64 - 1
    };
    let found = if find_first {
        let start = from.max(0) as usize;
        if start >= items.len() {
            -1
        } else {
            let mut f = -1;
            for (i, item) in items.iter().enumerate().skip(start) {
                if models_equal(item, &searched)? {
                    f = i as i64;
                    break;
                }
            }
            f
        }
    } else {
        // Java findInSeq(searched, startIndex)：fromIndex >= size → 从尾部；< 0 → -1
        if from < 0 {
            -1
        } else {
            let start = from.min(items.len() as i64 - 1).max(0) as usize;
            let mut f = -1;
            for (i, item) in items.iter().enumerate().take(start + 1).rev() {
                if models_equal(item, &searched)? {
                    f = i as i64;
                    break;
                }
            }
            f
        }
    };
    Ok(Some(TModel::from_number(TNumber::from_i64(found))))
}

/// ?sort —— Java sortBI：直接按元素排序（键 = 元素本身）
pub fn sort(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("sort", args, 0, 0)?;
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "sort")?;
    if items.is_empty() {
        return Ok(Some(t));
    }
    let keys: Vec<TModel> = items.clone();
    Ok(Some(TModel::from_sequence(sort_by_keys(&items, &keys)?)))
}

/// ?sort_by(key[, key2, ...]) —— Java sort_byBI：按子变量键排序（键路径）
pub fn sort_by(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    if arg_count(args) < 1 {
        return Err(TemplateError::misc(
            "?sort_by expects at least one argument",
        ));
    }
    let mut key_names: Vec<String> = Vec::new();
    for i in 0..arg_count(args) {
        let a = args.unwrap()[i].clone();
        let m = crate::core::eval::eval(env, &a)?;
        if let Some(s) = &m.scalar {
            key_names.push(s.as_string()?);
        } else if m.is_sequence() {
            let seq = m.sequence.clone().unwrap();
            let n = seq.size()?;
            for j in 0..n {
                let item = seq.get(j)?;
                key_names.push(item.get_scalar().map_err(|_| {
                    TemplateError::misc(
                        "The argument to ?sort_by(key) must be a sequence of strings",
                    )
                })?);
            }
        } else {
            return Err(TemplateError::misc(
                "The argument to ?sort_by(key) must be a string or a sequence of strings",
            ));
        }
    }
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, "sort_by")?;
    if items.is_empty() {
        return Ok(Some(t));
    }
    // 取每个元素的键（Java：逐级取子变量；缺失/非哈希 → 报错）
    let mut keys = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let mut key = item.clone();
        for name in &key_names {
            let h = key.get_hash().map_err(|_| {
                TemplateError::misc(format!(
                    "?sort_by failed at sequence index {i}: Sequence items must be hashes when using ?sort_by."
                ))
            })?;
            key = h
                .get(name)?
                .ok_or_else(|| {
                    TemplateError::misc(format!(
                        "?sort_by failed at sequence index {i}: The \"{name}\" subvariable was null or missing."
                    ))
                })?;
        }
        keys.push(key);
    }
    Ok(Some(TModel::from_sequence(sort_by_keys(&items, &keys)?)))
}

/// 按键排序（Java sortBI.sort：键类型按首个元素定；数字/字符串/日期/布尔；
/// 排序稳定 —— Rust sort_by 稳定）
fn sort_by_keys(items: &[TModel], keys: &[TModel]) -> Result<Vec<TModel>> {
    let mut idx: Vec<usize> = (0..items.len()).collect();
    let key_type = classify_key(&keys[0])?;
    let mut err: Option<TemplateError> = None;
    idx.sort_by(|a, b| {
        if err.is_some() {
            return Ordering::Equal;
        }
        match compare_keys(&keys[*a], &keys[*b], key_type) {
            Ok(o) => o,
            Err(e) => {
                err = Some(e);
                Ordering::Equal
            }
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(idx.into_iter().map(|i| items[i].clone()).collect())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyType {
    Number,
    String,
    Date,
    Boolean,
}

fn classify_key(k: &TModel) -> Result<KeyType> {
    if k.is_scalar() {
        Ok(KeyType::String)
    } else if k.is_number() {
        Ok(KeyType::Number)
    } else if k.is_date() {
        Ok(KeyType::Date)
    } else if k.is_boolean() {
        Ok(KeyType::Boolean)
    } else {
        Err(TemplateError::misc(
            "Values used for sorting must be numbers, strings, date/times or booleans.",
        ))
    }
}

/// 键比较（Java KVPComparator 家族；字符串用 UTF-16 码元序近似 Collator）
fn compare_keys(a: &TModel, b: &TModel, t: KeyType) -> Result<Ordering> {
    match t {
        KeyType::Number => Ok(a
            .get_number()?
            .as_big_decimal()
            .cmp(&b.get_number()?.as_big_decimal())),
        KeyType::String => Ok(utf16_cmp(&a.get_scalar()?, &b.get_scalar()?)),
        KeyType::Date => Ok(a.get_date()?.dt.cmp(&b.get_date()?.dt)),
        KeyType::Boolean => Ok(a.get_boolean()?.cmp(&b.get_boolean()?)),
    }
}

/// UTF-16 码元字典序（同 eval.rs utf16_cmp）
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

/// ?min / ?max —— Java MinOrMaxBI：序列元素极值（数字比较；Java 也支持日期等可比较类型，
/// v1 按 EvalUtil.compare 语义）
fn min_max_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    want_max: bool,
) -> Result<Option<TModel>> {
    check_arg_count(if want_max { "max" } else { "min" }, args, 0, 0)?;
    let t = crate::core::eval::eval(env, target)?;
    let items = sequence_items(&t, if want_max { "max" } else { "min" })?;
    if items.is_empty() {
        return Err(TemplateError::misc(
            "The sequence is empty, ?max/?min failed",
        ));
    }
    let mut best = items[0].clone();
    for item in items.iter().skip(1) {
        let ord =
            crate::core::eval::compare_models(env, item, &best, crate::core::eval::CmpOp::Gt)?;
        if ord == want_max {
            best = item.clone();
        }
    }
    Ok(Some(best))
}

pub fn min(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    min_max_impl(env, target, args, false)
}

pub fn max(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    min_max_impl(env, target, args, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_order() {
        assert_eq!(utf16_cmp("a", "b"), Ordering::Less);
        assert_eq!(utf16_cmp("ab", "a"), Ordering::Greater);
    }
}
