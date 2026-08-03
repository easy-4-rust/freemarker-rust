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

use crate::core::environment::{expr_desc, lambda_model, model_to_string};
use crate::core::{
    ArithmeticEngine, BigDecimalEngine, BuiltinVar, Expr, ExprKind, RangeKind, StrPart,
};
use crate::error::{Result, TemplateError};
use crate::span::Span;
use crate::template::{
    TModel, TemplateCollectionModel, TemplateHashModel, TemplateHashModelEx, TemplateSequenceModel,
};
use crate::utility::java_trim;
use crate::value::{DateType, DateValue, TNumber};
use bigdecimal::ToPrimitive;
use indexmap::IndexMap;
use std::cmp::Ordering;
use std::rc::Rc;
use unicode_normalization::UnicodeNormalization;

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
        ExprKind::Str(s) => Ok(TModel::from_scalar(s.clone())),
        ExprKind::InterpStr(parts) => eval_interp_str(env, parts),
        ExprKind::Num(n) => Ok(TModel::from_number(n.clone())),
        ExprKind::Bool(b) => Ok(TModel::from_boolean(*b)),
        ExprKind::Ident(name) => env.get_variable(name),
        ExprKind::BuiltinVar(v) => eval_builtin_var(env, *v),
        ExprKind::Dot { target, name } => eval_dot(env, target, name),
        ExprKind::DynKey { target, key } => eval_dyn_key(env, target, key),
        ExprKind::Call { callee, args } => eval_call(env, callee, args),
        ExprKind::UnaryMinus(t) => {
            // Java UnaryPlusMinusExpression.java:42 _eval（TYPE_MINUS → ArithmeticEngine.negate）；
            // 操作数 null → modelToNumber → NonNumericalException（消息同 InvalidReference）
            let m = eval(env, t)?;
            if m.is_nothing() {
                return Err(TemplateError::invalid_reference(
                    crate::core::environment::expr_desc(t),
                ));
            }
            let n = m.get_number()?;
            let engine = BigDecimalEngine::default();
            Ok(TModel::from_number(engine.negate(&n)?))
        }
        ExprKind::Not(t) => {
            // Java NotExpression `evalToBoolean` → modelToBoolean（classic 兼容见下）
            let m = eval(env, t)?;
            let b = model_to_boolean(env, &m)?;
            Ok(TModel::from_boolean(!b))
        }
        ExprKind::Add(a, b) => eval_add(env, a, b),
        ExprKind::Sub(a, b) => eval_binary_number(env, a, b, NumOp::Sub),
        ExprKind::Mul(a, b) => eval_binary_number(env, a, b, NumOp::Mul),
        ExprKind::Div(a, b) => eval_binary_number(env, a, b, NumOp::Div),
        ExprKind::Mod(a, b) => eval_binary_number(env, a, b, NumOp::Mod),
        ExprKind::Eq(a, b) => eval_compare(env, a, b, CmpOp::Eq),
        ExprKind::NotEq(a, b) => eval_compare(env, a, b, CmpOp::NotEq),
        ExprKind::Gt(a, b) => eval_compare(env, a, b, CmpOp::Gt),
        ExprKind::Gte(a, b) => eval_compare(env, a, b, CmpOp::Gte),
        ExprKind::Lt(a, b) => eval_compare(env, a, b, CmpOp::Lt),
        ExprKind::Lte(a, b) => eval_compare(env, a, b, CmpOp::Lte),
        ExprKind::And(a, b) => {
            // Java AndExpression：短路（lho.evalToBoolean && rho.evalToBoolean）
            let lm = eval(env, a)?;
            let l = model_to_boolean(env, &lm)?;
            if !l {
                return Ok(TModel::from_boolean(false));
            }
            let rm = eval(env, b)?;
            let r = model_to_boolean(env, &rm)?;
            Ok(TModel::from_boolean(l && r))
        }
        ExprKind::Or(a, b) => {
            // Java OrExpression：短路
            let lm = eval(env, a)?;
            let l = model_to_boolean(env, &lm)?;
            if l {
                return Ok(TModel::from_boolean(true));
            }
            let rm = eval(env, b)?;
            let r = model_to_boolean(env, &rm)?;
            Ok(TModel::from_boolean(l || r))
        }
        ExprKind::Range { start, end, kind } => eval_range(env, start, end, *kind),
        ExprKind::Default { target, default } => eval_default_to(env, target, default),
        ExprKind::Exists(t) => eval_exists(env, t),
        ExprKind::BuiltIn { target, name, args } => eval_builtin(env, target, name, args),
        ExprKind::ListLit(items) => {
            // Java ListLiteral：逐元素求值 → SimpleSequence
            let mut v = Vec::with_capacity(items.len());
            for i in items {
                v.push(eval(env, i)?);
            }
            Ok(TModel::from_sequence(v))
        }
        ExprKind::HashLit(pairs) => eval_hash_lit(env, pairs),
        ExprKind::Lambda { params, body } => {
            // Java LocalLambdaExpression：v1 仅构造槽位模型（?map/?filter 消费方由内建智能体扩展）
            Ok(lambda_model(params.clone(), Rc::new((**body).clone())))
        }
        ExprKind::Paren(inner) => eval(env, inner),
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

/// 插值字符串（Java StringLiteral 的插值片段拼接；各片段按输出字符串规则转换）
fn eval_interp_str(env: &mut crate::core::Environment, parts: &[StrPart]) -> Result<TModel> {
    let mut out = String::new();
    for part in parts {
        match part {
            StrPart::Text(t) => out.push_str(t),
            StrPart::Interp(e) => {
                let m = eval(env, e)?;
                if m.is_nothing() {
                    // Java EvalUtil.coerceModelToTextualCommon：tm == null 时 classic 兼容
                    // 模式回退空串（EvalUtil.java:486-489），否则 InvalidReferenceException。
                    if env.settings.classic_compatible {
                        continue;
                    }
                    return Err(TemplateError::invalid_reference(
                        crate::core::environment::expr_desc(e),
                    ));
                }
                out.push_str(&model_to_string(env, &m)?);
            }
        }
    }
    Ok(TModel::from_scalar(out))
}

/// 内置变量 —— 对应 Java `BuiltinVariable._eval`（BuiltinVariable.java:186-300）
/// 语义对照见 expression.rs BuiltinVar 各 variant 注释
fn eval_builtin_var(env: &mut crate::core::Environment, v: BuiltinVar) -> Result<TModel> {
    match v {
        BuiltinVar::True => Ok(TModel::from_boolean(true)),
        BuiltinVar::False => Ok(TModel::from_boolean(false)),
        // Java :249-252：NOW → new SimpleDate(new Date(), DATETIME)
        BuiltinVar::Now => Ok(TModel::from_date(DateValue {
            dt: chrono::Utc::now().fixed_offset(),
            kind: DateType::DateTime,
            is_sql: false,
        })),
        // Java :188：NAMESPACE → env.getCurrentNamespace()
        BuiltinVar::Namespace => Ok(crate::core::environment::namespace_model(
            env.get_current_namespace(),
        )),
        // Java :189-190：MAIN → env.getMainNamespace()
        BuiltinVar::Main => Ok(crate::core::environment::namespace_model(
            env.get_main_namespace(),
        )),
        // Java :191-192：GLOBALS → env.getGlobalVariables()（Environment.java:2861-2878：
        // globalNamespace → rootDataModel → sharedVariables 的复合哈希）
        BuiltinVar::Globals => Ok(globals_model(env)),
        // Java :193-196：LOCALS → 当前宏帧局部变量哈希；无宏帧 → null
        BuiltinVar::Locals => match env.get_current_macro_frame() {
            Some(frame) => {
                let locals = frame.locals.borrow();
                let mut map = indexmap::IndexMap::new();
                for (k, m) in locals.iter() {
                    map.insert(k.clone(), m.clone());
                }
                Ok(TModel::from_hash(map))
            }
            None => Ok(TModel::nothing()),
        },
        // Java :197-198：DATA_MODEL → env.getDataModel()（Environment.java:2811-2845：
        // rootDataModel → sharedVariables 的复合哈希）
        BuiltinVar::DataModel => Ok(data_model_model(env)),
        // Java :199-200：VARS → VarsHash（get = 完整变量解析链；v1 快照近似）
        BuiltinVar::Vars => {
            let map = crate::core::environment::vars_snapshot(env);
            Ok(TModel::from_hash(map))
        }
        // Java :201-203：LOCALE → locale.toString()
        BuiltinVar::Locale => Ok(TModel::from_scalar(env.settings.locale.clone())),
        // Java :204-206：LOCALE_OBJECT → ObjectWrapper.wrap(Locale)（v1：Java 描述串）
        BuiltinVar::LocaleObject => Ok(TModel::from_scalar(format!(
            "java.util.Locale \"{}\"",
            env.settings.locale
        ))),
        // Java :207-208：LANG → locale.getLanguage()
        BuiltinVar::Lang => Ok(TModel::from_scalar(
            env.settings.locale.split(['_', '-']).next().unwrap_or("").to_string(),
        )),
        // Java :209-211：CURRENT_NODE/NODE → env.getCurrentVisitorNode()（Environment.java:2931-2933；
        // 非节点上下文返回 null —— nothing 而非抛错，`!`/`??` 继续抑制）
        BuiltinVar::Node => Ok(env
            .get_current_visitor_node()
            .unwrap_or_else(TModel::nothing)),
        // Java :212-217：TEMPLATE_NAME → 主模板名（getTemplate230().getName()）
        BuiltinVar::TemplateName | BuiltinVar::MainTemplateName => Ok(TModel::from_scalar(
            env.template.name.clone(),
        )),
        // Java :218-220：CURRENT_TEMPLATE_NAME → 当前执行模板名
        BuiltinVar::CurrentTemplateName => Ok(TModel::from_scalar(
            env.current_template_name.clone(),
        )),
        // Java :228-229：OUTPUT_ENCODING → getOutputEncoding()（未设置 → null）
        BuiltinVar::OutputEncoding => {
            let enc = env.settings.output_encoding.trim();
            if enc.is_empty() {
                Ok(TModel::nothing())
            } else {
                Ok(TModel::from_scalar(enc.to_string()))
            }
        }
        // Java :230-231：URL_ESCAPING_CHARSET → getURLEscapingCharset()
        BuiltinVar::UrlEscapingCharset => {
            let enc = env.settings.url_escaping_charset.trim();
            if enc.is_empty() {
                Ok(TModel::nothing())
            } else {
                Ok(TModel::from_scalar(enc.to_string()))
            }
        }
        // Java :232-234：ERROR → getCurrentRecoveredErrorMessage()（recoveredErrorStack 栈顶）
        BuiltinVar::Error => Ok(TModel::from_scalar(
            env.recovered_errors.last().cloned().unwrap_or_default(),
        )),
        // Java :238-239：VERSION → Configuration.getVersionNumber()
        BuiltinVar::Version => Ok(TModel::from_scalar("2.3.34".to_string())),
        // Java :240-241：INCOMPATIBLE_IMPROVEMENTS
        BuiltinVar::IncompatibleImprovements => {
            Ok(TModel::from_scalar(format!(
                "{}.{}.{}",
                env.settings.incompatible_improvements.major,
                env.settings.incompatible_improvements.minor,
                env.settings.incompatible_improvements.micro
            )))
        }
        // Java :250-253：OUTPUT_FORMAT → OutputFormat.getName()
        BuiltinVar::OutputFormat => {
            Ok(TModel::from_scalar(env.settings.output_format.name().to_string()))
        }
        // Java :254-256：AUTO_ESC
        BuiltinVar::AutoEsc => Ok(TModel::from_boolean(env.is_auto_escape())),
        // Java :264-267：TIME_ZONE → getTimeZone().getID()
        BuiltinVar::TimeZone => Ok(TModel::from_scalar(env.settings.time_zone_id.clone())),
        // Java :257-263：ARGS → 宏/函数参数哈希（仅宏内；v1 不支持 → 明确报错）
        // Java BuiltinVariable.java:269-276 + getRequiredMacroContext :285-293：
        // .args → 当前宏帧的参数值（macro → 哈希 / function → 序列）；
        // 宏外 → "Can't get .args here, as there's no macro or function (that's
        // implemented in the template) call in context."
        // Java BuiltinVariable.Args 访问时才构造（惰性）：位置 catch-all 非空 +
        // 访问 .args → 报错；不访问 .args 的宏不受该限制（jar 实测 2.3.34）
        BuiltinVar::Args => match env.get_current_macro_frame() {
            Some(frame) => {
                if let Some(v) = frame.args_value.borrow().as_ref().map(|b| b.as_ref().clone()) {
                    return Ok(v);
                }
                let v = crate::core::environment::build_args_special(
                    &frame,
                    &frame.def,
                    frame.is_function,
                )?;
                *frame.args_value.borrow_mut() = Some(Box::new(v.clone()));
                Ok(v)
            }
            None => Err(TemplateError::misc(
                "Can't get .args here, as there's no macro or function (that's implemented in the template) call in context.",
            )),
        },
    }
}

/// `.globals` 哈希 —— 对应 Java `Environment.getGlobalVariables()`
/// （Environment.java:2861-2878）：只读**普通**哈希（非 extended），
/// get(key) = globalNamespace → rootDataModel → sharedVariables 的活视图
/// （各源以 Rc/克隆持有，不借用 env —— 与求值期的 &mut env 兼容）。
struct GlobalsHash {
    global_ns: Rc<crate::core::environment::Namespace>,
    root: TModel,
    shared: std::collections::HashMap<String, TModel>,
}

impl TemplateHashModel for GlobalsHash {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        if let Some(m) = self.global_ns.get_member(key) {
            return Ok(Some(m));
        }
        if let Ok(h) = self.root.get_hash() {
            if let Some(m) = h.get(key)? {
                return Ok(Some(m));
            }
        }
        Ok(self.shared.get(key).cloned())
    }
    fn is_empty(&self) -> Result<bool> {
        // Java getGlobalVariables().isEmpty() 恒 false（get 会落到数据模型/共享变量）
        Ok(false)
    }
}

/// `.data_model` 哈希 —— 对应 Java `Environment.getDataModel()`（Environment.java:2811-2845）：
/// get(key) = rootDataModel → sharedVariables（getDataModelOrSharedVariable :2495-2499）；
/// root 为 extended 时 keys/size 委托 root（Java 注释 "NB: The methods below do not take
/// into account configuration shared variables ..., if only for BWC reasons"）
struct DataModelHash {
    root: TModel,
    shared: std::collections::HashMap<String, TModel>,
}

impl TemplateHashModel for DataModelHash {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        if let Ok(h) = self.root.get_hash() {
            if let Some(m) = h.get(key)? {
                return Ok(Some(m));
            }
        }
        Ok(self.shared.get(key).cloned())
    }
    fn is_empty(&self) -> Result<bool> {
        // Java getDataModel().isEmpty() 恒 false（get 会落到共享变量）
        Ok(false)
    }
}

impl TemplateHashModelEx for DataModelHash {
    fn size(&self) -> Result<usize> {
        match &self.root.hash_ex {
            Some(ex) => ex.size(),
            None => Ok(0),
        }
    }
    fn keys(&self) -> Result<Vec<String>> {
        match &self.root.hash_ex {
            Some(ex) => ex.keys(),
            None => Ok(Vec::new()),
        }
    }
}

/// `.globals` 值模型（Java getGlobalVariables：普通 TemplateHashModel）
fn globals_model(env: &crate::core::Environment) -> TModel {
    TModel {
        hash: Some(Rc::new(GlobalsHash {
            global_ns: env.get_global_namespace(),
            root: env.root.clone(),
            shared: env.template.configuration.shared_vars.clone(),
        })),
        hash_ex: None,
        type_name: "hash",
        kind: crate::template::ModelKind::Hash,
        ..TModel::nothing()
    }
}

/// `.data_model` 值模型（Java getDataModel：root extended 时同样 extended）
fn data_model_model(env: &crate::core::Environment) -> TModel {
    let h = Rc::new(DataModelHash {
        root: env.root.clone(),
        shared: env.template.configuration.shared_vars.clone(),
    });
    let rc: Rc<dyn TemplateHashModel> = h.clone();
    let ex: Rc<dyn TemplateHashModelEx> = h;
    TModel {
        hash: Some(rc),
        hash_ex: Some(ex),
        type_name: "hash",
        kind: crate::template::ModelKind::Hash,
        ..TModel::nothing()
    }
}

/// 点访问（Java Dot.java:49-62 `_eval`：目标为哈希/命名空间 → get(key)；否则 NonHashException）
fn eval_dot(env: &mut crate::core::Environment, target: &Expr, name: &str) -> Result<TModel> {
    // `?string.xs` / `?date.xs` / `?datetime.xs` / `?string.yes.no`：Java 中 ?string/?date 等
    // 返回"格式化器"模型（TemplateHashModel.get(key)，BuiltInsForMultipleTypes.java DateFormatter.get
    // :622-627 / dateBI.DateParser.get :146-150），点访问即格式参数；本引擎在求值期把点链
    // 合并为内建参数（解析器生成同样的 Dot(BuiltIn) 嵌套，见 grammar.rs builtin() 注释）
    if let Some((inner, bname, mut names)) = dot_builtin_chain(target) {
        names.push(name.to_string());
        let args: Vec<Expr> = names
            .iter()
            .map(|n| Expr::new(ExprKind::Str(n.clone()), target.span))
            .collect();
        return eval_builtin(env, &inner, &bname, &Some(args));
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
    // 命名空间成员（Java Namespace extends SimpleHash，含宏；`<@ns.macro>`/`ns.var`）；
    // 成员缺失 → Java 返回 null（SimpleHash.get 无键 → null），由使用点抛错
    if let Some(ns) = env.as_namespace(&t) {
        return Ok(ns.get_member(name).unwrap_or_else(TModel::nothing));
    }
    // 节点哈希角色（Java NodeModel 实现 TemplateHashModel：键 = 子元素名/@attr/@@key/
    // XPath；NodeHashModel.get 需 env —— ns_prefixes 解析）
    if let Some(nh) = &t.node_hash {
        return Ok(nh.get(env, name)?.unwrap_or_else(TModel::nothing));
    }
    if t.is_hash() {
        let h = t.get_hash()?;
        // 键缺失 → Java SimpleHash.get 返回 null 不抛（Dot._eval 仅 target null 时抛）
        return Ok(h.get(name)?.unwrap_or_else(TModel::nothing));
    }
    // Java NonHashException（blamed = target 表达式；位置 = target 起始）：
    // `For "." left-hand operand: Expected a hash, but this has evaluated to a {type}:`
    // `==> {target}`
    Err(
        TemplateError::type_mismatch("hash", t.type_name).with_blame_at(
            ".",
            "left-hand operand",
            &crate::core::environment::expr_desc(target),
            &env.current_template_name,
            target.span,
        ),
    )
}

/// 收集 `?builtin.a.b` 点链（Java 同样生成 Dot(BuiltIn) 嵌套——格式化器哈希访问；
/// 仅 `?string`/`?date`/`?time`/`?datetime` 的格式化器支持点参数）
fn dot_builtin_chain(e: &Expr) -> Option<(Box<Expr>, String, Vec<String>)> {
    match &e.kind {
        ExprKind::BuiltIn {
            target,
            name,
            args: None,
        } if matches!(name.as_str(), "string" | "date" | "time" | "datetime") => {
            Some((target.clone(), name.clone(), Vec::new()))
        }
        ExprKind::Dot { target, name } => {
            let (inner, bname, mut names) = dot_builtin_chain(target)?;
            names.push(name.clone());
            Some((inner, bname, names))
        }
        _ => None,
    }
}

/// 动态键访问（Java DynamicKeyName.java:69-93 `_eval`：数字键 → 序列/字符串索引；
/// 字符串键 → 哈希；v1 不支持范围键）
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

/// 范围键切片 —— Java DynamicKeyName.dealWithRangeKey（DynamicKeyName.java:183-334）：
/// 空有界范围 → 空结果（首下标可越界）；负起始 → 报错；起始越界按 adaptive 区分
/// （自适应递增可 == 目标长度，其余 >= 即错）；结果长度 = 范围长度但越界部分被
/// 自适应裁剪（`..*` 系）；无界 → 目标长度 - 起始；字符串降序且结果 > 1 → 报错
/// （"Decreasing ranges aren't allowed for slicing strings"）。
fn slice_with_range(
    t: &TModel,
    r: &crate::template::RangeSpec,
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

/// 空切片结果（Java emptyResult：序列 → 空序列；字符串 → 空字符串）
fn empty_slice_result(t: &TModel) -> TModel {
    if t.is_scalar() {
        TModel::from_scalar(String::new())
    } else {
        TModel::from_sequence(vec![])
    }
}

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

/// 方法/函数调用（Java MethodCall.java:54-77 `_eval`：
/// 宏角色 → invokeFunction（输出丢弃）；方法角色 → exec(args)）
fn eval_call(env: &mut crate::core::Environment, callee: &Expr, args: &[Expr]) -> Result<TModel> {
    let c = eval(env, callee)?;
    if let Some(mv) = env.as_macro(&c) {
        // Java MethodCall :68-71：instanceof Macro → invokeFunction
        if !mv.def.is_function {
            return Err(TemplateError::misc(
                "A macro cannot be called in an expression. (Functions can be.)",
            ));
        }
        let args: Vec<(String, Expr)> = args.iter().map(|e| (String::new(), e.clone())).collect();
        return env.invoke_function(&mv, &args);
    }
    if let Some(m) = &c.method {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            // Java：标识符求值不抛错（Environment.getVariable 返回 null），缺失
            // 参数以 null 传入方法（`m.bar(null, 11)` 的 null 即缺失变量——
            // jar 实测合法）；本引擎解析层抛 Err → 此处按 Java 语义转为 nothing
            match eval(env, a) {
                Ok(v) => vals.push(v),
                Err(TemplateError::InvalidReference { .. }) => vals.push(TModel::nothing()),
                Err(e) => return Err(e),
            }
        }
        // Java :60-66：TemplateMethodModelEx.exec(arguments.getModelList(env))；
        // Java 结果经 ObjectWrapper.wrap，Rust 侧方法直接返回 TModel
        return m.exec(vals);
    }
    Err(TemplateError::misc(format!(
        "The value of {} is not a method or function (it's a {})",
        crate::core::environment::expr_desc(callee),
        c.type_name
    )))
}

/// 加法/字符串拼接（Java AddConcatExpression.java:63-134 `_eval`）：
/// 数字+数字 → BigDecimalEngine.add；序列+序列 → 拼接序列；哈希+哈希 → 拼接哈希；
/// 其余 → 字符串拼接（数字 canonical、布尔 boolean_format、标量原样）
fn eval_add(env: &mut crate::core::Environment, a: &Expr, b: &Expr) -> Result<TModel> {
    let l = eval(env, a)?;
    let r = eval(env, b)?;
    if l.is_number() && r.is_number() {
        let engine = BigDecimalEngine::default();
        return Ok(TModel::from_number(
            engine.add(&l.get_number()?, &r.get_number()?)?,
        ));
    }
    if l.is_sequence() && r.is_sequence() {
        // Java ConcatenatedSequence
        let ls = l.get_sequence()?;
        let rs = r.get_sequence()?;
        let n = ls.size()? + rs.size()?;
        let mut v = Vec::with_capacity(n);
        for i in 0..ls.size()? {
            v.push(ls.get(i)?);
        }
        for i in 0..rs.size()? {
            v.push(rs.get(i)?);
        }
        return Ok(TModel::from_sequence(v));
    }
    if l.is_hash() && r.is_hash() {
        // Java ConcatenatedHashEx（AddConcatExpression.java:462-550）：
        // - get(key)：先右后左（右键覆盖左键）；
        // - keys()：先左后右，重复键保留首次出现位置（LinkedHashSet 语义）；
        // - values()：按 keys 顺序取 get 值
        let lh = l.get_hash()?;
        let rh = r.get_hash()?;
        let mut map = IndexMap::new();
        if let Some(ex) = &l.hash_ex {
            for key in ex.keys()? {
                if let Some(v) = lh.get(&key)? {
                    map.entry(key).or_insert(v);
                }
            }
        }
        if let Some(ex) = &r.hash_ex {
            for key in ex.keys()? {
                if let Some(v) = rh.get(&key)? {
                    map.entry(key).or_insert(v);
                }
            }
        }
        // 值取右侧优先（Java ConcatenatedHash.get :470-475）
        for key in map.keys().cloned().collect::<Vec<_>>() {
            if let Some(v) = rh.get(&key)? {
                map.insert(key, v);
            }
        }
        return Ok(TModel::from_hash(map));
    }
    // 字符串拼接（Java EvalUtil.coerceModelToStringOrMarkup）
    let ls = model_to_string(env, &l)?;
    let rs = model_to_string(env, &r)?;
    Ok(TModel::from_scalar(ls + &rs))
}

/// 数值运算（Java ArithmeticExpression.java:48-57 `_eval` → ArithmeticEngine）
enum NumOp {
    Sub,
    Mul,
    Div,
    Mod,
}

impl NumOp {
    fn symbol(&self) -> &'static str {
        match self {
            NumOp::Sub => "-",
            NumOp::Mul => "*",
            NumOp::Div => "/",
            NumOp::Mod => "%",
        }
    }
}

fn eval_binary_number(
    env: &mut crate::core::Environment,
    a: &Expr,
    b: &Expr,
    op: NumOp,
) -> Result<TModel> {
    // Java ArithmeticExpression._eval（:50-51）：lho.evalToNumber → rho.evalToNumber；
    // 操作数 null → Expression.modelToNumber（:154-160）→ NonNumericalException(blamed, null)
    // → UnexpectedTypeException 对 null 模型输出 "The following has evaluated to null or missing"
    let l = eval(env, a)?;
    if l.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(a),
        ));
    }
    let r = eval(env, b)?;
    if r.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(b),
        ));
    }
    // Java ArithmeticExpression._eval：操作数类型失败时 blame 对应操作数——
    // `For "-" left-hand operand: Expected a number, ... ==> lho` /
    // `For "-" right-hand operand: ... ==> rho`（位置 = 操作数表达式起始）
    let l = l
        .get_number()
        .map_err(|e| blame_number_operand(e, env, op.symbol(), "left-hand operand", a))?;
    let r = r
        .get_number()
        .map_err(|e| blame_number_operand(e, env, op.symbol(), "right-hand operand", b))?;
    let engine = BigDecimalEngine::default();
    let out = match op {
        NumOp::Sub => engine.sub(&l, &r)?,
        NumOp::Mul => engine.mul(&l, &r)?,
        NumOp::Div => engine.div(&l, &r)?,
        NumOp::Mod => engine.mod_op(&l, &r)?,
    };
    Ok(TModel::from_number(out))
}

/// 数字操作数类型错误 → Java `For "{op}" {side}: ... ==> {expr}` 形式
/// （NonNumericalException 的 blamer/blame 表达式/位置）
fn blame_number_operand(
    e: TemplateError,
    env: &crate::core::Environment,
    op: &str,
    side: &str,
    blamed: &Expr,
) -> TemplateError {
    match e {
        TemplateError::TypeMismatch {
            expected,
            actual,
            ctx,
        } => TemplateError::TypeMismatch {
            expected,
            actual,
            ctx: Box::new(crate::error::ErrorCtx {
                blamer: Some(format!("For \"{op}\" {side}: ")),
                blamed_expr: Some(crate::core::environment::expr_desc(blamed)),
                span: blamed.span,
                template_name: Some(env.current_template_name.clone()),
                ..*ctx
            }),
        },
        other => other,
    }
}

/// 比较运算（Java ComparisonExpression.java:92-97 → EvalUtil.compare :183-317）
/// pub(crate)：`<#switch>` case 比较复用（Java SwitchBlock :66-71 同源）
#[derive(Clone, Copy)]
pub enum CmpOp {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
}

fn eval_compare(
    env: &mut crate::core::Environment,
    a: &Expr,
    b: &Expr,
    op: CmpOp,
) -> Result<TModel> {
    let l = eval(env, a)?;
    let r = eval(env, b)?;
    let ord = compare_models(env, &l, &r, op)?;
    Ok(TModel::from_boolean(ord))
}

/// 模型比较 —— 对照 Java `EvalUtil.compare`（EvalUtil.java:183-317）：
/// - 数字 vs 数字：按 BigDecimal 数值比较（Java ArithmeticEngine.compareNumbers）；
/// - 日期 vs 日期：类型必须一致（:239-250 报错），按时间戳比较；
/// - 字符串 vs 字符串：只允许 == 和 !=（:261-267 报错），按 NFKC 归一化后 compareTo
///   （v1 用 UTF-16 码元字典序近似，注释见下）；
/// - 布尔 vs 布尔：只允许 == 和 !=（:269-275 报错）；
/// - 跨类型：报 "Can't compare values of these types..."（:307-326，classic 模式除外——v1 不支持）。
///   供 exec.rs 的 `<#switch>` case 比较复用。
pub fn compare_models(
    _env: &mut crate::core::Environment,
    l: &TModel,
    r: &TModel,
    op: CmpOp,
) -> Result<bool> {
    let order = if l.is_number() && r.is_number() {
        // Java ArithmeticEngine.compareNumbers（:295-360）：先按符号判定（无穷可比较），
        // 同类型直接 compareTo，其余转 BigDecimal
        compare_numbers(&l.get_number()?, &r.get_number()?)
    } else if l.is_date() && r.is_date() {
        let ld = l.get_date()?;
        let rd = r.get_date()?;
        if ld.kind != rd.kind {
            // Java :240-250：Can't compare dates of different types.
            return Err(TemplateError::misc(format!(
                "Can't compare dates of different types. Left date type is {}, right date type is {}.",
                ld.kind.name(),
                rd.kind.name()
            )));
        }
        ld.dt.cmp(&rd.dt)
    } else if l.is_scalar() && r.is_scalar() {
        if !matches!(op, CmpOp::Eq | CmpOp::NotEq) {
            // Java :262-266：Can't use operator ">" on string values.
            return Err(TemplateError::misc(format!(
                "Can't use operator \"{}\" on string values.",
                cmp_op_str(op)
            )));
        }
        // Java 2.3.34（IcI >= 2.3.33）：NFKC 归一化后 compareTo（:282-286）。
        // v1 近似：UTF-16 码元字典序（encode_utf16 逐码元比较；NFKC 归一化属 P4）。
        let ls = l.get_scalar()?;
        let rs = r.get_scalar()?;
        // Java 2.3.34（IcI >= 2.3.33）：Normalizer.normalize(NFKC) 后 compareTo
        // （EvalUtil.java:282-286）——`'á' == 'a\u0301'` 规范化后相等
        let ln: String = ls.chars().nfkc().collect();
        let rn: String = rs.chars().nfkc().collect();
        utf16_cmp(&ln, &rn)
    } else if l.is_boolean() && r.is_boolean() {
        if !matches!(op, CmpOp::Eq | CmpOp::NotEq) {
            return Err(TemplateError::misc(format!(
                "Can't use operator \"{}\" on boolean values.",
                cmp_op_str(op)
            )));
        }
        let lb = l.get_boolean()?;
        let rb = r.get_boolean()?;
        lb.cmp(&rb)
    } else {
        // Java :307-326：Can't compare values of these types.
        return Err(TemplateError::misc(
            "Can't compare values of these types. Allowed comparisons are between two numbers, two strings, two dates, or two booleans.",
        ));
    };
    Ok(match op {
        CmpOp::Eq => order == Ordering::Equal,
        CmpOp::NotEq => order != Ordering::Equal,
        CmpOp::Gt => order == Ordering::Greater,
        CmpOp::Gte => order != Ordering::Less,
        CmpOp::Lt => order == Ordering::Less,
        CmpOp::Lte => order != Ordering::Greater,
    })
}

/// 数字比较（Java ArithmeticEngine.compareNumbers 的 v1 复刻：符号优先，
/// 避免无穷/NaN 转 BigDecimal 失败——Java 注释 "Infinity > 0" 不会失败）
pub(crate) fn compare_numbers(a: &crate::value::TNumber, b: &crate::value::TNumber) -> Ordering {
    use crate::value::TNumber as N;
    let sa = number_signum(a);
    let sb = number_signum(b);
    if sa != sb {
        return sa.cmp(&sb);
    }
    if sa == 0 && sb == 0 {
        return Ordering::Equal;
    }
    match (a, b) {
        (N::Float(x), N::Float(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (N::Double(x), N::Double(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (N::Int(x), N::Int(y)) => x.cmp(y),
        (N::Long(x), N::Long(y)) => x.cmp(y),
        (N::BigInt(x), N::BigInt(y)) => x.cmp(y),
        (N::Float(x), N::Double(y)) => (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal),
        (N::Double(x), N::Float(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal),
        _ => a.as_big_decimal().cmp(&b.as_big_decimal()),
    }
}

/// 数值符号（-1/0/1；Java NumberUtil.getSignum）
fn number_signum(n: &crate::value::TNumber) -> i32 {
    use crate::value::TNumber as N;
    match n {
        N::Int(v) => v.signum(),
        N::Long(v) => v.signum() as i32,
        N::BigInt(v) => match v.sign() {
            num_bigint::Sign::Minus => -1,
            num_bigint::Sign::NoSign => 0,
            num_bigint::Sign::Plus => 1,
        },
        N::Decimal(d) => match d.sign() {
            num_bigint::Sign::Minus => -1,
            num_bigint::Sign::NoSign => 0,
            num_bigint::Sign::Plus => 1,
        },
        N::Float(v) => {
            if *v > 0.0 {
                1
            } else if *v < 0.0 {
                -1
            } else {
                0
            }
        }
        N::Double(v) => {
            if *v > 0.0 {
                1
            } else if *v < 0.0 {
                -1
            } else {
                0
            }
        }
    }
}

fn cmp_op_str(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::NotEq => "!=",
        CmpOp::Gt => ">",
        CmpOp::Gte => ">=",
        CmpOp::Lt => "<",
        CmpOp::Lte => "<=",
    }
}

/// UTF-16 码元字典序（近似 Java String.compareTo 的 UTF-16 char 比较；
/// 常见 BMP 文本与 Rust str 字节序一致，非 BMP 字符差异属 P4 对齐项）
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

/// 范围（Java Range.java:52-63 `_eval` → BoundedRangeModel / ListableRightUnboundedRangeModel）
/// - `a..b` 含端；`a..<b` 排端；`a..*n` 从 a 起 n 个（Java END_SIZE_LIMITED：begin+rho 为末端）；
///   有界范围实现为惰性 BoundedRangeSeq（Java BoundedRangeModel 是 TemplateSequenceModel）；
/// - `a..*` 无界 → 惰性 RightUnboundedRange（v1 集合角色 + 迭代上限）。
fn eval_range(
    env: &mut crate::core::Environment,
    start: &Expr,
    end: &Option<Box<Expr>>,
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
            m.range = Some(std::rc::Rc::new(crate::template::RangeSpec {
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
                nonlistable_right_unbounded_range_model(s_i)
            };
            m.range = Some(std::rc::Rc::new(crate::template::RangeSpec {
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

/// 有界范围序列 —— 对应 Java `BoundedRangeModel`（TemplateSequenceModel：get(i) = begin ± i，
/// size 惰性计算；不急切物化，避免超大范围内存爆炸）
pub(crate) struct BoundedRangeSeq {
    start: i64,
    count: usize,
    ascending: bool,
}

fn bounded_range_model(start: i64, count: usize, ascending: bool) -> TModel {
    let seq = Rc::new(BoundedRangeSeq {
        start,
        count,
        ascending,
    });
    TModel {
        sequence: Some(seq.clone()),
        collection: Some(seq),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

impl TemplateSequenceModel for BoundedRangeSeq {
    fn get(&self, index: usize) -> Result<TModel> {
        if index >= self.count {
            return Err(TemplateError::misc(format!(
                "Sequence index out of bounds: {index}"
            )));
        }
        let v = if self.ascending {
            self.start + index as i64
        } else {
            self.start - index as i64
        };
        Ok(TModel::from_number(TNumber::from_i64(v)))
    }
    fn size(&self) -> Result<usize> {
        Ok(self.count)
    }
}

impl TemplateCollectionModel for BoundedRangeSeq {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        let (start, count, ascending) = (self.start, self.count, self.ascending);
        Ok(Box::new((0..count).map(move |i| {
            let v = if ascending {
                start + i as i64
            } else {
                start - i as i64
            };
            Ok(TModel::from_number(TNumber::from_i64(v)))
        })))
    }
}

/// 无界范围迭代上限（防呆；Java 为真正的无限惰性序列）
const UNBOUNDED_RANGE_ITER_CAP: usize = 1_000_000;

/// 右无界范围序列长度 —— Java `ListableRightUnboundedRangeModel.size()`
/// 返回 `Integer.MAX_VALUE`（2147483647）
const UNBOUNDED_RANGE_SIZE: usize = i32::MAX as usize;

/// 右无界范围（ICI ≥ 2.3.21）—— 对应 Java `ListableRightUnboundedRangeModel`：
/// 序列 + 集合双角色；`?size` = Integer.MAX_VALUE；`r[i]` = begin + i（越界抛
/// "Range item index ... is out of bounds."）；迭代器带上限防呆
pub(crate) struct ListableRightUnboundedRange {
    start: i64,
}

fn listable_right_unbounded_range_model(start: i64) -> TModel {
    let inner = Rc::new(ListableRightUnboundedRange { start });
    let seq: Rc<dyn TemplateSequenceModel> = inner.clone();
    let coll: Rc<dyn TemplateCollectionModel> = inner;
    TModel {
        sequence: Some(seq),
        collection: Some(coll),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

impl TemplateSequenceModel for ListableRightUnboundedRange {
    fn get(&self, index: usize) -> Result<TModel> {
        if index >= UNBOUNDED_RANGE_SIZE {
            return Err(TemplateError::misc(format!(
                "Range item index {index} is out of bounds."
            )));
        }
        Ok(TModel::from_number(TNumber::from_i64(
            self.start + index as i64,
        )))
    }
    fn size(&self) -> Result<usize> {
        Ok(UNBOUNDED_RANGE_SIZE)
    }
}

impl TemplateCollectionModel for ListableRightUnboundedRange {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        let start = self.start;
        Ok(Box::new((0..UNBOUNDED_RANGE_ITER_CAP).map(move |i| {
            Ok(TModel::from_number(TNumber::from_i64(start + i as i64)))
        })))
    }
}

/// 右无界范围（ICI < 2.3.21）—— 对应 Java `NonListableRightUnboundedRangeModel`：
/// 旧版兼容：size() = 0、迭代为空（`(4..)?size` == 0、`<#list 4.. as i>` 不执行、
/// `(4..)[0]` 越界 → 数字键路径按 invalid reference 报错）
pub(crate) struct NonListableRightUnboundedRange;

fn nonlistable_right_unbounded_range_model(_start: i64) -> TModel {
    // Java NonListable 同样持有 begin（构造函数），但 size=0 时无从可见
    let seq: Rc<dyn TemplateSequenceModel> = Rc::new(NonListableRightUnboundedRange);
    let coll: Rc<dyn TemplateCollectionModel> = Rc::new(NonListableRightUnboundedRange);
    TModel {
        sequence: Some(seq),
        collection: Some(coll),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

impl TemplateSequenceModel for NonListableRightUnboundedRange {
    fn get(&self, _index: usize) -> Result<TModel> {
        Err(TemplateError::misc("Range item index is out of bounds."))
    }
    fn size(&self) -> Result<usize> {
        Ok(0)
    }
}

impl TemplateCollectionModel for NonListableRightUnboundedRange {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        Ok(Box::new(std::iter::empty()))
    }
}

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

fn eval_default_to(
    env: &mut crate::core::Environment,
    target: &Expr,
    default: &Option<Box<Expr>>,
) -> Result<TModel> {
    // Java DefaultToExpression._eval：目标 null/缺失 → 默认值；无默认值 → 空串模型
    let m = eval_lenient(env, target)?;
    if !m.is_nothing() {
        return Ok(m);
    }
    match default {
        Some(d) => eval(env, d),
        None => Ok(TModel::from_scalar(String::new())),
    }
}

/// 存在性（Java ExistsExpression：求值成功且非 null → TRUE）
fn eval_exists(env: &mut crate::core::Environment, t: &Expr) -> Result<TModel> {
    let m = eval_lenient(env, t)?;
    Ok(TModel::from_boolean(!m.is_nothing()))
}

/// 哈希字面量（Java HashLiteral：键求值为标量）
fn eval_hash_lit(env: &mut crate::core::Environment, pairs: &[(Expr, Expr)]) -> Result<TModel> {
    // Java HashLiteral → SimpleHash(LinkedHashMap)：插入序即键序
    let mut map = IndexMap::new();
    for (k, v) in pairs {
        // Java HashLiteral：键按 EvalUtil 强制转字符串（数字 "123"、布尔按 boolean_format）
        let km = eval(env, k)?;
        let key = model_to_string(env, &km)?;
        let value = eval(env, v)?;
        map.insert(key, value);
    }
    Ok(TModel::from_hash(map))
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

// ---------------------------------------------------------------------------
// 内建函数（Java BuiltInsFor*.java；docs/05 迁移清单）
// ---------------------------------------------------------------------------

/// 内建函数参数表达式视图（惰性内建按需求值）
struct BuiltinArgs<'a> {
    exprs: Option<&'a [Expr]>,
}

/// 内建函数求值（Java `BuiltIn.calculateResult(Environment)`）。
/// 分派顺序：① `crate::builtins::lookup` 注册表（内建函数智能体填表；?replace/?split/
/// ?matches/?string/?c 等 flags/模式类内建已迁入）→ ② 本文件的内建集 →
/// ③ 未命中 `Unknown built-in: ?{name}`（Java 消息）。
fn eval_builtin(
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
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
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
        // TemplateSequenceModel 但不可 #list）
        "is_sequence" => is_type_test(env, target, |m| m.is_sequence() && !m.is_method()),
        "is_collection" => is_type_test(env, target, |m| m.is_collection()),
        // Java is_enumerableBI（:319-327）：序列/集合且（ICI < 2.3.21 或非方法模型）
        "is_enumerable" => is_type_test(env, target, |m| {
            (m.is_sequence() || m.is_collection()) && !m.is_method()
        }),
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
        "has_api" => is_type_test(env, target, |_| false),
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
        "html" => str_builtin(env, target, crate::utility::html_escape),
        "xml" => str_builtin(env, target, crate::utility::xml_escape),
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
            crate::builtins::eval_util::check_arg_count("seq_contains", args.exprs, 1, 1)?;
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
        "keys" => {
            let m = eval(env, target)?;
            let h = m.hash_ex.clone().ok_or_else(|| {
                TemplateError::misc(format!(
                    "?keys is not applicable to a {} value",
                    m.type_name
                ))
            })?;
            // Java BuiltInsForHashes：SimpleHash(LinkedHashMap) 插入序
            Ok(Some(TModel::from_sequence(
                h.keys()?.into_iter().map(TModel::from_scalar).collect(),
            )))
        }
        "values" => {
            let m = eval(env, target)?;
            let h = m.hash_ex.clone().ok_or_else(|| {
                TemplateError::misc(format!(
                    "?values is not applicable to a {} value",
                    m.type_name
                ))
            })?;
            // Java BuiltInsForHashes：按插入序取值
            let mut v = Vec::new();
            for key in h.keys()? {
                v.push(h.get(&key)?.unwrap_or_else(TModel::nothing));
            }
            Ok(Some(TModel::from_sequence(v)))
        }
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
            // v1 延迟到方法调用时（等价；类名解析错误消息对齐 Java）
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

/// 从字符下标切出子串（Java String.substring 语义近似；下标为 char 计数）
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
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
        fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
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
