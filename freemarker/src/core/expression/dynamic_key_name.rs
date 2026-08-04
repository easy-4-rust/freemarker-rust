//! 动态键访问 —— 对应 Java `freemarker.core.DynamicKeyName`
//! （`_eval` :69-93：数字键 → 序列/字符串索引；字符串键 → 哈希；范围键 → 切片）
//! 范围键切片（`slice_with_range`）对应 Java dealWithRangeKey :183-334

use crate::core::environment::model_to_string;
use crate::core::eval::{eval, trunc_i64};
use crate::core::expression::dot_builtin_chain;
use crate::core::expression::eval_builtin;
use crate::core::{Expr, ExprKind};
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 动态键访问表达式（对应 DynamicKeyName.java；解析器经 `ExprKind::DynKey` 承载）
pub struct DynamicKeyName {
    pub target: Expr,
    pub key: Expr,
}

impl DynamicKeyName {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: Expr, key: Expr) -> Self {
        DynamicKeyName { target, key }
    }

    /// 求值（Java `_eval`）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_dyn_key(env, &self.target, &self.key)
    }
}

fn eval_dyn_key(env: &mut crate::core::Environment, target: &Expr, key: &Expr) -> Result<TModel> {
    // `date?string[""]` / `date?datetime["xs"]`：格式化器哈希访问（Java DateFormatter
    // 实现 TemplateHashModel.get(key)）；把字符串键并入内建参数
    if let Some((inner, bname, mut names)) = dot_builtin_chain(target) {
        if let Ok(k) = eval(env, key) {
            if let Ok(k) = k.get_scalar() {
                names.push(k);
                let args: Vec<Expr> = names
                    .iter()
                    .map(|n| Expr::new(ExprKind::Str(n.clone()), target.span))
                    .collect();
                return eval_builtin(env, &inner, &bname, &Some(args));
            }
        }
        // 非字符串键：Java 同样报错（格式化器 get 只接受字符串）→ 落常规路径
    }
    let t = eval(env, target)?;
    if t.is_nothing() {
        // Java Dot._eval / DynamicKeyName._eval：目标 null → classic 兼容模式继续
        // 传播 null（noSuchVar.foo.bar 整链求值为 null）；strict 模式 InvalidReference
        if env.settings.classic_compatible {
            return Ok(TModel::nothing());
        }
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    let k = eval(env, key)?;
    if let Ok(n) = k.get_number() {
        // 数字键（Java dealWithNumericalKey :98-160）
        let idx = trunc_i64(&n).ok_or_else(|| {
            TemplateError::misc(format!(
                "The index {} is out of the range of representable integers",
                n.to_plain_string()
            ))
        })?;
        if idx < 0 {
            // Java dealWithNumericalKey（DynamicKeyName.java:98-147）：序列目标把负下标
            // 交给模型 get()——RangeModel 抛 "Range item index -1 is out of bounds."
            // （RangeModel.java:29-31）；SimpleSequence.get 越界返回 null（→ 下游
            // InvalidReferenceException "has evaluated to null or missing"）；
            // 仅字符串目标报 "Negative index not allowed"（DynamicKeyName.java:141-143）
            if t.range.is_some() {
                return Err(TemplateError::misc(format!(
                    "Range item index {idx} is out of bounds."
                )));
            }
            if t.sequence.is_some() {
                return Err(TemplateError::invalid_reference(format!(
                    "{}[{}]",
                    crate::core::environment::expr_desc(target),
                    crate::core::environment::expr_desc(key)
                )));
            }
            return Err(TemplateError::misc(format!(
                "Negative index not allowed: {idx}"
            )));
        }
        let i = idx as usize;
        if let Some(seq) = &t.sequence {
            let size = seq.size()?;
            if i < size {
                return seq.get(i);
            }
            // Java dealWithNumericalKey（DynamicKeyName.java:112-117）：越界返回 null
            // （含 RangeModel——BoundedRangeModel/NonListable 均实现 TemplateSequenceModel；
            // "Range item index ... is out of bounds." 仅负下标路径，见上）
            return Ok(TModel::nothing());
        }
        // Java NodeModel 实现 TemplateSequenceModel（NodeModel.java:415-423）：
        // size()=1、get(0)=自身——`node[0]` 返回节点自身（`doc.*[0]`/`r["N:t1"][0]`
        // 即此语义）；越界 → null（缺失）
        if t.node.is_some() {
            return if i == 0 {
                Ok(t.clone())
            } else {
                Ok(TModel::nothing())
            };
        }
        // Java 2.3.34 dealWithNumericalKey :121-147 回退：目标经
        // evalAndCoerceToPlainText 强制转字符串后按下标取单字符——数字/布尔/日期等
        // 非序列目标均走此路径（`${true[0]}` → 布尔强制转字符串按 boolean_format
        // 报错，jar 实测 type_index_boolean 基线；`${1[0]}` → "1" 首字符）；
        // 越界 → "String index out of range: ..."（Java 捕获 StringIndexOutOfBounds
        // 后改报 FTL 消息）
        let text = match model_to_string(env, &t) {
            Ok(s) => s,
            // Java dealWithNumericalKey :157-166：evalAndCoerceToPlainText 抛
            // NonStringException → catch 后改抛 UnexpectedTypeException，expected
            // = "sequence or " + STRING_COERCABLE_TYPES_DESC；哈希目标附
            // "You had a numerical value inside the []..."（:163-165），集合目标由
            // UnexpectedTypeException 附 "you could convert it to a sequence" 提示
            // （UnexpectedTypeException.java:96-101，jar 实测 coll_index/hash_num_key）
            // 仅 NonStringException（TypeMismatch）转换；其余错误（如 boolean_format
            // 的 Misc）原样传播——type_index_boolean 基线逐字
            Err(_e @ TemplateError::TypeMismatch { .. }) => {
                let mut err = TemplateError::type_mismatch("sequence-or-string", t.type_name)
                    .with_expected_phrase(
                        "a sequence or string or something automatically convertible to string (number, date or boolean)",
                    )
                    .with_blame_at(
                        "...[...]",
                        "left-hand operand",
                        &crate::core::environment::expr_desc(target),
                        &env.current_template_name,
                        target.span,
                    );
                if t.hash.is_some() {
                    err = err.with_tip("You had a numerical value inside the []. Currently that's only supported for sequences (lists) and strings. To get a Map item with a non-string key, use myMap?api.get(myKey).");
                }
                if t.collection.is_some() {
                    err = err.with_tip("As the problematic value contains a collection of items, you could convert it to a sequence like someValue?sequence. Be sure though that you won't have a large number of items, as all will be held in memory the same time.");
                }
                return Err(err);
            }
            Err(e) => return Err(e),
        };
        return match text.chars().nth(i) {
            Some(c) => Ok(TModel::from_scalar(c.to_string())),
            None => Err(TemplateError::misc(format!(
                "String index out of range: The index was {} (0-based), but the length of the string is only {}.",
                i,
                text.chars().count()
            ))),
        };
    }
    if let Some(r) = &k.range {
        // 范围键（Java DynamicKeyName 的 RangeModel 分支：SequenceOrStringSlicer，
        // 负下标按长度回绕；越界报错）
        return slice_with_range(
            &t,
            r,
            &crate::core::environment::expr_desc(target),
            &crate::core::environment::expr_desc(key),
        );
    }
    if let Ok(s) = k.get_scalar() {
        // 字符串键（Java dealWithStringKey :162-167）；键缺失 → Java
        // SimpleHash.get 返回 null 不抛 → Ok(nothing)
        // 节点哈希角色（Java NodeModel 的 DynamicKeyName：子元素名/@attr/@@key/XPath）
        if let Some(nh) = &t.node_hash {
            return Ok(nh.get(env, &s)?.unwrap_or_else(TModel::nothing));
        }
        if let Some(h) = &t.hash {
            return Ok(h.get(&s)?.unwrap_or_else(TModel::nothing));
        }
        return Err(TemplateError::type_mismatch("hash", t.type_name));
    }
    // Java UnexpectedTypeException（key 既非数字也非字符串）
    Err(TemplateError::type_mismatch(
        "number, range, or string",
        k.type_name,
    ))
}

fn slice_with_range(
    t: &TModel,
    r: &crate::core::RangeSpec,
    _td: &str,
    _kd: &str,
) -> Result<TModel> {
    let (target_size, is_str): (i64, bool) = if let Some(seq) = &t.sequence {
        (seq.size()? as i64, false)
    } else if let Some(s) = &t.scalar {
        (s.as_string()?.chars().count() as i64, true)
    } else {
        return Err(TemplateError::type_mismatch("sequence", t.type_name));
    };
    let step: i64 = if r.ascending { 1 } else { -1 };
    // 空有界范围 → 空结果（Java :207-210：不含非法下标，可接受越界起始）
    if !r.unbounded && r.count == 0 {
        return Ok(empty_slice_result(t));
    }
    let first = r.start;
    if first < 0 {
        return Err(TemplateError::misc(format!(
            "Negative range start index ({first}) isn't allowed for a range used for slicing."
        )));
    }
    // 起始越界（Java :224-236：自适应递增可 == 目标长度，其余 >= 即错）
    let start_ok = if r.adaptive && step == 1 {
        first <= target_size
    } else {
        first < target_size
    };
    if !start_ok {
        return Err(TemplateError::misc(format!(
            "Range start index {first} is out of bounds, because the sliced {} has only {target_size} {}(s). (Note that indices are 0-based).",
            if is_str { "string" } else { "sequence" },
            if is_str { "character" } else { "element" }
        )));
    }
    // 结果长度（Java :238-269）
    let result_size: i64 = if r.unbounded {
        target_size - first
    } else {
        let last = first + (r.count as i64 - 1) * step;
        if last < 0 {
            if !r.adaptive {
                return Err(TemplateError::misc(format!(
                    "Negative range end index ({last}) isn't allowed for a range used for slicing."
                )));
            }
            first + 1
        } else if last >= target_size {
            if !r.adaptive {
                return Err(TemplateError::misc(format!(
                    "Range end index {last} is out of bounds, because the sliced {} has only {target_size} {}(s). (Note that indices are 0-based).",
                    if is_str { "string" } else { "sequence" },
                    if is_str { "character" } else { "element" }
                )));
            }
            (target_size - first).abs()
        } else {
            r.count as i64
        }
    };
    if result_size == 0 {
        return Ok(empty_slice_result(t));
    }
    // 字符串降序切片 → 报错（Java :323-334；resultSize==1 允许，如 `0..*-1`）。
    // 旧版 bug 模拟：`a..b` 闭区间范围（isAffectedByStringSlicingBug）且结果长为 2
    // → "foo"[n .. n-1] 给 "" 而非报错（DynamicKeyName.java:322-330；FTL 2.4 修复前
    // 保持兼容；`..<`/`..!`/`..*` 运算符不受影响——template 注释 "But it isn't
    // emulated for operators introduced after 2.3.20"）
    if is_str && step < 0 && result_size > 1 {
        if r.affected_by_string_slicing_bug && result_size == 2 {
            return Ok(TModel::from_scalar(String::new()));
        }
        return Err(TemplateError::misc(format!(
            "Decreasing ranges aren't allowed for slicing strings (as it would give reversed text). The index range was: first = {first}, last = {}",
            first + (result_size - 1) * step
        )));
    }
    if is_str {
        let text = t.get_scalar()?;
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::new();
        let mut idx = first;
        for _ in 0..result_size {
            out.push(chars[idx as usize]);
            idx += step;
        }
        return Ok(TModel::from_scalar(out));
    }
    if let Some(seq) = &t.sequence {
        let mut out = Vec::new();
        let mut idx = first;
        for _ in 0..result_size {
            out.push(seq.get(idx as usize)?);
            idx += step;
        }
        return Ok(TModel::from_sequence(out));
    }
    Err(TemplateError::type_mismatch("sequence", t.type_name))
}

fn empty_slice_result(t: &TModel) -> TModel {
    if t.is_scalar() {
        TModel::from_scalar(String::new())
    } else {
        TModel::from_sequence(vec![])
    }
}
