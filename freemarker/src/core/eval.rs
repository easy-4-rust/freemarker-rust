//! 表达式求值 —— 对应 Java `freemarker.core.Expression` 家族各子类的 `eval(Environment)` 方法
//! （集中式入口，docs/04 §5）。各 `ExprKind` variant → Java 类映射（expression.rs 文件头另有总表）：
//! - Ident → Identifier.java:37 `_eval`；BuiltinVar → BuiltinVariable.java:186 `_eval`
//! - Str/InterpStr → StringLiteral.java:88 / AddConcatExpression 拼接
//! - Add → AddConcatExpression.java:63-134；Sub/Mul/Div/Mod → ArithmeticExpression.java:48-94
//! - Eq/NotEq/Gt/Gte/Lt/Lte → ComparisonExpression.java:92 `evalToBoolean` → EvalUtil.compare :183-317
//! - And/Or/Not → AndExpression/OrExpression/NotExpression `evalToBoolean`（短路）
//! - Range → Range.java:52 `_eval`；Default → DefaultToExpression.java:84（惰性）
//! - Exists → ExistsExpression.java:37；Dot → Dot.java:49；DynKey → DynamicKeyName.java:69
//! - Call → MethodCall.java:54（方法角色）+ invokeFunction（宏角色）
//! - BuiltIn → BuiltIn 家族（BuiltInsFor*.java；docs/05）；ListLit → ListLiteral；
//!   HashLit → HashLiteral；Lambda → LocalLambdaExpression；Paren → ParentheticalExpression

use crate::core::expression::{
    eval_interp_str, ArithmeticExpression, BooleanLiteral, BuiltinVariable, ComparisonExpression,
    DefaultToExpression, Dot, DynamicKeyName, ExistsExpression, HashLiteral, Identifier,
    ListLiteral, LocalLambdaExpression, MethodCall, NotExpression, NumOp, NumberLiteral,
    ParentheticalExpression, Range, StringLiteral, UnaryPlusMinusExpression,
};
use crate::core::{Expr, ExprKind};
use crate::error::{Result, TemplateError};
use crate::span::Span;
use crate::template::TModel;
use crate::value::TNumber;
use bigdecimal::ToPrimitive;

/// 表达式求值 —— 对应 Java `Expression.eval(Environment)`（docs/04 §5）。
/// 求值失败一律 Err（Java 抛 TemplateException 族）；缺失变量为
/// `TemplateError::InvalidReference`（`??`/`!`/`?default`/`?exists` 等显式抑制）。
/// 错误位置：失败表达式（Java blamed Expression）的起始位置 + 当前模板名
/// （Java `InvalidReferenceException.getInstance(blamed, env)` 的 blamed.getStartLocation）；
/// 内层表达式失败时位置已带（如 `user.name` 中 `user` 的列），外层不再覆盖。
pub fn eval(env: &mut crate::core::Environment, expr: &Expr) -> Result<TModel> {
    let r = eval_inner(env, expr);
    match r {
        Err(e) => Err(attach_eval_location(e, env, expr.span)),
        ok => ok,
    }
}

/// eval 包装的位置附加（仅未带位置的错误；Java 异常构造时即取 blamed 位置）
fn attach_eval_location(
    e: TemplateError,
    env: &crate::core::Environment,
    span: Span,
) -> TemplateError {
    if e.has_location() {
        return e;
    }
    e.with_location(&env.current_template_name, span)
}

fn eval_inner(env: &mut crate::core::Environment, expr: &Expr) -> Result<TModel> {
    match &expr.kind {
        ExprKind::Str(s) => StringLiteral::new(s.clone()).eval(env),
        ExprKind::InterpStr(parts) => eval_interp_str(env, parts),
        ExprKind::Num(n) => NumberLiteral::new(n.clone()).eval(env),
        ExprKind::Bool(b) => BooleanLiteral::new(*b).eval(env),
        ExprKind::Ident(name) => Identifier::new(name.clone()).eval(env),
        ExprKind::BuiltinVar(v) => BuiltinVariable::new(*v).eval(env),
        ExprKind::Dot { target, name } => Dot::new((**target).clone(), name.clone()).eval(env),
        ExprKind::DynKey { target, key } => {
            DynamicKeyName::new((**target).clone(), (**key).clone()).eval(env)
        }
        ExprKind::Call { callee, args } => {
            MethodCall::new((**callee).clone(), args.clone()).eval(env)
        }
        ExprKind::UnaryMinus(t) => UnaryPlusMinusExpression::new((**t).clone()).eval(env),
        ExprKind::Not(t) => NotExpression::new((**t).clone()).eval(env),
        ExprKind::Add(a, b) => {
            crate::core::expression::AddConcatExpression::new((**a).clone(), (**b).clone())
                .eval(env)
        }
        ExprKind::Sub(a, b) => {
            ArithmeticExpression::new((**a).clone(), (**b).clone(), NumOp::Sub).eval(env)
        }
        ExprKind::Mul(a, b) => {
            ArithmeticExpression::new((**a).clone(), (**b).clone(), NumOp::Mul).eval(env)
        }
        ExprKind::Div(a, b) => {
            ArithmeticExpression::new((**a).clone(), (**b).clone(), NumOp::Div).eval(env)
        }
        ExprKind::Mod(a, b) => {
            ArithmeticExpression::new((**a).clone(), (**b).clone(), NumOp::Mod).eval(env)
        }
        ExprKind::Eq(a, b) => {
            ComparisonExpression::new((**a).clone(), (**b).clone(), CmpOp::Eq).eval(env)
        }
        ExprKind::NotEq(a, b) => {
            ComparisonExpression::new((**a).clone(), (**b).clone(), CmpOp::NotEq).eval(env)
        }
        ExprKind::Gt(a, b) => {
            ComparisonExpression::new((**a).clone(), (**b).clone(), CmpOp::Gt).eval(env)
        }
        ExprKind::Gte(a, b) => {
            ComparisonExpression::new((**a).clone(), (**b).clone(), CmpOp::Gte).eval(env)
        }
        ExprKind::Lt(a, b) => {
            ComparisonExpression::new((**a).clone(), (**b).clone(), CmpOp::Lt).eval(env)
        }
        ExprKind::Lte(a, b) => {
            ComparisonExpression::new((**a).clone(), (**b).clone(), CmpOp::Lte).eval(env)
        }
        // Java AndExpression / OrExpression（expression/and_expression.rs、
        // expression/or_expression.rs：短路语义）
        ExprKind::And(a, b) => {
            crate::core::expression::AndExpression::new((**a).clone(), (**b).clone()).eval(env)
        }
        ExprKind::Or(a, b) => {
            crate::core::expression::OrExpression::new((**a).clone(), (**b).clone()).eval(env)
        }
        ExprKind::Range { start, end, kind } => Range::new(
            (**start).clone(),
            end.as_ref().map(|e| (**e).clone()),
            *kind,
        )
        .eval(env),
        ExprKind::Default { target, default } => {
            DefaultToExpression::new((**target).clone(), default.as_ref().map(|d| (**d).clone()))
                .eval(env)
        }
        ExprKind::Exists(t) => ExistsExpression::new((**t).clone()).eval(env),
        ExprKind::BuiltIn { target, name, args } => {
            crate::core::expression::eval_builtin(env, target, name, args)
        }
        ExprKind::ListLit(items) => ListLiteral::new(items.clone()).eval(env),
        ExprKind::HashLit(pairs) => HashLiteral::new(pairs.clone()).eval(env),
        ExprKind::Lambda { params, body } => {
            LocalLambdaExpression::new(params.clone(), (**body).clone()).eval(env)
        }
        ExprKind::Paren(inner) => ParentheticalExpression::new((**inner).clone()).eval(env),
    }
}

/// 布尔强制 —— 对应 Java `Expression.modelToBoolean`（Expression.java:186-193）：
/// 布尔模型直读；classic 兼容模式 → `model != null && !isEmpty(model)`（缺失 → false、
/// 空串/空序列 → false，其余非布尔 → true）；strict 模式 → NonBooleanException。
pub(crate) fn model_to_boolean(env: &crate::core::Environment, m: &TModel) -> Result<bool> {
    if let Some(b) = &m.boolean {
        return b.as_boolean();
    }
    if !env.settings.classic_compatible {
        return Err(TemplateError::type_mismatch("boolean", m.type_name));
    }
    // Java MiscUtil.isEmpty（classic 分支）：null → false；标量/序列/集合/哈希 → 空判定
    if m.is_nothing() {
        return Ok(false);
    }
    if let Some(s) = &m.scalar {
        return Ok(!s.as_string()?.is_empty());
    }
    if let Some(seq) = &m.sequence {
        return Ok(seq.size()? != 0);
    }
    if let Some(col) = &m.collection {
        // Java MiscUtil.isEmpty：collection → iterator().hasNext()
        return Ok(col.iterator()?.next().is_some());
    }
    if let Some(ex) = &m.hash_ex {
        return Ok(ex.size()? != 0);
    }
    Ok(true)
}

pub(crate) use crate::core::expression::check_legacy_escaping_ban;
/// 插值字符串（Java StringLiteral 的插值片段拼接；各片段按输出字符串规则转换）
pub(crate) use crate::core::expression::compare_numbers;
pub use crate::core::expression::{compare_models, CmpOp};

/// 默认值（Java DefaultToExpression.java:84-105：目标缺失 → 求默认值（惰性）；
/// 无默认值 → 空字符串模型 EMPTY_STRING_AND_SEQUENCE_AND_HASH（v1 简化为空字符串））
/// 存在性运算符的目标求值 —— Java 语义（ExistsExpression.java:42-50 /
/// DefaultToExpression.java:84-90 / BuiltInsForExistenceHandling.evalMaybeNonexistentTarget）：
/// **仅括号目标**（ParentheticalExpression）捕获 InvalidReferenceException；非括号目标
/// （Dot/DynKey 等在 target null 时抛 IRE）错误直接上传；标识符等"eval 返回 null 不抛"
/// 的表达式在本引擎解析层抛 Err（get_variable）→ 此处等价捕获（Java：v!'-' → null →
/// 默认值/存在性判定）。
pub(crate) fn eval_lenient(env: &mut crate::core::Environment, target: &Expr) -> Result<TModel> {
    let catches = matches!(&target.kind, ExprKind::Paren(_) | ExprKind::Ident(_));
    match eval(env, target) {
        Ok(m) => Ok(m),
        Err(TemplateError::InvalidReference { .. }) if catches => Ok(TModel::nothing()),
        Err(e) => Err(e),
    }
}

/// 数值截断（Java `Number.intValue()/longValue()` 向零截断语义）
pub(crate) fn trunc_i64(n: &TNumber) -> Option<i64> {
    match n {
        TNumber::Int(v) => Some(*v as i64),
        TNumber::Long(v) => Some(*v),
        TNumber::BigInt(v) => i64::try_from(v.clone()).ok(),
        TNumber::Decimal(d) => i64::try_from(d.with_scale(0).as_bigint_and_scale().0.as_ref()).ok(),
        TNumber::Float(v) => Some(*v as i64),
        TNumber::Double(v) => Some(*v as i64),
    }
}

/// 精确整数转换（Java `NumberUtil.toIntExact`：非整数值 → None；
/// abcBI 等要求无损整数值的内建使用；1.00001 → None，1.0 → Some(1)）
pub(crate) fn to_int_exact(n: &TNumber) -> Option<i64> {
    match n {
        TNumber::Int(v) => Some(*v as i64),
        TNumber::Long(v) => Some(*v),
        TNumber::BigInt(v) => i64::try_from(v.clone()).ok(),
        TNumber::Decimal(d) => {
            if d.is_integer() {
                d.to_i64()
            } else {
                None
            }
        }
        TNumber::Float(v) => {
            if v.is_finite() && v.fract() == 0.0 {
                Some(*v as i64)
            } else {
                None
            }
        }
        TNumber::Double(v) => {
            if v.is_finite() && v.fract() == 0.0 {
                Some(*v as i64)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "eval_tests.rs"]
mod eval_tests;
