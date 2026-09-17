//! 宏参数绑定与默认参数求值 —— 对应 Java `setMacroContextLocalsFromArguments`
//! （Environment.java:919-1094）与 `Macro.Context.checkParamsSetAndApplyDefaults`
//! （Macro.java:255-340），含 `.args` 惰性构建。

use super::environment_macros::{MacroFrame, WithArgs, WithArgsKind};
use super::Environment;
use crate::core::{Expr, MacroDef, MacroParam};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use indexmap::IndexMap;
use std::rc::Rc;

/// 校验一次写入后不会超过单个输出/捕获缓冲上限。
pub(crate) fn ensure_output_limit(current_len: usize, additional_len: usize) -> Result<()> {
    match current_len.checked_add(additional_len) {
        Some(total) if total <= super::MAX_OUTPUT_BYTES => Ok(()),
        _ => Err(TemplateError::misc(format!(
            "Template output exceeds the {}-byte safety limit.",
            super::MAX_OUTPUT_BYTES
        ))),
    }
}

/// 宏参数绑定 —— 对应 Java `setMacroContextLocalsFromArguments`（Environment.java:919-1094，
/// 含 `?with_args` 预绑定合并）。位置参数按声明顺序（catch-all 不计入普通参数槽，
/// Macro.java:74-81）；命名参数匹配普通参数，未声明者进入命名 catch-all 哈希
/// （Java :1017-1039）。
///
/// `with_args` 阶段（Java :917-1003 的 WithArgsState 处理）：
/// - byName：已声明参数直接绑定；其余 → 命名 catch-all —— orderLast=false 立即
///   插入，orderLast=true 先收集（:938-958 的 orderLastByNameCatchAll），调用参数
///   绑定完后再补（:1053-1066，重复键跳过）；
/// - byPosition + orderLast=false：从位置 0 起按声明顺序绑定，溢出进位置 catch-all
///   （:942-961）；orderLast=true：绑定延后到调用参数之后（:1067-1092），调用含
///   命名参数且预绑定非空 → 报错（:971-975）。
pub(crate) fn bind_macro_args(
    env: &mut Environment,
    frame: &Rc<MacroFrame>,
    def: &MacroDef,
    args: &[(String, crate::core::Expr)],
    with_args: &Option<WithArgs>,
) -> Result<()> {
    let normal_params: Vec<&MacroParam> = def.params.iter().filter(|p| !p.catch_all).collect();
    let catch_all_name = def
        .params
        .iter()
        .find(|p| p.catch_all)
        .map(|p| p.name.clone());
    let mut next_pos = 0usize;
    // Java 命名 catch-all → SimpleHash(LinkedHashMap)：参数插入序
    let mut named_catch_all: Option<IndexMap<String, TModel>> = None;
    let mut positional_catch_all: Option<Vec<TModel>> = None;
    // orderLast 的 byName catch-all 待定条目（Java WithArgsState.orderLastByNameCatchAll）
    let mut order_last_pending: Option<Vec<(String, TModel)>> = None;

    let has_named_call = args.iter().any(|(n, _)| !n.is_empty());
    let has_positional_call = args.iter().any(|(n, _)| n.is_empty());
    let call_positional_count = args.iter().filter(|(n, _)| n.is_empty()).count();

    // Phase 1：?with_args 预绑定（Java :921-1003）
    if let Some(wa) = with_args {
        match &wa.kind {
            WithArgsKind::ByName(bound) => {
                for (arg_name, arg_value) in bound {
                    if normal_params.iter().any(|p| p.name == *arg_name) {
                        // Java :956-957 setLocalVar（含 null 值 —— 后续默认值处理）
                        frame
                            .locals
                            .borrow_mut()
                            .insert(arg_name.clone(), arg_value.clone());
                    } else if let Some(cn) = &catch_all_name {
                        // Java :938-941：首个未声明预绑定键即初始化命名 catch-all
                        // （initNamedCatchAllParameter —— 即使 orderLast 只收集待定
                        // 条目，命名 catch-all 已非 null，位置实参溢出时触发 "both
                        // named and positional" 错误）
                        named_catch_all.get_or_insert_with(IndexMap::new);
                        if wa.order_last {
                            // Java :946-952：收集，待调用参数绑定后补入（不覆盖）
                            order_last_pending
                                .get_or_insert_with(Vec::new)
                                .push((arg_name.clone(), arg_value.clone()));
                        } else {
                            let hash = named_catch_all.as_mut().unwrap();
                            hash.insert(arg_name.clone(), arg_value.clone());
                            frame
                                .locals
                                .borrow_mut()
                                .insert(cn.clone(), TModel::from_hash(hash.clone()));
                        }
                    } else {
                        // Java newUndeclaredParamNameException（Environment.java:1148-1154）
                        return Err(undeclared_param_error(def, &normal_params, arg_name));
                    }
                }
            }
            WithArgsKind::ByPosition(bound) => {
                if wa.order_last {
                    // Java :971-975：调用含命名参数且预绑定非空 → 无法定位 → 报错
                    if has_named_call && !bound.is_empty() {
                        return Err(TemplateError::misc(
                            "Call can't pass parameters by name, as there's \"with args last\" in effect that specifies parameters by position.",
                        ));
                    }
                    // Java :982-988：无 catch-all → 总数预检（调用位置参数 + 预绑定）
                    if catch_all_name.is_none() {
                        let total = call_positional_count + bound.len();
                        if total > normal_params.len() {
                            return Err(too_many_args_error(def, &normal_params, total));
                        }
                    }
                    // 绑定延后到 Phase 3（Java :1067-1092）
                } else {
                    // Java :942-944：预绑定过多且无 catch-all → 报错（总数 = 预绑定数）
                    if normal_params.len() < bound.len() && catch_all_name.is_none() {
                        return Err(too_many_args_error(def, &normal_params, bound.len()));
                    }
                    for arg_value in bound {
                        if next_pos < normal_params.len() {
                            let name = normal_params[next_pos].name.clone();
                            next_pos += 1;
                            frame.locals.borrow_mut().insert(name, arg_value.clone());
                        } else {
                            let cn = catch_all_name
                                .as_ref()
                                .expect("预绑定过多且无 catch-all 已在上方报错");
                            let list = positional_catch_all.get_or_insert_with(Vec::new);
                            list.push(arg_value.clone());
                            let seq = TModel::from_sequence(list.clone());
                            frame.locals.borrow_mut().insert(cn.clone(), seq);
                        }
                    }
                }
            }
        }
    }

    // Phase 2 前置：catch-all 形态初始化（Java :1007-1013 与 :1036-1040 ——
    // 两个分支的 init 条件合并：有命名调用 → 命名 catch-all；有位置调用 → 位置
    // catch-all；无调用实参 → 按 with_args 种类（byPosition → 序列，否则哈希））
    if catch_all_name.is_some() && positional_catch_all.is_none() && named_catch_all.is_none() {
        let by_position = if has_positional_call {
            true
        } else if has_named_call {
            false
        } else {
            matches!(
                &with_args,
                Some(wa) if matches!(wa.kind, WithArgsKind::ByPosition(_))
            )
        };
        if by_position {
            positional_catch_all = Some(Vec::new());
        } else {
            named_catch_all = Some(IndexMap::new());
        }
    }
    // Phase 2 前置：位置参数总数预检（Java :1041-1053）——位置 catch-all 未初始化
    // （即无 catch-all 参数）且总数超声明数 → 命名 catch-all 已有内容 → "both"
    // 错误，否则 too-many 错误
    if positional_catch_all.is_none() {
        let total = call_positional_count + next_pos;
        if normal_params.len() < total {
            if named_catch_all.is_some() {
                return Err(both_named_positional_error(def));
            }
            return Err(too_many_args_error(def, &normal_params, total));
        }
    }

    // Phase 2：调用参数绑定（Java :1004-1052）
    for (arg_name, arg_expr) in args {
        // Java Environment.getVariable 不抛错（缺失变量 → null）：参数求值 lenient
        // （`f(11, null, 33)` 的 null 即缺失变量，Java checkParamsSetAndApplyDefaults
        // 对有默认值的参数回退默认值——Macro.java:273-322）
        let value = match crate::core::eval::eval(env, arg_expr) {
            Ok(v) => v,
            Err(TemplateError::InvalidReference { .. }) => TModel::nothing(),
            Err(e) => return Err(e),
        };
        // Java Macro.Context.checkParamsSetAndApplyDefaults（Macro.java:273-322）：
        // 参数值为 null 时——有默认值 → 求默认值；无默认值且 classic 兼容 → 参数
        // 保持未设置（变量查找回退外层作用域）；strict → "required parameter ...
        // was specified, but had null/missing value."。本引擎 classic 模式跳过绑定
        // （回退外层），strict 保持绑定 nothing（既有偏差，见 docs）
        if value.is_nothing() && env.settings.classic_compatible {
            if arg_name.is_empty() {
                next_pos += 1; // 位置槽仍被消耗（Java localVars 含 null 条目，get 回 null）
            }
            continue;
        }
        if arg_name.is_empty() {
            // 位置参数（Java :1041-1080）
            if next_pos < normal_params.len() {
                let name = normal_params[next_pos].name.clone();
                next_pos += 1;
                frame.locals.borrow_mut().insert(name, value);
            } else if let Some(cn) = &catch_all_name {
                let list = positional_catch_all.get_or_insert_with(Vec::new);
                list.push(value);
                let seq = TModel::from_sequence(list.clone());
                frame.locals.borrow_mut().insert(cn.clone(), seq);
            } else {
                // 上方总数预检已拦截；此处为防御分支（Java newTooManyArgumentsException）
                return Err(too_many_args_error(
                    def,
                    &normal_params,
                    call_positional_count + next_pos,
                ));
            }
        } else if normal_params.iter().any(|p| p.name == *arg_name) {
            frame.locals.borrow_mut().insert(arg_name.clone(), value);
        } else if let Some(cn) = &catch_all_name {
            // 命名 catch-all（Java :1019-1036）；位置 catch-all 已有内容 → "both" 错误
            if positional_catch_all.is_some() {
                return Err(both_named_positional_error(def));
            }
            let hash = named_catch_all.get_or_insert_with(IndexMap::new);
            hash.insert(arg_name.clone(), value);
            frame
                .locals
                .borrow_mut()
                .insert(cn.clone(), TModel::from_hash(hash.clone()));
        } else {
            // Java newUndeclaredParamNameException（Environment.java:1148-1154）：
            // "Macro "m" has no parameter with name "b". Valid parameter names are: a"
            return Err(undeclared_param_error(def, &normal_params, arg_name));
        }
    }

    // Phase 3：orderLast 收尾（Java :1053-1092）
    if let Some(wa) = with_args {
        if wa.order_last {
            if let Some(pending) = &order_last_pending {
                for (name, value) in pending {
                    let exists = named_catch_all
                        .as_ref()
                        .is_some_and(|h| h.contains_key(name));
                    if !exists {
                        let hash = named_catch_all.get_or_insert_with(IndexMap::new);
                        hash.insert(name.clone(), value.clone());
                    }
                }
                if let (Some(cn), Some(h)) = (&catch_all_name, &named_catch_all) {
                    frame
                        .locals
                        .borrow_mut()
                        .insert(cn.clone(), TModel::from_hash(h.clone()));
                }
            } else if let WithArgsKind::ByPosition(bound) = &wa.kind {
                for arg_value in bound {
                    if next_pos < normal_params.len() {
                        let name = normal_params[next_pos].name.clone();
                        next_pos += 1;
                        frame.locals.borrow_mut().insert(name, arg_value.clone());
                    } else {
                        let cn = catch_all_name
                            .as_ref()
                            .expect("无 catch-all 时的总数预检已在上方拦截");
                        let list = positional_catch_all.get_or_insert_with(Vec::new);
                        list.push(arg_value.clone());
                        let seq = TModel::from_sequence(list.clone());
                        frame.locals.borrow_mut().insert(cn.clone(), seq);
                    }
                }
            }
        }
    }

    // Java Environment.java:1007-1013/1048-1053：catch-all 未收到任何额外参数时也
    // 必须绑定——by-position 调用（存在位置参数）→ 空序列；by-name 调用 → 空哈希
    // （如 `<@m foo=1/>` 后 `bar` 为 size 0 的哈希，宏体内可直接 ?keys）
    if let Some(cn) = &catch_all_name {
        let bound = frame.locals.borrow().contains_key(cn);
        if !bound {
            let by_position = if has_positional_call {
                true
            } else if has_named_call {
                false
            } else {
                matches!(
                    &with_args,
                    Some(wa) if matches!(wa.kind, WithArgsKind::ByPosition(_))
                )
            };
            let value = if by_position {
                TModel::from_sequence(Vec::new())
            } else {
                TModel::from_hash(indexmap::IndexMap::new())
            };
            frame.locals.borrow_mut().insert(cn.clone(), value);
        }
    }
    Ok(())
}

/// Java newTooManyArgumentsException（Environment.java:1130-1136）：
/// "Macro "m" only accepts 3 parameters, but got 4."
fn too_many_args_error(def: &MacroDef, normal_params: &[&MacroParam], cnt: usize) -> TemplateError {
    TemplateError::misc(format!(
        "{} {} only accepts {} parameters, but got {}.",
        if def.is_function { "Function" } else { "Macro" },
        quote_name(&def.name),
        normal_params.len(),
        cnt,
    ))
}

/// Java newBothNamedAndPositionalCatchAllParamsException（Environment.java:1155-1160）
fn both_named_positional_error(def: &MacroDef) -> TemplateError {
    TemplateError::misc(format!(
        "{} {} call can't have both named and positional arguments that has to go into catch-all parameter.",
        if def.is_function { "Function" } else { "Macro" },
        quote_name(&def.name),
    ))
}

/// Java newUndeclaredParamNameException（Environment.java:1148-1154）：
/// "Macro "m" has no parameter with name "b". Valid parameter names are: a"
fn undeclared_param_error(
    def: &MacroDef,
    normal_params: &[&MacroParam],
    arg_name: &str,
) -> TemplateError {
    let valid: Vec<&str> = normal_params.iter().map(|p| p.name.as_str()).collect();
    TemplateError::misc(format!(
        "{} {} has no parameter with name {}. Valid parameter names are: {}",
        if def.is_function { "Function" } else { "Macro" },
        quote_name(&def.name),
        quote_name(arg_name),
        valid.join(", "),
    ))
}

/// 默认参数求值 —— 对应 Java `Macro.Context.checkParamsSetAndApplyDefaults`
/// （Macro.java:255-340，bytecode 实测）：
///
/// 多遍重试循环（默认值可相互引用，如 `b=c[a] a=d c={"3":"4"}`）：
/// 每遍遍历参数——已设置跳过；带默认值 → 求值：成功非 null → 绑定 +
/// somethingChanged；求值为 null → 记录首个 null 默认值表达式；抛
/// InvalidReferenceException → 记录首个 IR（Java 只 catch 这一种，其余直接上传）。
/// 遍末：有失败且本遍有赋值 → 整遍重试（失败记录重置）；无失败 → 成功；
/// 失败但无进展 → 抛 firstIR；无 firstIR → 抛默认值表达式的
/// `InvalidReferenceException.getInstance(expr, env)`（classic 兼容吞掉）。
/// 无默认值且未设置 → 循环内立即抛必需参数错误（containsKey 区分
/// "specified, but had null/missing value." 与 "not specified."）。
pub(crate) fn apply_macro_defaults(
    env: &mut Environment,
    frame: &Rc<MacroFrame>,
    def: &MacroDef,
) -> Result<()> {
    loop {
        let mut first_ir: Option<TemplateError> = None;
        let mut first_null_expr: Option<&Expr> = None;
        let mut has_failure = false;
        let mut something_changed = false;
        for (idx, param) in def.params.iter().enumerate() {
            if param.catch_all {
                continue;
            }
            let cur = frame.locals.borrow().get(&param.name).cloned();
            let set = matches!(&cur, Some(m) if !m.is_nothing());
            if set {
                continue;
            }
            if let Some(def_expr) = &param.default {
                match crate::core::eval::eval(env, def_expr) {
                    Ok(v) if !v.is_nothing() => {
                        frame.locals.borrow_mut().insert(param.name.clone(), v);
                        something_changed = true;
                    }
                    Ok(_) => {
                        // 默认值本身为 null → 记录首个 null 默认值表达式
                        // （Java bytecode :115-130；遍末无 firstIR 时按它构造 IR）
                        if !has_failure {
                            first_null_expr = Some(def_expr);
                            has_failure = true;
                        }
                    }
                    Err(e) => {
                        // Java 只 catch InvalidReferenceException 重试
                        // （bytecode Exception table: InvalidReferenceException）；
                        // 其余异常（TypeMismatch 等）直接上传
                        if !matches!(e, TemplateError::InvalidReference { .. }) {
                            return Err(e);
                        }
                        if !has_failure {
                            first_ir = Some(e);
                            has_failure = true;
                        }
                    }
                }
            } else if !env.settings.classic_compatible {
                // 必需参数（无默认值且未设置）→ 循环内立即抛（Java bytecode :176-356）；
                // localVars.containsKey 区分显式传 null 与完全未传
                // （"specified, but had null/missing value." vs "not specified."）
                let specified_but_null = frame.locals.borrow().contains_key(&param.name);
                if specified_but_null {
                    return Err(TemplateError::misc(format!(
                        "When calling {} {}, required parameter {} (parameter #{}) was specified, but had null/missing value.\n\n----\nTip: If the parameter value expression on the caller side is known to be legally null/missing, you may want to specify a default value for it with the \"!\" operator, like paramValue!defaultValue.\n----",
                        if def.is_function { "function" } else { "macro" },
                        quote_name(&def.name),
                        quote_name(&param.name),
                        idx + 1,
                    )));
                }
                return Err(TemplateError::misc(format!(
                    "When calling {} {}, required parameter {} (parameter #{}) was not specified.\n\n----\nTip: If the omission was deliberate, you may consider making the parameter optional in the macro by specifying a default value for it, like <#macro macroName paramName=defaultExpr>)\n----",
                    if def.is_function { "function" } else { "macro" },
                    quote_name(&def.name),
                    quote_name(&param.name),
                    idx + 1,
                )));
            }
        }
        if has_failure && something_changed {
            continue; // 整遍重试（Java goto 29：失败记录与 changed 全部重置）
        }
        if let Some(ir) = first_ir {
            return Err(ir);
        }
        // 无 firstIR：默认值表达式求值为 null → InvalidReferenceException.getInstance
        // （Java bytecode :398-411；blame = 默认值表达式及其位置；classic 兼容吞掉）
        if !env.settings.classic_compatible {
            if let Some(expr) = first_null_expr {
                return Err(TemplateError::invalid_reference_at(
                    super::environment_models::expr_desc(expr),
                    expr.span,
                )
                .with_location(&env.current_template_name, expr.span));
            }
        }
        return Ok(());
    }
}

/// 构造 `.args` 特殊变量值 —— 对应 Java `Macro.Context.checkParamsSetAndApplyDefaults`
/// （Macro.java:344-397）：
/// - macro → SimpleHash（参数名 → 最终值，含默认值解析后；命名 catch-all 哈希展开；
///   位置 catch-all 序列非空 → "The macro can only by called with named arguments,
///   because it uses both .args and a non-empty catch-all parameter."）
/// - function → SimpleSequence（位置参数值 + 位置 catch-all 展开）
///
/// 该函数被 `BuiltinVariable.Args` 惰性调用（Java 仅在访问 `.args` 时构造）；
/// `pub(crate)` 供 eval.rs 的 `.args` 求值路径复用。
pub(crate) fn build_args_special(
    frame: &Rc<MacroFrame>,
    def: &MacroDef,
    is_function: bool,
) -> Result<TModel> {
    let normal: Vec<&MacroParam> = def.params.iter().filter(|p| !p.catch_all).collect();
    let catch_all_name = def
        .params
        .iter()
        .find(|p| p.catch_all)
        .map(|p| p.name.clone());
    let locals = frame.locals.borrow();
    let get = |name: &str| locals.get(name).cloned().unwrap_or_else(TModel::nothing);
    if is_function {
        // Java :346-370：SimpleSequence（参数值 + 位置 catch-all 展开）
        let mut vals: Vec<TModel> = normal.iter().map(|p| get(&p.name)).collect();
        if let Some(cn) = &catch_all_name {
            let catch = get(cn);
            if let Ok(seq) = catch.get_sequence() {
                for i in 0..seq.size()? {
                    vals.push(seq.get(i)?);
                }
            }
        }
        return Ok(TModel::from_sequence(vals));
    }
    // Java :374-396：SimpleHash（参数名 → 值 + 命名 catch-all 展开）
    let mut map: IndexMap<String, TModel> = IndexMap::new();
    for p in &normal {
        map.insert(p.name.clone(), get(&p.name));
    }
    if let Some(cn) = &catch_all_name {
        let catch = get(cn);
        if catch.is_sequence() {
            if catch.get_sequence()?.size()? != 0 {
                return Err(TemplateError::misc(
                    "The macro can only by called with named arguments, because it uses both .args and a non-empty catch-all parameter.",
                ));
            }
        } else if let Some(h) = &catch.hash_ex {
            // Java Macro.java:387-394：catchAllHash.keyValuePairIterator（哈希条目展开）
            for k in h.keys()? {
                if let Some(v) = h.get(&k)? {
                    map.insert(k, v);
                }
            }
        }
    }
    Ok(TModel::from_hash(map))
}

/// Java `_CoreStringUtils.jQuote` 的简化形式（错误消息用）
fn quote_name(s: &str) -> String {
    format!("\"{}\"", s)
}
