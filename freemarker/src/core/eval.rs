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
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::core::environment::lambda_model;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
    use indexmap::IndexMap;
    use std::rc::Rc;
    use std::sync::Arc;

    /// 渲染 `${expr}` 返回输出字符串（表达式单元测试统一入口）
    fn eval_out(root: DynValue, src: &str) -> Result<String> {
        let mut c = Configuration::new();
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

    /// 渲染 `${expr}` 返回布尔（?c 显式输出——Java 默认 boolean_format 插值会报错）
    fn eval_bool(root: DynValue, src: &str) -> bool {
        eval_out(root, &format!("({src})?c"))
            .unwrap()
            .parse()
            .unwrap()
    }

    fn no_root() -> DynValue {
        DynValue::Map(vec![])
    }

    /// Java FTL.jj :2230-2238 BuiltInBannedWhenAutoEscaping：auto-escaping on +
    /// markup 输出格式时 ?html/?xml/?rtf 报错（消息逐字对齐）
    #[test]
    fn legacy_escaping_banned_when_autoescaping_on() {
        // 直接构造 settings（auto_escaping=on + output_format=html）
        use crate::core::{AutoEscaping, OutputFormatKind};
        let mut c = Configuration::new();
        c.settings.auto_escaping = AutoEscaping::On;
        c.settings.output_format = OutputFormatKind::Html;
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", "${s?html}");
        let t = c.get_template("t.ftl").unwrap();
        let mut root = IndexMap::new();
        root.insert("s".to_string(), TModel::from_scalar("<b>".to_string()));
        let mut out = Vec::new();
        let err = t.process(TModel::from_hash(root), &mut out).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Using ?html (legacy escaping) is not allowed when auto-escaping is on with a markup output format (HTML), to avoid double-escaping mistakes."),
            "{msg}"
        );
    }

    /// auto-escaping off 时 ?html 放行（正常转义）
    #[test]
    fn legacy_escaping_allowed_when_autoescaping_off() {
        use crate::core::{AutoEscaping, OutputFormatKind};
        let mut c = Configuration::new();
        c.settings.auto_escaping = AutoEscaping::Off;
        c.settings.output_format = OutputFormatKind::Html;
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", "${s?html}");
        let t = c.get_template("t.ftl").unwrap();
        let mut root = IndexMap::new();
        root.insert("s".to_string(), TModel::from_scalar("<b>".to_string()));
        let mut out = Vec::new();
        t.process(TModel::from_hash(root), &mut out).unwrap();
        assert_eq!(out, b"&lt;b&gt;");
    }

    #[test]
    fn literals() {
        assert_eq!(eval_out(no_root(), "42").unwrap(), "42");
        assert_eq!(eval_out(no_root(), "\"hi\"").unwrap(), "hi");
        assert_eq!(eval_out(no_root(), "true?c").unwrap(), "true");
        assert_eq!(eval_out(no_root(), "1.5").unwrap(), "1.5");
    }

    #[test]
    fn arithmetic_precedence() {
        assert_eq!(eval_out(no_root(), "1 + 2 * 3").unwrap(), "7");
        assert_eq!(eval_out(no_root(), "(1 + 2) * 3").unwrap(), "9");
        // Java 默认 number_format "number" = getNumberInstance → 至多 3 位小数
        assert_eq!(eval_out(no_root(), "10 / 4").unwrap(), "2.5");
        assert_eq!(eval_out(no_root(), "1 / 3").unwrap(), "0.333");
        assert_eq!(eval_out(no_root(), "10 % 3").unwrap(), "1");
        assert_eq!(eval_out(no_root(), "-5 + 2").unwrap(), "-3");
    }

    #[test]
    fn string_concat_and_interp() {
        let root = DynValue::Map(vec![("x".into(), DynValue::Int(3))]);
        assert_eq!(eval_out(root.clone(), "\"a\" + x + \"b\"").unwrap(), "a3b");
        // 布尔拼接（Java：默认 boolean_format "true,false" 是遗留默认 → 报错；
        // 用 ?c 显式指定输出；jar 实测 ${"v=" + true} 默认配置报 legacy 错误）
        assert_eq!(eval_out(root.clone(), "\"v=\" + true?c").unwrap(), "v=true");
        let err = eval_out(root.clone(), "\"v=\" + true").unwrap_err();
        assert!(err.to_string().contains("Can't convert boolean"), "{err}");
        // 插值字符串
        assert_eq!(eval_out(root.clone(), "\"x=${x}\"").unwrap(), "x=3");
    }

    #[test]
    fn comparisons() {
        assert!(eval_bool(no_root(), "1 == 1"));
        assert!(eval_bool(no_root(), "1 == 1.0"));
        assert!(eval_bool(no_root(), "1 < 2"));
        assert!(!eval_bool(no_root(), "2 >= 3"));
        assert!(eval_bool(no_root(), "\"a\" == \"a\""));
        assert!(eval_bool(no_root(), "\"a\" != \"b\""));
        // 字符串与数字跨类型比较 → 报错（EvalUtil.compare :307-326）
        let err = eval_out(no_root(), "1 == \"a\"").unwrap_err();
        assert!(
            err.to_string()
                .contains("Can't compare values of these types"),
            "{err}"
        );
        // 字符串字面量 > 在解析期被拒（Java numberLiteralOnly，FTL.jj :1948-1949，
        // jar 实测 "Found string literal: \"a\". Expecting: number"）；变量形式
        // 才到运行时（EvalUtil.compare :261-267）
        let err = eval_out(no_root(), "\"a\" > \"b\"").unwrap_err();
        assert!(
            err.to_string()
                .contains("Found string literal: \"a\". Expecting: number"),
            "{err}"
        );
        let err = eval_out(
            DynValue::Map(vec![
                ("x".into(), DynValue::Str("a".into())),
                ("y".into(), DynValue::Str("b".into())),
            ]),
            "x > y",
        )
        .unwrap_err();
        assert!(err.to_string().contains("Can't use operator"), "{err}");
    }

    #[test]
    fn boolean_short_circuit() {
        let root = DynValue::Map(vec![("x".into(), DynValue::Int(1))]);
        // 短路：右侧缺失变量不报错
        assert!(!eval_bool(root.clone(), "false && (missing?boolean)"));
        assert!(eval_bool(root.clone(), "true || (missing?boolean)"));
        let err = eval_out(root.clone(), "true && missing").unwrap_err();
        assert!(
            matches!(err, TemplateError::InvalidReference { .. }),
            "{err}"
        );
    }

    #[test]
    fn default_to_is_lazy() {
        let root = DynValue::Map(vec![("x".into(), DynValue::Int(1))]);
        // x!missing → 目标存在，默认表达式不求值（用报错表达式验证惰性）
        assert_eq!(eval_out(root.clone(), "x!missing").unwrap(), "1");
        // 目标缺失 → 求默认
        assert_eq!(eval_out(root.clone(), "nope!\"d\"").unwrap(), "d");
        // 无默认值 → 空字符串（Java EMPTY_STRING_AND_SEQUENCE_AND_HASH）
        assert_eq!(eval_out(root.clone(), "nope!").unwrap(), "");
    }

    #[test]
    fn exists_builtin() {
        let root = DynValue::Map(vec![("x".into(), DynValue::Int(1))]);
        assert!(eval_bool(root.clone(), "x??"));
        assert!(!eval_bool(root.clone(), "nope??"));
        assert!(eval_bool(root.clone(), "x?exists"));
        assert!(!eval_bool(root.clone(), "nope?exists"));
        assert_eq!(
            eval_out(root.clone(), "nope?if_exists?is_nothing?c").unwrap(),
            "true"
        );
        assert_eq!(eval_out(root.clone(), "x?default(\"d\")").unwrap(), "1");
        assert_eq!(eval_out(root.clone(), "nope?default(\"d\")").unwrap(), "d");
    }

    #[test]
    fn dot_and_dyn_key() {
        let root = DynValue::Map(vec![(
            "h".into(),
            DynValue::Map(vec![("k".into(), DynValue::Str("v".into()))]),
        )]);
        assert_eq!(eval_out(root.clone(), "h.k").unwrap(), "v");
        assert_eq!(eval_out(root.clone(), "h[\"k\"]").unwrap(), "v");
        let list_root = DynValue::Map(vec![(
            "l".into(),
            DynValue::List(vec![DynValue::Str("a".into()), DynValue::Str("b".into())]),
        )]);
        assert_eq!(eval_out(list_root.clone(), "l[1]").unwrap(), "b");
        let err = eval_out(root.clone(), "h.missing").unwrap_err();
        assert!(
            matches!(err, TemplateError::InvalidReference { .. }),
            "{err}"
        );
    }

    #[test]
    fn method_call() {
        let m = TModel::from_method(MethodStub);
        let mut root_map = IndexMap::new();
        root_map.insert("f".to_string(), m);
        let root = TModel::from_hash(root_map);
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", "${f(\"x\")}");
        let t = c.get_template("t.ftl").unwrap();
        let mut out = Vec::new();
        t.process(root, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "x");
    }

    #[test]
    fn unknown_builtin_errors() {
        let root = DynValue::Map(vec![("x".into(), DynValue::Int(1))]);
        // 未知内建名在**解析期**被拒（Java BuiltIn.newBuiltIn，BuiltIn.java:349-397：
        // "Unknown built-in: ..." + 字母序内建清单；jar 实测 unknown_builtin 基线）
        let err = eval_out(root.clone(), "x?definitely_not_a_builtin").unwrap_err();
        assert!(
            err.to_string()
                .contains("Unknown built-in: \"definitely_not_a_builtin\""),
            "{err}"
        );
        // 合法内建名 → 求值期正常路径（未实现的内建仍报 ?name 形式）
        let err = eval_out(root.clone(), "x?definitely_not_a_builtin").unwrap_err();
        assert!(
            !err.to_string().contains("?definitely_not_a_builtin"),
            "{err}"
        );
    }

    /// 内建补齐（M5 后缺口 5 个）：is_date_like/web_safe/eval_json/节点 sibling
    #[test]
    fn filled_builtins() {
        let root = DynValue::Map(vec![
            ("s".into(), DynValue::Str("a<b&c".into())),
            ("n".into(), DynValue::Int(5)),
            (
                "d".into(),
                DynValue::Map(vec![("k".into(), DynValue::Int(1))]),
            ),
        ]);
        // web_safe = ?html 弃用别名（BuiltIn.java:312）
        assert_eq!(
            eval_out(root.clone(), "s?web_safe").unwrap(),
            "a&lt;b&amp;c"
        );
        assert_eq!(
            eval_out(root.clone(), "s?web_safe").unwrap(),
            eval_out(root.clone(), "s?html").unwrap()
        );
        // is_date_like：与 is_date 同一 BI（misnomer）；非日期为 false
        assert_eq!(
            eval_out(root.clone(), "n?is_date_like?string('yes','no')").unwrap(),
            "no"
        );
        assert_eq!(
            eval_out(root.clone(), "n?is_date?string('yes','no')").unwrap(),
            eval_out(root.clone(), "n?is_date_like?string('yes','no')").unwrap()
        );
        // eval_json：对象→hash、数组→sequence、嵌套访问
        assert_eq!(
            eval_out(
                root.clone(),
                "'{\"a\": 1, \"b\": [true, null]}'?eval_json.a"
            )
            .unwrap(),
            "1"
        );
        let err = eval_out(root.clone(), "'{bad'?eval_json").unwrap_err();
        assert!(
            err.to_string()
                .contains("Failed to \"?eval_json\" string with this error"),
            "{err}"
        );
        assert!(err.to_string().contains("---begin-message---"), "{err}");
    }

    #[test]
    fn utf16_length_semantics() {
        // 非 BMP 字符（U+10000）在 UTF-16 中占 2 个码元（Java String.length 语义）
        // FTL 字符串字面量只支持 \uXXXX，故经根模型注入该字符
        let root = DynValue::Map(vec![("s".into(), DynValue::Str("a\u{10000}b".into()))]);
        assert_eq!(eval_out(root, "s?length").unwrap(), "4");
        assert_eq!(
            eval_out(DynValue::Map(vec![]), "\"abc\"?length").unwrap(),
            "3"
        );
    }

    #[test]
    fn builtins_basic_set() {
        let root = DynValue::Map(vec![
            ("s".into(), DynValue::Str("Hello World".into())),
            ("n".into(), DynValue::Float(1.5)),
            ("b".into(), DynValue::Bool(true)),
            (
                "seq".into(),
                DynValue::List(vec![DynValue::Str("a".into()), DynValue::Str("b".into())]),
            ),
        ]);
        assert_eq!(
            eval_out(root.clone(), "s?upper_case").unwrap(),
            "HELLO WORLD"
        );
        assert_eq!(
            eval_out(root.clone(), "s?lower_case").unwrap(),
            "hello world"
        );
        assert_eq!(
            eval_out(root.clone(), "s?cap_first").unwrap(),
            "Hello World"
        );
        assert_eq!(eval_out(root.clone(), "s?trim").unwrap(), "Hello World");
        assert_eq!(eval_out(root.clone(), "s?length").unwrap(), "11");
        assert_eq!(eval_out(root.clone(), "n?c").unwrap(), "1.5");
        assert_eq!(eval_out(root.clone(), "b?c").unwrap(), "true");
        assert_eq!(eval_out(root.clone(), "n?int").unwrap(), "1");
        assert_eq!(eval_out(root.clone(), "n?double").unwrap(), "1.5");
        assert_eq!(eval_out(root.clone(), "\"123\"?number").unwrap(), "123");
        assert_eq!(
            eval_out(root.clone(), "(\"true\"?boolean)?c").unwrap(),
            "true"
        );
        assert_eq!(eval_out(root.clone(), "seq?size").unwrap(), "2");
        assert_eq!(eval_out(root.clone(), "seq?first").unwrap(), "a");
        assert_eq!(eval_out(root.clone(), "seq?last").unwrap(), "b");
        assert_eq!(eval_out(root.clone(), "seq?join(\"-\")").unwrap(), "a-b");
        assert_eq!(
            eval_out(root.clone(), "seq?reverse?join(\"\")").unwrap(),
            "ba"
        );
        assert_eq!(
            eval_out(root.clone(), "seq?seq_contains(\"b\")?c").unwrap(),
            "true"
        );
        assert_eq!(
            eval_out(root.clone(), "seq?seq_contains(\"z\")?c").unwrap(),
            "false"
        );
        assert_eq!(
            eval_out(root.clone(), "s?contains(\"World\")?c").unwrap(),
            "true"
        );
        assert_eq!(
            eval_out(root.clone(), "s?starts_with(\"Hello\")?c").unwrap(),
            "true"
        );
        assert_eq!(
            eval_out(root.clone(), "s?ends_with(\"World\")?c").unwrap(),
            "true"
        );
        assert_eq!(
            eval_out(root.clone(), "s?index_of(\"World\")").unwrap(),
            "6"
        );
        assert_eq!(eval_out(root.clone(), "s?substring(6)").unwrap(), "World");
        assert_eq!(
            eval_out(root.clone(), "s?matches(\"Hello.*\")?c").unwrap(),
            "true"
        );
        assert_eq!(
            eval_out(root.clone(), "s?replace(\"World\", \"Rust\")").unwrap(),
            "Hello Rust"
        );
        assert_eq!(
            eval_out(root.clone(), "\"a,b,c\"?split(\",\")?join(\"|\")").unwrap(),
            "a|b|c"
        );
        assert_eq!(eval_out(root.clone(), "s?is_string?c").unwrap(), "true");
        assert_eq!(eval_out(root.clone(), "s?is_number?c").unwrap(), "false");
        assert_eq!(eval_out(root.clone(), "seq?is_sequence?c").unwrap(), "true");
        assert_eq!(
            eval_out(root.clone(), "seq?is_enumerable?c").unwrap(),
            "true"
        );
        assert_eq!(
            eval_out(root.clone(), "missing?is_string?c").unwrap(),
            "false"
        );
    }

    #[test]
    fn ranges() {
        let root = no_root();
        assert_eq!(eval_out(root.clone(), "(1..5)?size").unwrap(), "5");
        assert_eq!(
            eval_out(root.clone(), "(1..5)?join(\",\")").unwrap(),
            "1,2,3,4,5"
        );
        assert_eq!(
            eval_out(root.clone(), "(1..<5)?join(\",\")").unwrap(),
            "1,2,3,4"
        );
        assert_eq!(
            eval_out(root.clone(), "(5..1)?join(\",\")").unwrap(),
            "5,4,3,2,1"
        );
        assert_eq!(
            eval_out(root.clone(), "(10..*3)?join(\",\")").unwrap(),
            "10,11,12"
        );
        // 无界范围（解析器契约：`1..` 为无界）：ICI ≥ 2.3.21 → Listable
        // （Java ListableRightUnboundedRangeModel.size() = Integer.MAX_VALUE）
        let out = eval_out(root.clone(), "(1..)?size").unwrap();
        assert_eq!(out.replace(',', ""), "2147483647");
    }

    #[test]
    fn add_concat_sequence_hash_semantics() {
        // Java AddConcatExpression._eval（AddConcatExpression.java:70-134）：
        // 双序列 → 懒惰拼接；双哈希 → 右值胜出合并；数字+字符串 → 字符串拼接
        let root = DynValue::Map(vec![]);
        let src = "<#assign x = [11]><#assign x += [22]>${x[0]}|${x[1]}|${x?size}";
        assert_eq!(render_template(root.clone(), src).unwrap(), "11|22|2");
        let src = "<#assign x = {'a': 11}><#assign x += {'b': 22}>${x.a}|${x.b}|${x?size}";
        assert_eq!(render_template(root.clone(), src).unwrap(), "11|22|2");
        // 键冲突右值胜出
        let src = "<#assign x = {'a': 1}><#assign x += {'a': 2}>${x.a}";
        assert_eq!(render_template(root.clone(), src).unwrap(), "2");
        // 数字+字符串 → 字符串拼接（'1a' / 'a1'）
        let src = "<#assign x = 1><#assign x += 'a'>${x}";
        assert_eq!(render_template(root.clone(), src).unwrap(), "1a");
        let src = "<#assign x = 'a'><#assign x += 1>${x}";
        assert_eq!(render_template(root.clone(), src).unwrap(), "a1");
    }

    #[test]
    fn assign_compound_operator_errors() {
        // Java NonNumericalException / InvalidReferenceException 消息（jar 实测）
        let root = DynValue::Map(vec![]);
        // 目标非数值
        let err = render_template(root.clone(), "<#assign foo = 'a'><#assign foo -= 1>")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Expected a number, but assignment target variable \"foo\" has evaluated to a string."),
            "{err}"
        );
        // 目标缺失（含操作符名）
        let err = render_template(root.clone(), "<#assign noSuchVar -= 1>")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "The target variable of the assignment, \"noSuchVar\", was null or missing in the template namespace, and the \"-=\" operator must get its value from there before assigning to it."
            ),
            "{err}"
        );
        // 右值非数值 → "For \"#assign\" assignment source: ... ==> 'a'"
        let err = render_template(root.clone(), "<#assign x = 1><#assign x -= 'a'>")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "For \"#assign\" assignment source: Expected a number, but this has evaluated to a string: ==> 'a'"
            ),
            "{err}"
        );
        // $ 前缀目标 → Tip 段（"must not start with \"$\""）
        let err = render_template(root.clone(), "<#assign $noSuchVar += 1>")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "Tip: Variable references must not start with \"$\", unless the \"$\" is really part of the variable name."
            ),
            "{err}"
        );
    }

    /// 整模板渲染（赋值等语句测试）
    fn render_template(root: DynValue, src: &str) -> Result<String> {
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", src);
        let t = c.get_template("t.ftl")?;
        let root_model = SimpleObjectWrapper
            .wrap(&root)?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn substring_java_error_semantics() {
        // Java substringBI（BuiltInsForStringsBasic.java:609-662）：负下标/越界
        // 按序报错（消息逐字对齐 jar 实测，string-builtins1.ftl:27-40 矩阵）
        let root = DynValue::Map(vec![]);
        let err = eval_out(root.clone(), "'ab'?substring(-1)")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("The index must be at least 0, but was -1."),
            "{err}"
        );
        let err = eval_out(root.clone(), "'ab'?substring(3)")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "The index mustn't be greater than the length of the string, 2, but it was 3."
            ),
            "{err}"
        );
        let err = eval_out(root.clone(), "'ab'?substring(1, -1)")
            .unwrap_err()
            .to_string();
        assert!(err.contains("at least 0"), "{err}");
        let err = eval_out(root.clone(), "'ab'?substring(0, 3)")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("greater than the length of the string"),
            "{err}"
        );
        let err = eval_out(root.clone(), "'ab'?substring(1, 0)")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "The begin index argument, 1, shouldn't be greater than the end index argument, 0."
            ),
            "{err}"
        );
        // 正常切片（含 1 参数与 2 参数）
        assert_eq!(eval_out(root.clone(), "'ab'?substring(1)").unwrap(), "b");
        assert_eq!(
            eval_out(root.clone(), "'ab'?substring(0, 2)").unwrap(),
            "ab"
        );
        assert_eq!(eval_out(root.clone(), "'ab'?substring(2)").unwrap(), "");
        // 参数数量（checkMethodArgCount(1, 2)）
        let err = eval_out(root.clone(), "'ab'?substring()")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?substring(...) expects 1 or 2 arguments but has received none."),
            "{err}"
        );
    }

    #[test]
    fn is_macro_and_lambda_slots() {
        let root = DynValue::Map(vec![]);
        assert_eq!(eval_out(root, "true?is_macro?c").unwrap(), "false");
        // lambda 表达式求值为槽位模型（?is_lambda；消费方 ?map/?filter 属 P4/内建智能体）。
        // 解析器仅在 `?builtin(args)` 参数位解析 lambda（grammar.rs），故直接构造验证槽位语义
        let lam = lambda_model(
            vec!["x".to_string()],
            Rc::new(Expr::new(
                ExprKind::Add(
                    Box::new(Expr::new(
                        ExprKind::Ident("x".into()),
                        crate::span::Span::default(),
                    )),
                    Box::new(Expr::new(
                        ExprKind::Num(crate::value::TNumber::Int(1)),
                        crate::span::Span::default(),
                    )),
                ),
                crate::span::Span::default(),
            )),
        );
        assert!(lam.is_lambda());
        assert_eq!(lam.type_name, "lambda");
        assert!(lam
            .internal::<crate::core::environment::LambdaValue>()
            .is_some());
    }

    struct MethodStub;
    impl crate::template::TemplateMethodModelEx for MethodStub {
        fn exec(&self, _env: &mut crate::core::Environment, args: Vec<TModel>) -> Result<TModel> {
            args.first()
                .cloned()
                .ok_or_else(|| TemplateError::misc("no arg"))
        }
    }
}

#[cfg(test)]
mod range_slice_tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
    use std::sync::Arc;

    /// `${expr}` 渲染输出
    fn eval_out(root: DynValue, src: &str) -> Result<String> {
        render_with(root, &format!("${{{src}}}"))
    }

    /// 渲染模板（含变量）返回输出
    fn render_with(root: DynValue, src: &str) -> Result<String> {
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", src);
        let t = c.get_template("t.ftl")?;
        let root_model = SimpleObjectWrapper
            .wrap(&root)?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn root_s() -> DynValue {
        DynValue::Map(vec![
            ("s".to_string(), DynValue::Str("012".to_string())),
            (
                "seq".to_string(),
                DynValue::List(vec![
                    DynValue::Str("a".to_string()),
                    DynValue::Str("b".to_string()),
                    DynValue::Str("c".to_string()),
                ]),
            ),
        ])
    }

    #[test]
    fn eval_builtin_parses_and_evaluates() {
        // ?eval：字符串按表达式解析并求值（Java BuiltInsForStringsMisc.evalBI）
        assert_eq!(eval_out(root_s(), "'[1,2][1]'?eval").unwrap(), "2");
        assert_eq!(eval_out(root_s(), "'1+2*3'?eval").unwrap(), "7");
        // 求值错误包进 "Failed to ?eval" 消息
        let err = eval_out(root_s(), "'s[5..]'?eval").unwrap_err().to_string();
        assert!(err.contains("Failed to \"?eval\""), "{err}");
    }

    #[test]
    fn range_slice_string_semantics() {
        // Java dealWithRangeKey（DynamicKeyName.java:183-334）：
        // 自适应裁剪（`..*` 越界索引被裁剪而非报错）
        assert_eq!(
            render_with(root_s(), "${s[0..*-2]}").unwrap(),
            "0",
            "0..*-2 → [0,-1] 裁剪为 [0]"
        );
        assert_eq!(render_with(root_s(), "${s[2..*2]}").unwrap(), "2");
        assert_eq!(render_with(root_s(), "${s[2..]}").unwrap(), "2");
        assert_eq!(render_with(root_s(), "${s[3..]}").unwrap(), "");
        // 降序字符串切片 → 报错（resultSize>1）
        let err = render_with(root_s(), "${s[1..*-2]}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Decreasing ranges aren't allowed for slicing strings"),
            "{err}"
        );
        // 起始越界（非自适应）→ 报错
        let err = render_with(root_s(), "${s[4..]}").unwrap_err().to_string();
        assert!(err.contains("is out of bounds"), "{err}");
        // 负起始 → 报错
        let err = render_with(root_s(), "${s[-1..]}").unwrap_err().to_string();
        assert!(err.contains("Negative range start index"), "{err}");
    }

    #[test]
    fn range_slice_sequence_semantics() {
        // 序列切片：按范围下标取元素（降序允许，与字符串不同）
        assert_eq!(
            render_with(root_s(), "${seq[1..*-2]?join('')}").unwrap(),
            "ba",
            "seq[1..*-2] → [1,0]"
        );
        assert_eq!(
            render_with(root_s(), "${seq[0..*-2]?join('')}").unwrap(),
            "a",
            "seq[0..*-2] → [0,-1] 裁剪为 [0]"
        );
        assert_eq!(
            render_with(root_s(), "${seq[2..]?join('')}").unwrap(),
            "c",
            "seq[2..] → 无界"
        );
        assert_eq!(
            render_with(root_s(), "${seq[3..]?join('')}").unwrap(),
            "",
            "seq[3..] → 空（自适应递增起始可 == 长度）"
        );
        // 起始超出长度 → 报错（Java :224-236：firstIdx > targetSize）
        let err = render_with(root_s(), "${seq[4..]?join('')}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("is out of bounds"), "{err}");
    }

    #[test]
    fn interpret_transform_flow() {
        // ?interpret 变换模型：<#transform> 先输出解释模板，body 直通
        // （Interpret.TemplateProcessorModel.getWriter 返回透传 writer）
        let src = "<#global x=['a','b','c']><#transform r'<#foreach y in x>${y}</#foreach>'?interpret>def</#transform>";
        assert_eq!(render_with(root_s(), src).unwrap(), "abcdef");
        // 调用产物后解释模板内宏可见（env.include 注册进命名空间）
        let src2 = "<#assign t = r'<#macro m>M</#macro>'?interpret><@t /><@m/>";
        assert_eq!(render_with(root_s(), src2).unwrap(), "M");
    }

    #[test]
    fn string_slicing_legacy_bug_emulation() {
        // Java DynamicKeyName.java:322-330：`a..b` 闭区间降序范围结果长为 2 →
        // 模拟旧版 bug 返回 ""（"foo"[n .. n-1] 给 "" 而非报错，FTL 2.4 修复前）；
        // 结果长 > 2 → 报错；`..<`/`..!`/`..*` 运算符不受影响（template 注释
        // "But it isn't emulated for operators introduced after 2.3.20"）
        // 测试根 s = "012"（3 字符，最大合法起始下标 2）
        assert_eq!(
            render_with(root_s(), "${s[1..0]}").unwrap(),
            "",
            "s[1..0] → 旧版 bug 模拟为空串"
        );
        assert_eq!(render_with(root_s(), "${s[2..1]}").unwrap(), "");
        // 结果长 3 的闭区间降序 → 报错（resultSize != 2 不模拟）
        let err = render_with(root_s(), "${s[2..0]}").unwrap_err().to_string();
        assert!(
            err.contains("Decreasing ranges aren't allowed for slicing strings"),
            "{err}"
        );
        // `..<`（排端）与 `..*`（定长）不受旧版 bug 影响 → 即使结果长 2 也报错
        let err = render_with(root_s(), "${s[2..<0]}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Decreasing ranges aren't allowed for slicing strings"),
            "{err}"
        );
        let err = render_with(root_s(), "${s[2..*-2]}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Decreasing ranges aren't allowed for slicing strings"),
            "{err}"
        );
        // resultSize==1 的降序字符串切片合法（`0..*-1` → "0"）
        assert_eq!(
            render_with(root_s(), "${s[0..*-1]}").unwrap(),
            "0",
            "0..*-1 → [0]"
        );
    }

    #[test]
    fn right_unbounded_range_ici_models() {
        // Java Range.java:44-47：ICI ≥ 2.3.21 → ListableRightUnboundedRangeModel
        // （size=Integer.MAX_VALUE、r[i]=begin+i）；< 2.3.21 → NonListable（size=0、空）
        let root = SimpleObjectWrapper
            .wrap(&DynValue::Map(vec![]))
            .unwrap()
            .unwrap_or_else(TModel::nothing);
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        c.settings.incompatible_improvements = crate::template::Version::parse("2.3.34").unwrap();
        loader.put("t.ftl", "<#assign r = 4..>${r?size}|${r[0]}|${r[1000000]}");
        let t = c.get_template("t.ftl").unwrap();
        let mut out = Vec::new();
        t.process(root.clone(), &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap().replace(',', ""),
            "2147483647|4|1000004",
            "ICI≥2.3.21：size=Integer.MAX_VALUE、get(i)=begin+i"
        );
        // 负下标 → RangeModel.get 的 "Range item index ... out of bounds."（Java RangeModel.java:29-31）
        loader.put("t2.ftl", "<#assign r = 4..>${r[-1]}");
        let t = c.get_template("t2.ftl").unwrap();
        let mut out = Vec::new();
        let err = t.process(root.clone(), &mut out).unwrap_err();
        assert!(
            err.to_string()
                .contains("Range item index -1 is out of bounds."),
            "{err}"
        );
        // 无界范围迭代：break 终止（惰性拉取，不物化 2^31-1 项）
        loader.put(
            "t3.ftl",
            "<#assign r = 4..><#list r as i><#if i == 6><#break></#if>${i},</#list>",
        );
        let t = c.get_template("t3.ftl").unwrap();
        let mut out = Vec::new();
        t.process(root.clone(), &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "4,5,");
        // ICI < 2.3.21 → NonListable：size=0、迭代为空、[0] 报 invalid reference
        c.settings.incompatible_improvements = crate::template::Version::parse("2.3.20").unwrap();
        loader.put(
            "t4.ftl",
            "<#assign r = 4..>${r?size}<#list r as i>${i}</#list>",
        );
        let t = c.get_template("t4.ftl").unwrap();
        let mut out = Vec::new();
        t.process(root.clone(), &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "0",
            "ICI<2.3.21：size=0、迭代为空"
        );
        loader.put("t5.ftl", "<#assign r = 4..>${r[0]}");
        let t = c.get_template("t5.ftl").unwrap();
        let mut out = Vec::new();
        let err = t.process(root, &mut out).unwrap_err();
        assert!(
            err.to_string().contains("has evaluated to null or missing"),
            "{err}"
        );
    }
}

#[cfg(test)]
/// ?join 内建（Java joinBI）测试：集合支持/whenEmpty/afterLast/null 跳过/
/// 右无界守卫/失败索引包装/参数数量（Probe23 双 ICI 实测矩阵固化）
mod join_builtin_tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
    use std::sync::Arc;

    fn render_with(root: DynValue, src: &str) -> Result<String> {
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", src);
        let t = c.get_template("t.ftl")?;
        let root_model = SimpleObjectWrapper
            .wrap(&root)?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn root() -> DynValue {
        DynValue::Map(vec![
            // 含 null 项序列（Java SimpleSequence 支持 null；?join 跳过）
            (
                "withNull".to_string(),
                DynValue::List(vec![
                    DynValue::Str("a".to_string()),
                    DynValue::Null,
                    DynValue::Str("b".to_string()),
                ]),
            ),
            ("h".to_string(), DynValue::Map(vec![])),
            (
                "seq".to_string(),
                DynValue::List(vec![
                    DynValue::Str("x".to_string()),
                    DynValue::Str("y".to_string()),
                    DynValue::Str("z".to_string()),
                ]),
            ),
            ("empty".to_string(), DynValue::List(vec![])),
        ])
    }

    #[test]
    fn join_null_skip_and_3args() {
        // null 项跳过（idx 仍递增）；afterLast 在 hadItem 时追加；whenEmpty 在空时使用
        assert_eq!(
            render_with(root(), "${withNull?join('-')}").unwrap(),
            "a-b",
            "null 项跳过且分隔符逻辑不变"
        );
        assert_eq!(
            render_with(root(), "${withNull?join('-', 'EMPTY', 'LAST')}").unwrap(),
            "a-bLAST",
            "hadItem → afterLast"
        );
        assert_eq!(
            render_with(root(), "${empty?join('-', 'EMPTY', 'LAST')}").unwrap(),
            "EMPTY",
            "空列表 → whenEmpty"
        );
        // 无 whenEmpty/afterLast 时保持原样
        assert_eq!(render_with(root(), "${empty?join('-')}").unwrap(), "");
    }

    #[test]
    fn join_lazy_collection_and_unbounded_guard() {
        // 惰性集合（?map → LazilyGeneratedCollectionModel 等价物）可直接 ?join
        assert_eq!(
            render_with(root(), "${(seq?map(x -> x))?join(',')}").unwrap(),
            "x,y,z",
            "TemplateCollectionModel 优先惰性迭代"
        );
        // 右无界数值范围守卫（BuiltInsForSequences.java:929-935）
        let err = render_with(root(), "${(1..)?join(',')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "The input sequence is a right-unbounded numerical range, thus, it's infinitely long, and can't processed with this built-in."
            ),
            "{err}"
        );
    }

    #[test]
    fn join_error_wrapping_and_arg_count() {
        // 逐项转换错误包装失败索引（Java :230-238）
        let err = render_with(root(), "${[h]?join(',')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("\"?join\" failed at index 0 with this error:"),
            "{err}"
        );
        // 参数数量 1-3（_MessageUtil.newArgCntError）
        let err = render_with(root(), "${seq?join()}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?join(...) expects 1 to 3 arguments but has received none."),
            "{err}"
        );
        let err = render_with(root(), "${seq?join(',', 'a', 'b', 'c')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?join(...) expects 1 to 3 arguments but has received 4."),
            "{err}"
        );
    }

    /// var-layers 全链路 —— 对应 Java templatesuite 的 var-layers 用例：
    /// `.globals`（全局命名空间→数据模型→共享变量）与 `.data_model`（数据模型→
    /// 共享变量）复合哈希、`.main`/`.namespace`/`.locals` 分层、`<@.main.foo>` 跨
    /// 命名空间宏调用；共享变量 y = 7 对应 Java TemplateTestCase.java:353
    /// conf.setSharedVariable("y", 7)（黄金套件数据模型由主会话补齐）。
    #[test]
    fn var_layers_full_flow() {
        let mut c = Configuration::new();
        c.set_shared_variable("y", TModel::from_number(TNumber::from_i64(7)));
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put(
            "var-layers.ftl",
            "<#import \"varlayers_lib.ftl\" as lib>\n<@foo 1/>\n${x} = ${.data_model.x} = ${.globals.x}\n<#assign x = 5>\n${x} = ${.main.x} = ${.namespace.x}\n<#global x = 6>\n${.globals.x} but ${.data_model.x} = 4\n${y} = ${.globals.y} = ${.data_model.y?default(\"ERROR\")}\n<#macro foo x>\n  ${x} = ${.locals.x}\n  <#local x = 2>\n  ${x} = ${.locals.x}\n  <#local y = 3>\n  ${y} = ${.locals.y}\n</#macro>\n--\n<@lib.foo/>\n--\n",
        );
        loader.put(
            "varlayers_lib.ftl",
            "<#assign x1 = .data_model.x>\n<#assign x2 = x>\n<#assign z2 = z>\n<#macro foo>\n<@.main.foo 1/>\n  ${z} = ${z2} = ${x1} = ${.data_model.x}\n  5\n  ${x} == ${.globals.x}\n  ${y} == ${.globals.y} == ${.data_model.y?default(\"ERROR\")}\n</#macro>\n",
        );
        let root = DynValue::Map(vec![
            ("x".into(), DynValue::Int(4)),
            ("z".into(), DynValue::Int(4)),
        ]);
        let t = c.get_template("var-layers.ftl").unwrap();
        let root_model = SimpleObjectWrapper
            .wrap(&root)
            .unwrap()
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out).unwrap();
        // 逐字节对照 Java expected/var-layers.txt（版权头之后）
        assert_eq!(
            String::from_utf8(out).unwrap(),
            concat!(
                "  1 = 1\n",
                "  2 = 2\n",
                "  3 = 3\n",
                "4 = 4 = 4\n",
                "5 = 5 = 5\n",
                "6 but 4 = 4\n",
                "7 = 7 = 7\n",
                "--\n",
                "  1 = 1\n",
                "  2 = 2\n",
                "  3 = 3\n",
                "  4 = 4 = 4 = 4\n",
                "  5\n",
                "  6 == 6\n",
                "  7 == 7 == 7\n",
                "--\n",
            )
        );
    }
}
