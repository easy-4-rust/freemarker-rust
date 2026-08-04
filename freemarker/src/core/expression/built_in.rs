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
use crate::core::eval::{eval, eval_lenient, trunc_i64};
use crate::core::{Expr, ExprKind};
use crate::error::{Result, TemplateError};
use crate::template::utility::java_trim;
use crate::template::TModel;
use crate::value::{DateType, TNumber};

// ---------------------------------------------------------------------------
// 内建函数（Java BuiltInsFor*.java；docs/05 迁移清单）
// ---------------------------------------------------------------------------

/// 内建函数参数表达式视图（惰性内建按需求值）
pub(crate) struct BuiltinArgs<'a> {
    exprs: Option<&'a [Expr]>,
}

/// 内建函数求值（Java `BuiltIn.calculateResult(Environment)`）。
/// 分派顺序：① `crate::builtins::lookup` 注册表（内建函数智能体填表；?replace/?split/
/// ?matches/?string/?c 等 flags/模式类内建已迁入）→ ② 本文件的内建集 →
/// ③ 未命中 `Unknown built-in: ?{name}`（Java 消息）。
pub(crate) fn eval_builtin(
    env: &mut crate::core::Environment,
    target: &Expr,
    name: &str,
    args: &Option<Vec<Expr>>,
) -> Result<TModel> {
    // ① `?itemCycle(a, b, ...)` 带参调用：Java item_cycleBI 的 DirectCall 语义 ——
    // 解析为方法调用 `(x?itemCycle)(...)`（FTL.jj BuiltIn 产生式对
    // BuiltInForLoopVariable 提前 return，括号成为 MethodArgs），BIMethod.exec
    // 直接按迭代 index 返回轮换值；`?itemCycle()` 零参 → 参数个数错误。
    // （无括号的 `?itemCycle` 才是方法模型，落注册表 loop_vars.rs）
    if name == "item_cycle" && args.is_some() {
        return eval_item_cycle_direct(env, target, args.as_deref().unwrap_or(&[]));
    }
    // ①b `"类名"?new(...)` 带参调用：Java FTL.jj 对 NewBI 同样提前 return，
    // 括号成为 MethodArgs（NewBI 是 BuiltIn 但 `?new(args)` 即构造调用，
    // NewBI.java:24-27 → ConstructorFunction.exec）——直接执行构造
    if name == "new" && args.is_some() {
        let m = eval(env, target)?;
        let class_name = crate::core::environment::model_to_string(env, &m)?;
        // Java NewBI._eval（NewBI.java:24-27）：构造器创建时即经
        // env.getNewBuiltinClassResolver().resolve 做权限判定（含 ?new 词法所在
        // 模板名——OptIn 的 trusted_templates 匹配）
        let resolver = env.settings.new_builtin_class_resolver.clone();
        resolver.resolve(&class_name, Some(&env.current_template_name))?;
        let mut vals: Vec<TModel> = Vec::new();
        for a in args.as_deref().unwrap_or(&[]) {
            match eval(env, a) {
                Ok(v) => vals.push(v),
                Err(TemplateError::InvalidReference { .. }) => vals.push(TModel::nothing()),
                Err(e) => return Err(e),
            }
        }
        return crate::template::utility_transforms::new_utility_class(&class_name, &vals);
    }
    // ② 注册表（契约：先 lookup；参数以表达式原样传入——惰性内建 ?then/?switch 需要）
    if let Some(f) = crate::builtins::lookup(name) {
        let r = f(env, target, args.as_deref());
        // 目标类型错误 → Java `For "?{name}" left-hand operand: ... ==> {target}`
        // （BuiltInForString.calculateResult 的 coerceModelToStringOrMarkup blame；
        // 已带 blamer 的错误（?string 等自有措辞）跳过）
        let r = r.map_err(|e| blame_builtin_operand(e, env, target, name));
        if let Some(m) = r? {
            return Ok(m);
        }
        // 注册表返回 None → 落入本文件内建集（保持分派顺序兼容）
    }

    // ③ 内建集（本文件直接实现）
    let ba = BuiltinArgs {
        exprs: args.as_deref(),
    };
    let result = builtin_impl(env, target, name, &ba);
    let result = result.map_err(|e| blame_builtin_operand(e, env, target, name));
    match result {
        Ok(Some(m)) => Ok(m),
        Ok(None) => Err(TemplateError::misc(format!("Unknown built-in: ?{name}"))),
        Err(e) => Err(e),
    }
}

/// 内建左操作数类型错误 → Java `For "?{name}" left-hand operand: ... ==> {target}`
/// 形式（Java BuiltInForString / 各 BuiltIn 的 left-hand operand blame；
/// 仅未带 blamer 的 TypeMismatch——?string 等已在实现处设置自有措辞）
fn blame_builtin_operand(
    e: TemplateError,
    env: &crate::core::Environment,
    target: &Expr,
    name: &str,
) -> TemplateError {
    let lacks_blamer = matches!(
        &e,
        TemplateError::TypeMismatch { ctx, .. } if ctx.blamer.is_none()
    );
    if lacks_blamer {
        e.with_blame_at(
            &format!("?{name}"),
            "left-hand operand",
            &crate::core::environment::expr_desc(target),
            &env.current_template_name,
            target.span,
        )
    } else {
        e
    }
}

/// `?itemCycle(a, b, ...)` 直接取值 —— 对应 Java `item_cycleBI$BIMethod.exec`
/// （BuiltInsForLoopVariables.java:135-148）：按迭代 index 循环取第
/// `index % args.size()` 个参数；零参 → newArgCntError 消息逐字
/// （"expects 1 or more (unlimited) arguments but has received none."）。
fn eval_item_cycle_direct(
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
struct NewConstructorFunction {
    class_name: String,
}

impl crate::template::TemplateMethodModelEx for NewConstructorFunction {
    fn exec(&self, _env: &mut crate::core::Environment, args: Vec<TModel>) -> Result<TModel> {
        crate::template::utility_transforms::new_utility_class(&self.class_name, &args)
    }
}

/// 内建实现：返回 None 表示未知内建（由调用方报 "Unknown built-in: ?xxx"）
fn builtin_impl(
    env: &mut crate::core::Environment,
    target: &Expr,
    name: &str,
    args: &BuiltinArgs,
) -> Result<Option<TModel>> {
    // ---- 循环变量内建（Java BuiltInsForLoopVariables.java；须先于 is_* 类型测试）----
    if let Some(v) = loop_state_builtin(env, target, name, args)? {
        return Ok(Some(v));
    }
    match name {
        // ---- 存在性（Java BuiltInsForExistenceHandling.java）----
        "default" => {
            // Java defaultBI（BuiltInsForExistenceHandling.java:78-91）：目标非 null →
            // ConstantMethod（惰性，参数不求值）；null → FIRST_NON_NULL_METHOD.exec(args)
            // —— 遍历参数返回**首个非 null**（参数求值容忍缺失 → null，
            // Environment.getVariable 不抛错；`v?default(w, '-')` 中 w 缺失 → '-'）
            // Java defaultBI：evalMaybeNonexistentTarget（仅括号目标抑制错误）
            let m = eval_lenient(env, target)?;
            if !m.is_nothing() {
                return Ok(Some(m));
            }
            // FIRST_NON_NULL_METHOD.exec(args)：遍历参数返回首个非 null
            // （参数求值容忍缺失 → null，Environment.getVariable 不抛错；
            // `v?default(w, '-')` 中 w 缺失 → '-'）
            let mut result = TModel::nothing();
            for a in args.exprs.unwrap_or(&[]) {
                match eval(env, a) {
                    Ok(v) if !v.is_nothing() => {
                        result = v;
                        break;
                    }
                    Ok(_) => {}
                    Err(TemplateError::InvalidReference { .. }) => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(Some(result))
        }
        "exists" => {
            // Java existsBI：evalMaybeNonexistentTarget != null（仅括号目标抑制错误）
            let m = eval_lenient(env, target)?;
            Ok(Some(TModel::from_boolean(!m.is_nothing())))
        }
        "if_exists" => {
            // Java ifExistsBI：缺失/为 null → TemplateModel.NOTHING（GeneralPurposeNothing：
            // 全能空角色模型——插值输出 ""、布尔 false、序列/哈希为空；
            // 与真缺失不同，不触发 InvalidReference；Ok(nothing) 即 Java 的 null）
            let m = eval_lenient(env, target)?;
            if m.is_nothing() {
                return Ok(Some(TModel::gpn()));
            }
            Ok(Some(m))
        }
        // ---- 类型测试（Java BuiltInsForMultipleTypes.is_*BI；缺失 → false，其他错误上传）----
        "is_string" => is_type_test(env, target, |m| m.is_scalar()),
        "is_number" => is_type_test(env, target, |m| m.is_number()),
        "is_boolean" => is_type_test(env, target, |m| m.is_boolean()),
        "is_date" => is_type_test(env, target, |m| m.is_date()),
        // Java BuiltIn.java:149-150：is_date 与 is_date_like 是同一 BI（misnomer）——
        // 均为 `instanceof TemplateDateModel`（任何日期模型，含 unknown 类型）
        "is_date_like" => is_type_test(env, target, |m| m.is_date()),
        // Java is_dateOfTypeBI（BuiltInsForMultipleTypes.java:291-305）：
        // is_unknown_date_like/is_date_only/is_time/is_datetime
        "is_unknown_date_like" => is_type_test(
            env,
            target,
            |m| matches!(&m.date, Some(d) if d.as_date().map(|dv| dv.kind == DateType::Unknown).unwrap_or(false)),
        ),
        "is_date_only" => is_type_test(
            env,
            target,
            |m| matches!(&m.date, Some(d) if d.as_date().map(|dv| dv.kind == DateType::Date).unwrap_or(false)),
        ),
        "is_time" => is_type_test(
            env,
            target,
            |m| matches!(&m.date, Some(d) if d.as_date().map(|dv| dv.kind == DateType::Time).unwrap_or(false)),
        ),
        "is_datetime" => is_type_test(
            env,
            target,
            |m| matches!(&m.date, Some(d) if d.as_date().map(|dv| dv.kind == DateType::DateTime).unwrap_or(false)),
        ),
        // Java dateType_if_unknownBI（BuiltInsForDates.java:45-67）：未知日期类型 →
        // 转为指定类型；已知类型 → 原样返回；非日期 → NonDateException
        "datetime_if_unknown" => date_type_if_unknown(env, target, DateType::DateTime),
        "date_if_unknown" => date_type_if_unknown(env, target, DateType::Date),
        "time_if_unknown" => date_type_if_unknown(env, target, DateType::Time),
        // Java is_sequenceBI（BuiltInsForMultipleTypes.java:410-418）：ICI ≥ 2.3.24
        // 时排除方法模型（SimpleMethodModel/OverloadedMethodsModel 实现
        // TemplateSequenceModel 但不可 #list）；ICI < 2.3.24 不排除——
        // BeansWrapper 方法模型（GenericMethodModel，TModel.method_indexable）
        // 也算序列（type-builtins 的 min/2.3.21 变体 expected）
        "is_sequence" => {
            let ici = env.settings.incompatible_improvements.to_int();
            is_type_test(env, target, move |m| {
                if ici < 2_003_024 {
                    m.is_sequence() || m.method_indexable
                } else {
                    m.is_sequence() && !m.is_method()
                }
            })
        }
        "is_collection" => is_type_test(env, target, |m| m.is_collection()),
        // Java is_enumerableBI（:319-327）：序列/集合且（ICI < 2.3.21 或非方法模型）
        "is_enumerable" => {
            let ici = env.settings.incompatible_improvements.to_int();
            is_type_test(env, target, move |m| {
                if ici < 2_003_021 {
                    // ICI < 2.3.21：方法模型（GenericMethodModel 实现
                    // TemplateSequenceModel）同样可枚举
                    m.is_sequence() || m.is_collection() || m.method_indexable
                } else {
                    (m.is_sequence() || m.is_collection()) && !m.is_method()
                }
            })
        }
        // Java is_indexableBI（:350-355）：instanceof TemplateSequenceModel——
        // BeansWrapper 方法模型（GenericMethodModel）同样实现之（bean.m?is_indexable
        // → true）；自定义方法模型不实现 → false（TModel.method_indexable）
        "is_indexable" => is_type_test(env, target, |m| m.is_sequence() || m.method_indexable),
        "is_hash" => is_type_test(env, target, |m| m.is_hash()),
        "is_hash_ex" => is_type_test(env, target, |m| m.is_hash_ex()),
        "is_method" => is_type_test(env, target, |m| m.is_method()),
        "is_directive" => is_type_test(env, target, |m| m.is_directive()),
        "is_macro" => is_type_test(env, target, |m| m.is_macro()),
        "is_node" => is_type_test(env, target, |m| m.is_node()),
        "is_nothing" => is_type_test(env, target, |m| {
            // GeneralPurposeNothing（Java TemplateNullModel.INSTANCE，?if_exists 缺失
            // 返回值）同样算 nothing —— type_name 均为 "nothing"（TModel::gpn :96）
            m.is_nothing() || m.type_name == "nothing"
        }),
        "is_markup_output" => is_type_test(env, target, |m| m.is_markup_output()),
        "is_transform" => is_type_test(env, target, |m| m.is_transform()),
        "has_api" => is_type_test(env, target, |m| m.api.is_some()),
        // v1 扩展：lambda 槽位测试（Java 侧为 ?is_callable 家族，P4 对齐）
        "is_lambda" => is_type_test(env, target, |m| m.is_lambda()),
        // ---- 字符串（Java BuiltInsForStringsBasic / Misc / Encoding / Regexp）----
        // Java upperCaseBI/lowerCaseBI：str.toUpperCase(locale)/toLowerCase(locale)
        // （locale 感知；tr/az 的 i→İ、I→ı 特殊规则）
        "upper_case" => {
            let locale = env.settings.locale.clone();
            str_builtin(env, target, |s| locale_case(s, &locale, true))
        }
        "lower_case" => {
            let locale = env.settings.locale.clone();
            str_builtin(env, target, |s| locale_case(s, &locale, false))
        }
        "cap_first" => str_builtin(env, target, |s| {
            // Java capFirstBI（BuiltInsForStringsBasic.java:44-58）：跳过前导空白，
            // 首个非空白字符大写（Character.toUpperCase——无 locale 规则）
            let mut chars = s.chars();
            let mut skipped = String::new();
            let first = loop {
                match chars.next() {
                    Some(c) if c.is_whitespace() => skipped.push(c),
                    Some(c) => break Some(c),
                    None => break None,
                }
            };
            match first {
                Some(c) => {
                    let mut out = skipped;
                    out.extend(c.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                }
                None => s.to_string(),
            }
        }),
        "trim" => str_builtin(env, target, |s| java_trim(s).to_string()),
        // Java htmlBI（BuiltInsForStringsEncoding.java:36-62）——ICIChainMember 版本链：
        // ICI ≥ 2.3.20 → StringUtil.XHTMLEnc（' → &#39;，HTML_APOS）；ICI < 2.3.20 →
        // StringUtil.HTMLEnc = XMLEncNA（StringUtil.java:69-70：不转义 '）
        "html" | "web_safe" => {
            // Java FTL.jj :2230-2238 BuiltInBannedWhenAutoEscaping：auto-escaping on +
            // markup 输出格式时禁用 legacy 转义内建（防双重转义；Java 在解析期检查，
            // Rust 在求值期检查——文档化差异，消息逐字对齐）
            check_legacy_escaping_ban(env, name)?;
            // Java BuiltIn.java:312：web_safe 是 ?html 的弃用别名（deprecated; use ?html）
            let ici = env.settings.incompatible_improvements.to_int();
            str_builtin(env, target, move |s| {
                if ici < 2_003_020 {
                    crate::template::utility::html_enc_legacy(s)
                } else {
                    crate::template::utility::html_escape(s)
                }
            })
        }
        "xml" => {
            // Java FTL.jj :2230-2238 BuiltInBannedWhenAutoEscaping（同上）
            check_legacy_escaping_ban(env, name)?;
            str_builtin(env, target, crate::template::utility::xml_escape)
        }
        "contains" => {
            let arg = arg_expr(args, 0, "?contains requires one argument")?;
            let m = eval(env, target)?;
            let s = model_to_string(env, &m)?;
            let sub = eval(env, arg)?.get_scalar()?;
            Ok(Some(TModel::from_boolean(s.contains(&sub))))
        }
        "starts_with" => {
            let arg = arg_expr(args, 0, "?starts_with requires one argument")?;
            let m = eval(env, target)?;
            let s = model_to_string(env, &m)?;
            let pre = eval(env, arg)?.get_scalar()?;
            Ok(Some(TModel::from_boolean(s.starts_with(&pre))))
        }
        "ends_with" => {
            let arg = arg_expr(args, 0, "?ends_with requires one argument")?;
            let m = eval(env, target)?;
            let s = model_to_string(env, &m)?;
            let suf = eval(env, arg)?.get_scalar()?;
            Ok(Some(TModel::from_boolean(s.ends_with(&suf))))
        }
        "word_list" => {
            // Java word_listBI：按空白切分（StringUtil.split 空白语义；v1 用 split_whitespace）
            let s = eval(env, target)?.get_scalar()?;
            let v: Vec<TModel> = s
                .split_whitespace()
                .map(|p| TModel::from_scalar(p.to_string()))
                .collect();
            Ok(Some(TModel::from_sequence(v)))
        }
        "index_of" => {
            // Java string_indexOf：返回子串首次出现的字符下标（未找到 → -1）；
            // 目标按 EvalUtil 强制转字符串
            let sub_expr = arg_expr(args, 0, "?index_of requires one argument")?;
            let tm = eval(env, target)?;
            let s = model_to_string(env, &tm)?;
            let sub = eval(env, sub_expr)?.get_scalar()?;
            let from: usize = if args.exprs.is_some_and(|a| a.len() > 1) {
                let e = &args.exprs.as_ref().unwrap()[1];
                let n = trunc_i64(&eval(env, e)?.get_number()?).unwrap_or(0);
                n.max(0) as usize
            } else {
                0
            };
            let idx = match char_index_from(&s, from) {
                Some(rest) => rest
                    .find(&sub)
                    .map(|b| (from + rest[..b].chars().count()) as i64),
                None => None,
            };
            Ok(Some(TModel::from_number(TNumber::Int(
                idx.unwrap_or(-1) as i32
            ))))
        }
        "substring" => {
            // Java substringBI（BuiltInsForStringsBasic.java:609-662）：1-2 参数；
            // 检查顺序：begin<0 → begin>len → end<0 → end>len → begin>end（:625-642）；
            // 错误消息逐字对齐（"at least 0" / "greater than the length" /
            // "shouldn't be greater than the end index"）；字符下标（v1 按 char，UTF-16 P4）
            let a = args.exprs.unwrap_or(&[]);
            if a.is_empty() || a.len() > 2 {
                return Err(TemplateError::misc(format!(
                    "?substring(...) expects 1 or 2 arguments but has received {}.",
                    if a.is_empty() {
                        "none".to_string()
                    } else {
                        a.len().to_string()
                    }
                )));
            }
            let tm = eval(env, target)?;
            let s = model_to_string(env, &tm)?;
            let len = s.chars().count();
            let begin = trunc_i64(&eval(env, &a[0])?.get_number()?).unwrap_or(0);
            if begin < 0 {
                return Err(TemplateError::misc(format!(
                    "The index must be at least 0, but was {begin}."
                )));
            }
            if begin > len as i64 {
                return Err(TemplateError::misc(format!(
                    "The index mustn't be greater than the length of the string, {len}, but it was {begin}."
                )));
            }
            let end = if a.len() > 1 {
                let e = trunc_i64(&eval(env, &a[1])?.get_number()?).unwrap_or(0);
                if e < 0 {
                    return Err(TemplateError::misc(format!(
                        "The index must be at least 0, but was {e}."
                    )));
                }
                if e > len as i64 {
                    return Err(TemplateError::misc(format!(
                        "The index mustn't be greater than the length of the string, {len}, but it was {e}."
                    )));
                }
                if begin > e {
                    return Err(TemplateError::misc(format!(
                        "The begin index argument, {begin}, shouldn't be greater than the end index argument, {e}."
                    )));
                }
                e as usize
            } else {
                len
            };
            Ok(Some(TModel::from_scalar(
                s.chars()
                    .skip(begin as usize)
                    .take(end - begin as usize)
                    .collect(),
            )))
        }
        "length" => {
            // Java lengthBI：字符串 → UTF-16 码元数；序列 → size
            let m = eval(env, target)?;
            if let Some(sc) = &m.scalar {
                return Ok(Some(TModel::from_number(TNumber::from_i64(
                    sc.as_string()?.encode_utf16().count() as i64,
                ))));
            }
            if let Some(seq) = &m.sequence {
                return Ok(Some(TModel::from_number(TNumber::from_i64(
                    seq.size()? as i64
                ))));
            }
            Err(TemplateError::misc(format!(
                "?length is not applicable to a {} value",
                m.type_name
            )))
        }
        "number" => {
            // Java numberBI：数字原样；字符串按数字格式解析
            let m = eval(env, target)?;
            if let Some(n) = &m.number {
                return Ok(Some(TModel::from_number(n.as_number()?)));
            }
            let s = m.get_scalar()?;
            Ok(Some(TModel::from_number(parse_number(&s)?)))
        }
        "eval_json" => {
            // Java evalJsonBI（BuiltInsForStringsMisc.java:116-131）：JSON 字符串
            // 解析为模型；失败消息 = "Failed to "?eval_json" string with this error:"
            // + EMBEDDED_MESSAGE 段 + "The failing expression:"（源码拼接，jar 实测
            // 格式）。内嵌消息用 serde_json 原文（Java JSONParser 逐字消息无 golden/
            // parity 场景覆盖——文档化偏差）
            let m = eval(env, target)?;
            let s = m.get_scalar()?;
            match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(v) => Ok(Some(json_value_to_model(&v))),
                Err(e) => Err(TemplateError::misc(format!(
                    "Failed to \"?eval_json\" string with this error:\n\n---begin-message---\n{e}\n---end-message---\n\nThe failing expression:"
                ))),
            }
        }
        "boolean" => {
            // Java booleanBI（BuiltInsForStringsMisc.java:37）：布尔原样；字符串仅接受
            // 精确 "true"/"false" 或当前 boolean_format 的 true/false 串
            let m = eval(env, target)?;
            if let Some(b) = &m.boolean {
                return Ok(Some(TModel::from_boolean(b.as_boolean()?)));
            }
            // Java BuiltInForString：目标强制转字符串（数字 0 → "0" 再判错）
            let s = model_to_string(env, &m)?;
            let (ts, fs) = crate::core::environment::boolean_format_strings(env)
                .unwrap_or_else(|| ("true".to_string(), "false".to_string()));
            if s == "true" || s == ts {
                Ok(Some(TModel::from_boolean(true)))
            } else if s == "false" || s == fs {
                Ok(Some(TModel::from_boolean(false)))
            } else {
                Err(TemplateError::misc(format!(
                    "Can't convert this string to boolean: {s:?}"
                )))
            }
        }
        "int" | "long" | "float" | "double" => {
            // Java BuiltInsForNumbers.intBI/longBI 等：原始类型强转（**溢出回绕**，
            // Java `num.intValue()`/`num.longValue()` 语义，无范围错误）；
            // 字符串先解析；日期 → 纪元毫秒（Java EvalUtil.modelToLong 的 TemplateDateModel 分支）
            let m = eval(env, target)?;
            let n = if let Some(n) = &m.number {
                n.as_number()?
            } else if m.is_date() {
                let d = m.get_date()?;
                TNumber::Long(d.dt.timestamp_millis())
            } else {
                parse_number(&m.get_scalar()?)?
            };
            Ok(Some(match name {
                // Java intValue()：double/float 截断（向零），溢出按 32 位回绕
                "int" => TModel::from_number(TNumber::Int(trunc_i64(&n).unwrap_or(0) as i32)),
                // Java longValue()：double/float 截断，BigInteger 溢出按 64 位回绕
                "long" => TModel::from_number(TNumber::Long(trunc_i64(&n).unwrap_or(0))),
                "float" => TModel::from_number(TNumber::Float(n.as_f32().unwrap_or(0.0))),
                "double" => TModel::from_number(TNumber::Double(n.as_f64().unwrap_or(0.0))),
                _ => unreachable!(),
            }))
        }
        // ---- 序列（Java BuiltInsForSequences.java）----
        "size" => {
            let m = eval(env, target)?;
            if let Some(seq) = &m.sequence {
                return Ok(Some(TModel::from_number(TNumber::from_i64(
                    seq.size()? as i64
                ))));
            }
            if let Some(ex) = &m.hash_ex {
                return Ok(Some(TModel::from_number(TNumber::from_i64(
                    ex.size()? as i64
                ))));
            }
            // Java：非序列/扩展哈希 → 报错（?size 在无界范围上不可用，与 Java 一致）
            Err(TemplateError::misc(format!(
                "?size is not applicable to a {} value",
                m.type_name
            )))
        }
        "first" => {
            // Java firstBI（BuiltInsForSequences.java:149-189）：序列优先（2.3.x BC），
            // 否则集合迭代；空 → null（下游 InvalidReferenceException）
            let m = eval(env, target)?;
            if let Some(seq) = &m.sequence {
                if seq.size()? == 0 {
                    return Ok(Some(TModel::nothing()));
                }
                return Ok(Some(seq.get(0)?));
            }
            if let Some(c) = &m.collection {
                let mut it = c.iterator()?;
                return match it.next() {
                    Some(v) => Ok(Some(v?)),
                    None => Ok(Some(TModel::nothing())),
                };
            }
            if let Some(sc) = &m.scalar {
                let s = sc.as_string()?;
                return match s.chars().next() {
                    Some(c) => Ok(Some(TModel::from_scalar(c.to_string()))),
                    None => Err(TemplateError::misc("The string is empty, ?first failed")),
                };
            }
            Err(TemplateError::misc(format!(
                "?first is not applicable to a {} value",
                m.type_name
            )))
        }
        "last" => {
            // Java lastBI（BuiltInsForSequences.java:267-277）：空 → null
            let m = eval(env, target)?;
            if let Some(seq) = &m.sequence {
                let size = seq.size()?;
                if size == 0 {
                    return Ok(Some(TModel::nothing()));
                }
                return Ok(Some(seq.get(size - 1)?));
            }
            if let Some(sc) = &m.scalar {
                let s = sc.as_string()?;
                return match s.chars().next_back() {
                    Some(c) => Ok(Some(TModel::from_scalar(c.to_string()))),
                    None => Err(TemplateError::misc("The string is empty, ?last failed")),
                };
            }
            Err(TemplateError::misc(format!(
                "?last is not applicable to a {} value",
                m.type_name
            )))
        }
        "join" => {
            // Java joinBI（BuiltInsForSequences.java:191-265）：1-3 参数
            // （separator / whenEmpty / afterLast，checkMethodArgCount(args, 1, 3)）；
            // null（nothing）元素跳过（:225 `if (item != null)`，idx 仍递增）；
            // 逐项字符串转换错误包装失败索引（:230-238，EMBEDDED_MESSAGE_BEGIN/END）；
            // 右无界数值范围拒绝（:256 checkNotRightUnboundedNumericalRange，:929-935）
            if let Some(a) = args.exprs {
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
            let arg = arg_expr(
                args,
                0,
                "?join(...) expects 1 to 3 arguments but has received none.",
            )?;
            let m = eval(env, target)?;
            if m.range.as_ref().is_some_and(|r| r.unbounded) {
                return Err(TemplateError::misc(
                    "The input sequence is a right-unbounded numerical range, thus, it's infinitely long, and can't processed with this built-in.",
                ));
            }
            let sep = eval(env, arg)?.get_scalar()?;
            let when_empty = match args.exprs.and_then(|a| a.get(1)) {
                Some(a) => Some(eval(env, a)?.get_scalar()?),
                None => None,
            };
            let after_last = match args.exprs.and_then(|a| a.get(2)) {
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
        "reverse" => {
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
        "seq_contains" => {
            // Java seq_containsBI（BuiltInsForSequences.java:308-380）：checkMethodArgCount(1)；
            // 序列优先（2.3.x BC），否则集合迭代；参数缺失变量 → null → modelsEqual false
            crate::core::eval_util::check_arg_count("seq_contains", args.exprs, 1, 1)?;
            let m = eval(env, target)?;
            let needle = crate::builtins::sequences::eval_arg_lenient(env, args.exprs, 0)?;
            let items = crate::builtins::sequences::seq_or_collection_items(&m, "seq_contains")?;
            for (i, item) in items.iter().enumerate() {
                if crate::builtins::sequences::models_equal(i, item, &needle, Some(env))? {
                    return Ok(Some(TModel::from_boolean(true)));
                }
            }
            Ok(Some(TModel::from_boolean(false)))
        }
        // ---- 哈希（Java BuiltInsForHashes.java）----
        "keys" => crate::builtins::hashes::keys(env, target, args.exprs),
        "values" => crate::builtins::hashes::values(env, target, args.exprs),
        // ---- 输出/格式化（Java BuiltInsForMultipleTypes.java）----
        "has_content" => {
            // Java hasContentBI：evalMaybeNonexistentTarget（仅括号目标抑制）→ isEmpty
            let m = eval_lenient(env, target)?;
            Ok(Some(TModel::from_boolean(m.has_content()?)))
        }
        // ---- 动态解释（Java Interpret.java；BuiltIn.java:144）----
        "interpret" => {
            // 注：Java 的 ?interpret 返回惰性变换模型；本实现同语义（变换模型槽位）
            let r = builtin_interpret(env, target)?;
            Ok(Some(r))
        }
        // ---- 类实例化（Java NewBI.java：?new 返回 ConstructorFunction 方法模型，
        // 调用 `"类名"?new(args)` 时经 BeansWrapper.newInstance 实例化）----
        "new" => {
            // Java NewBI._eval（NewBI.java:24-27）：target 求值为类名字符串
            let m = eval(env, target)?;
            let class_name = crate::core::environment::model_to_string(env, &m)?;
            // Java：resolve 在构造器创建时执行（模板解析期 classname 已知时）——
            // v1 延迟到方法调用时（等价；类名解析错误消息对齐 Java）；
            // 权限判定与 Java 同步在此处（NewBI.java:32-38：resolve(className, env,
            // target.getTemplate())——template 即 ?new 词法所在模板，OptIn 的
            // trusted_templates 按它匹配）
            let resolver = env.settings.new_builtin_class_resolver.clone();
            resolver.resolve(&class_name, Some(&env.current_template_name))?;
            Ok(Some(TModel::from_method(NewConstructorFunction {
                class_name,
            })))
        }
        // ---- 表达式动态求值（Java BuiltInsForStringsMisc.evalBI）----
        "eval" => {
            // Java：源码包为 `(...)` 按表达式解析 → 求值；解析/求值失败均包进
            // "Failed to \"?eval\" string with this error: ..." 消息（:87-118）；
            // 但求值为 null（缺失变量）**不包装**——原样返回 null（Java exp.eval
            // 返回 null，`'fails'?eval!'-'` → evalBI null → 默认值 '-' 生效）
            let m = eval(env, target)?;
            let s = crate::core::environment::model_to_string(env, &m)?;
            let cfg = env.template.configuration.clone();
            let expr = crate::parser::parse_expression(&cfg, &s).map_err(|e| {
                TemplateError::misc(format!(
                    "Failed to \"?eval\" string with this error:\n\n{e}\n\nThe failing expression:"
                ))
            })?;
            // Java：?eval 字符串的源码没有宏上下文——`.args` 在其中静态非法
            // （FMParser 的 args 特殊变量检查；jar 实测消息逐字）
            if matches!(
                expr.kind,
                crate::core::ExprKind::BuiltinVar(crate::core::BuiltinVar::Args)
            ) {
                return Err(TemplateError::misc(format!(
                    "Failed to \"?eval\" string with this error:\n\n---begin-message---\nSyntax error in ?eval-ed string in line 1, column 3:\nThe \"args\" special variable must be inside a macro or function in the template source code.\n---end-message---\n\nThe failing expression:\n==> '{s}'?eval"
                )));
            }
            match eval(env, &expr) {
                Ok(v) => Ok(Some(v)),
                Err(TemplateError::InvalidReference { .. }) => Ok(Some(TModel::nothing())),
                Err(e) => Err(TemplateError::misc(format!(
                    "Failed to \"?eval\" string with this error:\n\n{e}\n\nThe failing expression:"
                ))),
            }
        }
        // ---- 其余未知内建：由调用方报 Unknown built-in ----
        _ => Ok(None),
    }
}

/// 循环变量内建（Java BuiltInsForLoopVariables.java：index/counter/has_next/has_previous/
/// is_first/is_last/is_odd/is_even/is_odd_item/is_even_item）
fn loop_state_builtin(
    env: &mut crate::core::Environment,
    target: &Expr,
    name: &str,
    args: &BuiltinArgs,
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
fn builtin_interpret(env: &mut crate::core::Environment, target: &Expr) -> Result<TModel> {
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
fn date_type_if_unknown(
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

fn is_type_test(
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
fn locale_case(s: &str, locale: &str, upper: bool) -> String {
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
fn str_builtin(
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
fn parse_number(s: &str) -> Result<TNumber> {
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
fn arg_expr<'a>(args: &'a BuiltinArgs, idx: usize, err: &str) -> Result<&'a Expr> {
    args.exprs
        .and_then(|a| a.get(idx))
        .ok_or_else(|| TemplateError::misc(err.to_string()))
}

/// JSON 值 → 模型（Java JSONParser.parse 的类型映射：object→hash、array→sequence、
/// 数字→Integer/Long/Double、字符串/布尔/null 直映；与 freemarker-test 的
/// json_to_model 同口径）
fn json_value_to_model(v: &serde_json::Value) -> TModel {
    match v {
        serde_json::Value::Null => TModel::nothing(),
        serde_json::Value::Bool(b) => TModel::from_boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TModel::from_number(crate::value::TNumber::from_i64(i))
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f.is_finite() {
                    TModel::from_number(crate::value::TNumber::from_i64(f as i64))
                } else {
                    TModel::from_number(crate::value::TNumber::Double(f))
                }
            } else {
                TModel::nothing()
            }
        }
        serde_json::Value::String(s) => TModel::from_scalar(s.clone()),
        serde_json::Value::Array(arr) => {
            TModel::from_sequence(arr.iter().map(json_value_to_model).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = indexmap::IndexMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_value_to_model(v));
            }
            TModel::from_hash(map)
        }
    }
}

/// 从字符下标切出子串（Java String.substring 语义近似；下标为 char 计数）
/// ?join 逐项拼接（Java BIMethodForCollection.exec :216-247）：null 项跳过
/// （idx 仍递增）；非 null 项间插 separator；转换错误包装
/// `"?join" failed at index {idx} with this error:...`（:230-238，
/// _MessageUtil.EMBEDDED_MESSAGE_BEGIN/END = "---begin-message---\\n" / "\\n---end-message---"）
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

fn char_index_from(s: &str, from: usize) -> Option<&str> {
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
