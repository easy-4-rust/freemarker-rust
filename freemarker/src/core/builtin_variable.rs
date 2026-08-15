//! 内置变量 —— 对应 Java `freemarker.core.BuiltinVariable`
//! （`_eval` :186-300；SPEC_VAR_NAMES 清单见 expression.rs BuiltinVar 注释）

use crate::core::BuiltinVar;
use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateHashModel, TemplateHashModelEx};
use crate::value::{DateType, DateValue};
use std::rc::Rc;

/// 内置变量表达式（对应 BuiltinVariable.java；解析器经 `ExprKind::BuiltinVar` 承载）
pub struct BuiltinVariable {
    pub name: BuiltinVar,
}

impl BuiltinVariable {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(name: BuiltinVar) -> Self {
        BuiltinVariable { name }
    }

    /// 求值（Java `_eval`）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_builtin_var(env, self.name)
    }
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
        // Java :264-267：CALLER_TEMPLATE_NAME(_CC) → 当前宏/函数**调用方**模板名
        // （getRequiredMacroContext(env).callPlace.getTemplate().getName()：
        // 调用点所在模板的查找名；无名模板 getName()==null → EMPTY_STRING ""）
        BuiltinVar::CallerTemplateName | BuiltinVar::CallerTemplateNameCc => {
            match env.get_current_macro_frame() {
                Some(frame) => Ok(TModel::from_scalar(
                    frame
                        .caller_template_name
                        .as_deref()
                        .unwrap_or("")
                        .to_string(),
                )),
                None => {
                    // Java getRequiredMacroContext（BuiltinVariable.java:285-293）：
                    // "Can't get .{字面名} here, as there's no macro or function ..."
                    let literal = if matches!(v, BuiltinVar::CallerTemplateNameCc) {
                        "callerTemplateName"
                    } else {
                        "caller_template_name"
                    };
                    Err(TemplateError::misc(format!(
                        "Can't get .{literal} here, as there's no macro or function (that's implemented in the template) call in context."
                    )))
                }
            }
        }
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
        // Java BuiltinVariable.java:258-262：GET_OPTIONAL_TEMPLATE(_CC) →
        // GetOptionalTemplateMethod 方法模型（无状态；exec 时经 env 查找模板；
        // 两个变体仅错误消息中的方法名不同）
        BuiltinVar::GetOptionalTemplate => Ok(TModel::from_method(
            crate::core::get_optional_template_method::GetOptionalTemplateMethod::snake_case(),
        )),
        BuiltinVar::GetOptionalTemplateCc => Ok(TModel::from_method(
            crate::core::get_optional_template_method::GetOptionalTemplateMethod::camel_case(),
        )),
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
