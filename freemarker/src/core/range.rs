//! 范围表达式 —— 对应 Java `freemarker.core.Range`
//! （`_eval` :52-63 → BoundedRangeModel / ListableRightUnboundedRangeModel /
//! NonListableRightUnboundedRangeModel；有界实现为惰性序列角色）

use crate::core::eval::{eval, trunc_i64};
use crate::core::expression::{
    bounded_range_model, listable_right_unbounded_range_model,
    non_listable_right_unbounded_range_model,
};
use crate::core::{Expr, RangeKind};
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 范围表达式（对应 Range.java；解析器经 `ExprKind::Range` 承载；
/// END_SIZE_LIMITED 为 `..*`）
pub struct Range {
    pub start: Expr,
    pub end: Option<Expr>,
    pub kind: RangeKind,
}

impl Range {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(start: Expr, end: Option<Expr>, kind: RangeKind) -> Self {
        Range { start, end, kind }
    }

    /// 求值（Java `_eval` :52-63）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_range(env, &self.start, &self.end, self.kind)
    }
}

/// 范围（Java Range.java:52-63 `_eval` → BoundedRangeModel / ListableRightUnboundedRangeModel）
/// - `a..b` 含端；`a..<b` 排端；`a..*n` 从 a 起 n 个（Java END_SIZE_LIMITED：begin+rho 为末端）；
///   有界范围实现为惰性 BoundedRangeSeq（Java BoundedRangeModel 是 TemplateSequenceModel）；
/// - `a..*` 无界 → 惰性 RightUnboundedRange（v1 集合角色 + 迭代上限）。
fn eval_range(
    env: &mut crate::core::Environment,
    start: &Expr,
    end: &Option<Expr>,
    kind: RangeKind,
) -> Result<TModel> {
    let s = eval(env, start)?.get_number()?;
    let s_i = trunc_i64(&s).ok_or_else(|| {
        TemplateError::misc(format!(
            "The start of the range {} is not a representable integer",
            s.to_plain_string()
        ))
    })?;
    match end {
        Some(e) => {
            let e_m = eval(env, e)?.get_number()?;
            let (count, ascending) = match kind {
                // Java BoundedRangeModel(begin, lhoValue, inclusive, sizeLimited=false)
                RangeKind::Inclusive => {
                    let e_i = trunc_i64(&e_m).ok_or_else(|| {
                        TemplateError::misc("Range end is not a representable integer")
                    })?;
                    (((e_i - s_i).abs() + 1) as usize, s_i <= e_i)
                }
                RangeKind::Exclusive => {
                    let e_i = trunc_i64(&e_m).ok_or_else(|| {
                        TemplateError::misc("Range end is not a representable integer")
                    })?;
                    ((e_i - s_i).unsigned_abs() as usize, s_i <= e_i)
                }
                // Java END_SIZE_LIMITED：end = begin + rho；size = |rho|
                RangeKind::SizeLimited => {
                    let n = trunc_i64(&e_m).ok_or_else(|| {
                        TemplateError::misc("Range size is not a representable integer")
                    })?;
                    (n.unsigned_abs() as usize, n >= 0)
                }
            };
            let mut m = bounded_range_model(s_i, count, ascending);
            m.range = Some(std::rc::Rc::new(crate::core::RangeSpec {
                start: s_i,
                count,
                ascending,
                unbounded: false,
                // Java：仅 END_SIZE_LIMITED（`..*`）自适应（Range.java:57-58）
                adaptive: kind == RangeKind::SizeLimited,
                // Java BoundedRangeModel：affectedByStringSlicingBug = inclusiveEnd
                // （仅 `a..b` 闭区间；`..<`/`..!`/`..*` 不受影响，Range.java:56-58）
                affected_by_string_slicing_bug: kind == RangeKind::Inclusive,
            }));
            Ok(m)
        }
        None => {
            // `a..` 右无界（Java Range.java:44-47）：ICI ≥ 2.3.21 →
            // ListableRightUnboundedRangeModel（size=Integer.MAX_VALUE、可索引）；
            // ICI < 2.3.21 → NonListableRightUnboundedRangeModel（旧版兼容：size=0、
            // 迭代为空，`(4..)?size` == 0）
            if kind != RangeKind::SizeLimited {
                return Err(TemplateError::misc("Malformed range expression"));
            }
            let mut m = if env.settings.incompatible_improvements.to_int() >= 2_003_021 {
                listable_right_unbounded_range_model(s_i)
            } else {
                non_listable_right_unbounded_range_model(s_i)
            };
            m.range = Some(std::rc::Rc::new(crate::core::RangeSpec {
                start: s_i,
                count: 0,
                ascending: true,
                unbounded: true,
                adaptive: true, // 无界恒自适应（DynamicKeyName.java:204）
                affected_by_string_slicing_bug: false, // RightUnboundedRangeModel.java:44
            }));
            Ok(m)
        }
    }
}
