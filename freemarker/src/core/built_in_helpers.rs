//! 内建函数 —— 对应 Java `freemarker.core.BuiltIn`（`eval`/`calculateResult`，
//! BuiltIn.java:144 注册表）及各 BuiltInsFor*.java 内建实现的聚合入口。
//! 各辅助函数对应 Java 类：loop_state_builtin/eval_item_cycle_direct →
//! BuiltInsForLoopVariables.java；builtin_interpret → Interpret.java；
//! date_type_if_unknown/is_type_test/str_builtin → BuiltInsForMultipleTypes.java；
//! parse_number → BuiltInsForNumbers.java；json_value_to_model →
//! BuiltInsForStringsMisc.java；locale_case → BuiltInsForStrings.java；
//! join 相关 → BuiltInsForSequenceBuiltin.java（BuiltInsFor*.java 的细化拆分
//! 见 builtins 对齐任务）。

use crate::core::environment::{expr_desc, model_to_string};
use crate::core::eval::eval;
use crate::core::{Expr, ExprKind};
use crate::error::{Result, TemplateError};
use crate::template::utility::java_trim;
use crate::template::TModel;
use crate::value::{DateType, TNumber};

// ---------------------------------------------------------------------------
// 内建函数（Java BuiltInsFor*.java；docs/05 迁移清单）
// ---------------------------------------------------------------------------

pub(crate) fn eval_item_cycle_direct(
    env: &mut crate::core::Environment,
    target: &Expr,
    args: &[Expr],
) -> Result<TModel> {
    if args.is_empty() {
        return Err(TemplateError::misc(
            "?itemCycle(...) expects 1 or more (unlimited) arguments but has received none.",
        ));
    }
    let target_var = match &target.kind {
        ExprKind::Ident(n) => Some(n.as_str()),
        _ => None,
    };
    let lc = env.get_loop_context(target_var).ok_or_else(|| {
        TemplateError::misc(
            "The target of ?itemCycle is not a loop variable (no enclosing loop in scope)",
        )
    })?;
    let idx = lc.borrow().index % args.len();
    let m = eval(env, &args[idx])?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(expr_desc(&args[idx])));
    }
    Ok(m)
}

/// `?new` 的构造器方法模型 —— 对应 Java `NewBI.ConstructorFunction`
/// （NewBI.java:32-77）：调用时经类解析实例化。v1 仅支持三个 utility 变换类
/// （utility_transforms::new_utility_class），其余类名按 Java ClassNotFoundException
/// 语义报错。
pub(crate) struct NewConstructorFunction {
    pub(crate) class_name: String,
}

impl crate::template::TemplateMethodModelEx for NewConstructorFunction {
    fn exec(&self, _env: &mut crate::core::Environment, args: Vec<TModel>) -> Result<TModel> {
        crate::template::utility_transforms::new_utility_class(&self.class_name, &args)
    }
}

/// 内建实现：返回 None 表示未知内建（由调用方报 "Unknown built-in: ?xxx"）
pub(crate) fn loop_state_builtin(
    env: &mut crate::core::Environment,
    target: &Expr,
    name: &str,
    args: &super::BuiltinArgs,
) -> Result<Option<TModel>> {
    if !matches!(
        name,
        "index"
            | "counter"
            | "has_next"
            | "has_previous"
            | "is_first"
            | "is_last"
            | "is_odd"
            | "is_even"
            | "is_odd_item"
            | "is_even_item"
            | "item_parity"
            | "item_parity_cap"
            | "item_cycle"
    ) {
        return Ok(None);
    }
    let target_var = match &target.kind {
        ExprKind::Ident(n) => Some(n.as_str()),
        _ => None,
    };
    let lc = env.get_loop_context(target_var).ok_or_else(|| {
        TemplateError::misc(format!(
            "The target of ?{name} is not a loop variable (no enclosing loop in scope)"
        ))
    })?;
    let lc = lc.borrow();
    let b = match name {
        "index" => {
            return Ok(Some(TModel::from_number(TNumber::from_i64(
                lc.index as i64,
            ))))
        }
        "counter" => {
            return Ok(Some(TModel::from_number(TNumber::from_i64(
                lc.index as i64 + 1,
            ))))
        }
        "has_next" => lc.has_next,
        "has_previous" => lc.index > 0,
        "is_first" => lc.index == 0,
        "is_last" => !lc.has_next,
        "is_odd" => lc.index % 2 == 1,
        "is_even" => lc.index % 2 == 0,
        "is_odd_item" => (lc.index + 1) % 2 == 1,
        "is_even_item" => (lc.index + 1) % 2 == 0,
        "item_parity" | "item_parity_cap" => {
            // Java BuiltInsForLoopVariables.itemParityBI：1 起始奇偶
            let odd = (lc.index + 1) % 2 == 1;
            let s = match name {
                "item_parity" => {
                    if odd {
                        "odd"
                    } else {
                        "even"
                    }
                }
                _ => {
                    if odd {
                        "Odd"
                    } else {
                        "Even"
                    }
                }
            };
            return Ok(Some(TModel::from_scalar(s.to_string())));
        }
        "item_cycle" => {
            // Java itemCycle(values...)：按 index 循环取值（Java 实测 0 起始）
            let args = args.exprs.unwrap_or(&[]);
            if args.is_empty() {
                return Err(TemplateError::misc(
                    "The ?itemCycle built-in requires at least one argument.",
                ));
            }
            let idx = lc.index % args.len();
            let m = eval(env, &args[idx])?;
            if m.is_nothing() {
                return Err(TemplateError::invalid_reference(expr_desc(&args[idx])));
            }
            return Ok(Some(m));
        }
        _ => unreachable!(),
    };
    Ok(Some(TModel::from_boolean(b)))
}

/// `?interpret` —— 对应 Java `Interpret`（Interpret.java，BuiltIn.java:144 putBI）：
/// 求值目标（字符串或 [源码, id] 序列）→ 动态解析为模板 → 返回变换模型
/// （Java 返回 TemplateTransformModel；`<#transform x?interpret>` 与 `<@x/>` 均可用）。
pub(crate) fn builtin_interpret(
    env: &mut crate::core::Environment,
    target: &Expr,
) -> Result<TModel> {
    let m = eval(env, target)?;
    let (source, _id) = if let Some(seq) = &m.sequence {
        // Java Interpret.calculateResult：序列 [0]=源码、[1]（可选）=模板名后缀
        let s0 = seq
            .get(0)
            .map_err(|_| TemplateError::misc("?interpret: the sequence is empty"))?;
        let text = crate::core::environment::model_to_string(env, &s0)?;
        (text, String::new())
    } else if let Some(sc) = &m.scalar {
        (sc.as_string()?, String::new())
    } else {
        return Err(TemplateError::type_mismatch(
            "sequence or string",
            m.type_name,
        ));
    };
    if source.is_empty() {
        return Err(TemplateError::misc(
            "?interpret: the template source is empty",
        ));
    }
    let cfg = env.template.configuration.clone();
    let name = format!("{}->anonymous_interpreted", env.current_template_name);
    let template = crate::parser::parse(&cfg, &name, &source).map_err(|e| {
        TemplateError::misc(format!(
            "Template parsing with \"?interpret\" has failed with this error:\n\n{e}"
        ))
    })?;
    Ok(TModel::from_transform(InterpretedTemplate(template)))
}

/// ?interpret 的变换模型 —— 对应 Java `Interpret.TemplateProcessorModel`
/// （Interpret.java:120-150：getWriter → env.include(template)）
struct InterpretedTemplate(crate::template::Template);

impl crate::template::TemplateTransformModel for InterpretedTemplate {
    fn transform(&self, env: &mut crate::core::Environment) -> Result<()> {
        // Java Interpret.TemplateProcessorModel.getWriter（Interpret.java:123-138）：
        // env.include(template) —— 宏注册进当前命名空间 + 执行（interpret.ftl 的
        // `<@t /><@m/>` 依赖此语义 —— 解释模板内定义的宏调用后可见）；
        // 返回透传 writer，调用方随后直通 body（TransformBlock/visitAndTransform）
        env.include_template(&self.0)
    }
}

/// 类型测试内建：目标缺失 → false；其他求值错误上传（Java is_*BI 语义）
/// 日期类型转换（Java dateType_if_unknownBI：未知 → 指定类型；已知 → 原样）
pub(crate) fn date_type_if_unknown(
    env: &mut crate::core::Environment,
    target: &Expr,
    kind: DateType,
) -> Result<Option<TModel>> {
    let m = eval(env, target)?;
    if let Some(d) = &m.date {
        let dv = d.as_date()?;
        if dv.kind != DateType::Unknown {
            return Ok(Some(m));
        }
        let mut nv = dv.clone();
        nv.kind = kind;
        return Ok(Some(TModel::from_date(nv)));
    }
    Err(TemplateError::type_mismatch("date", m.type_name))
}

pub(crate) fn is_type_test(
    env: &mut crate::core::Environment,
    target: &Expr,
    test: impl Fn(&TModel) -> bool,
) -> Result<Option<TModel>> {
    match eval(env, target) {
        Ok(m) => Ok(Some(TModel::from_boolean(test(&m)))),
        Err(TemplateError::InvalidReference { .. }) => Ok(Some(TModel::from_boolean(false))),
        Err(e) => Err(e),
    }
}

/// locale 感知的大小写转换 —— 对应 Java `String.toUpperCase(Locale)` /
/// `toLowerCase(Locale)` 的 ConditionalSpecialCasing 特殊规则：tr/az locale 下
/// `i` → `İ`（U+0130）、`I` → `ı`（U+0131）；`i`/`I` 后跟组合点（U+0307）时
/// 大写去点（i 保持）、小写保点。其余 locale 按 Unicode 默认规则。
pub(crate) fn locale_case(s: &str, locale: &str, upper: bool) -> String {
    let lang = locale.split(['_', '-']).next().unwrap_or("");
    let tr = lang == "tr" || lang == "az";
    if !tr {
        return if upper {
            s.to_uppercase()
        } else {
            s.to_lowercase()
        };
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if upper {
            match c {
                'i' if next == Some('\u{0307}') => {
                    // i + combining dot → 小写 i 保持、dot 移除
                    out.push('i');
                    i += 1;
                }
                'i' => out.push('\u{0130}'),
                _ => out.extend(c.to_uppercase()),
            }
        } else {
            match c {
                'I' if next == Some('\u{0307}') => {
                    // I + combining dot → i + dot（不重复点）
                    out.push('i');
                    i += 1;
                    out.push('\u{0307}');
                }
                'I' => out.push('\u{0131}'),
                _ => out.extend(c.to_lowercase()),
            }
        }
        i += 1;
    }
    out
}

/// Java FTL.jj :2230-2238 `BuiltInBannedWhenAutoEscaping` 检查：legacy 转义内建
/// （?html/?web_safe/?xml/?rtf）在 auto-escaping on + markup 输出格式时禁用——
/// 防止双重转义。Java 在解析期检查（ParseException），Rust 在求值期检查
/// （文档化差异）；错误消息逐字对齐（FTL.jj :2233-2236）。
pub(crate) fn check_legacy_escaping_ban(env: &crate::core::Environment, name: &str) -> Result<()> {
    // auto_escaping 生效判定与 environment.rs :563-568 一致
    // （Java FTL.jj :355-370 updateAutoEscaping）
    let auto_escape = match env.settings.auto_escaping {
        crate::core::AutoEscaping::On => true,
        crate::core::AutoEscaping::Off => false,
        crate::core::AutoEscaping::Default => env.settings.output_format.is_markup(),
    };
    if env.settings.output_format.is_markup() && auto_escape {
        return Err(TemplateError::misc(format!(
            "Using ?{name} (legacy escaping) is not allowed when auto-escaping is on with a markup output format ({}), to avoid double-escaping mistakes.",
            env.settings.output_format.name()
        )));
    }
    Ok(())
}

/// 字符串内建（目标按 Java EvalUtil.coerceModelToStringOrMarkup 强制转字符串：
/// 数字按 number_format、布尔按 boolean_format——默认格式下报错、日期/标量原样）
pub(crate) fn str_builtin(
    env: &mut crate::core::Environment,
    target: &Expr,
    f: impl Fn(&str) -> String,
) -> Result<Option<TModel>> {
    let m = eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    let s = model_to_string(env, &m)?;
    Ok(Some(TModel::from_scalar(f(&s))))
}

/// 数字解析（Java 按 number_format 解析；v1：整数 → Int/Long/BigInt，小数 → Decimal，
/// INF/NaN 家族 → Double——Java `_NumberUtil` 支持 "INF"/"Infinity"/"NaN"）
pub(crate) fn parse_number(s: &str) -> Result<TNumber> {
    let t = java_trim(s);
    match t {
        "INF" | "Infinity" => return Ok(TNumber::Double(f64::INFINITY)),
        "-INF" | "-Infinity" => return Ok(TNumber::Double(f64::NEG_INFINITY)),
        "NaN" => return Ok(TNumber::Double(f64::NAN)),
        _ => {}
    }
    if let Ok(i) = t.parse::<i64>() {
        return Ok(TNumber::from_i64(i));
    }
    if let Ok(b) = t.parse::<num_bigint::BigInt>() {
        return Ok(TNumber::BigInt(b));
    }
    if let Ok(d) = t.parse::<bigdecimal::BigDecimal>() {
        return Ok(TNumber::Decimal(d));
    }
    Err(TemplateError::misc(format!("{s} is not a number")))
}

/// 取第 n 个参数表达式（惰性内建不预求值）
pub(crate) fn arg_expr<'a>(
    args: &'a super::BuiltinArgs,
    idx: usize,
    err: &str,
) -> Result<&'a Expr> {
    args.exprs
        .and_then(|a| a.get(idx))
        .ok_or_else(|| TemplateError::misc(err.to_string()))
}

/// JSON 值 → 模型（Java JSONParser.parse 的类型映射：object→hash、array→sequence、
/// 数字→Integer/Long/Double、字符串/布尔/null 直映；与 freemarker-test 的
/// json_to_model 同口径）
/// 从字符下标切出子串（Java String.substring 语义近似；下标为 char 计数）
/// ?join 逐项拼接（Java BIMethodForCollection.exec :216-247）：null 项跳过
/// （idx 仍递增）；非 null 项间插 separator；转换错误包装
/// `"?join" failed at index {idx} with this error:...`（:230-238，
/// _MessageUtil.EMBEDDED_MESSAGE_BEGIN/END = "---begin-message---\\n" / "\\n---end-message---"）
pub(crate) fn char_index_from(s: &str, from: usize) -> Option<&str> {
    let mut chars = 0;
    for (i, _) in s.char_indices() {
        if chars == from {
            return Some(&s[i..]);
        }
        chars += 1;
    }
    if chars == from {
        Some("")
    } else {
        None
    }
}
