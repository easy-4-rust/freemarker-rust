//! `.get_optional_template(name[, options])` —— 对应 Java `GetOptionalTemplateMethod.java`
//! （BuiltinVariable.GET_OPTIONAL_TEMPLATE / GET_OPTIONAL_TEMPLATE_CC :79-80；
//! 求值为方法模型，调用返回 {exists/include/import} 哈希；Java 用线程局部
//! Environment，Rust 显式传 env —— exec 经 env 完成模板查找，与 Java 的无状态
//! INSTANCE/INSTANCE_CC 单例一致）

use crate::core::environment::{LookupOutcome, LookupResult};
use crate::core::Environment;
use crate::error::Result;
use crate::error::TemplateError;
use crate::template::{
    TModel, TemplateDirectiveBody, TemplateDirectiveModel, TemplateMethodModelEx,
};
use std::collections::HashMap;

/// 选项名（Java :53-54）
const OPTION_ENCODING: &str = "encoding";
const OPTION_PARSE: &str = "parse";

/// 结果键（Java :56-58）
const RESULT_EXISTS: &str = "exists";
const RESULT_INCLUDE: &str = "include";
const RESULT_IMPORT: &str = "import";

/// `.get_optional_template(name[, options])`（Java GetOptionalTemplateMethod.java：
/// INSTANCE/INSTANCE_CC 两个无状态单例——仅错误消息中的方法名不同，
/// Java :60-65 的 `"." + builtinVarName`）
pub(crate) struct GetOptionalTemplateMethod {
    method_name: &'static str,
}

impl GetOptionalTemplateMethod {
    /// `.get_optional_template`（BuiltinVariable.GET_OPTIONAL_TEMPLATE）
    pub(crate) fn snake_case() -> Self {
        Self {
            method_name: ".get_optional_template",
        }
    }

    /// `.getOptionalTemplate`（BuiltinVariable.GET_OPTIONAL_TEMPLATE_CC）
    pub(crate) fn camel_case() -> Self {
        Self {
            method_name: ".getOptionalTemplate",
        }
    }
}

impl TemplateMethodModelEx for GetOptionalTemplateMethod {
    fn exec(&self, env: &mut Environment, args: Vec<TModel>) -> Result<TModel> {
        let arg_cnt = args.len();
        // Java :70-72：newArgCntError(methodName, argCnt, 1, 2)
        if !(1..=2).contains(&arg_cnt) {
            return Err(TemplateError::misc(format!(
                "{}... expects 1 or 2 arguments but has received {}.",
                self.method_name,
                if arg_cnt == 0 {
                    "none".to_string()
                } else {
                    arg_cnt.to_string()
                }
            )));
        }

        // Java :80-93：第一个参数必须是字符串模板名；toFullTemplateName 解析为绝对名
        let name = {
            let arg = &args[0];
            if !arg.is_scalar() {
                return Err(TemplateError::misc(format!(
                    "{}... expects a string as argument #1, but received {}.",
                    self.method_name,
                    ftl_type_desc(arg)
                )));
            }
            arg.get_scalar()?
        };
        let abs_template_name = env.resolve_template_name(&name);

        // Java :95-136：可选 options 哈希（encoding/parse；未知选项报错）。
        // Java 还校验哈希键必须为字符串（:113-122）——Rust 侧 TemplateHashModelEx
        // 的键天然是 String，该检查不可能失败，省略
        let mut encoding: Option<String> = None;
        let mut parse = true;
        if arg_cnt > 1 {
            let opt = &args[1];
            if !opt.is_hash_ex() {
                return Err(TemplateError::misc(format!(
                    "{}... expects an extended hash as argument #2, but received {}.",
                    self.method_name,
                    ftl_type_desc(opt)
                )));
            }
            for (opt_name, opt_value) in opt.hash_ex.as_ref().unwrap().entries()? {
                if opt_name == OPTION_ENCODING {
                    // Java :126-127：getStringOption（值必须是字符串）
                    if !opt_value.is_scalar() {
                        return Err(invalid_option(
                            self.method_name,
                            format!(
                                "The value of the \"{OPTION_ENCODING}\" option must be a string, but it was {}.",
                                ftl_type_desc(&opt_value)
                            ),
                        ));
                    }
                    encoding = Some(opt_value.get_scalar()?);
                } else if opt_name == OPTION_PARSE {
                    // Java :128-129：getBooleanOption（值必须是布尔）
                    if !opt_value.is_boolean() {
                        return Err(invalid_option(
                            self.method_name,
                            format!(
                                "The value of the \"{OPTION_PARSE}\" option must be a boolean, but it was {}.",
                                ftl_type_desc(&opt_value)
                            ),
                        ));
                    }
                    parse = opt_value.get_boolean()?;
                } else {
                    // Java :130-133：未知选项报错
                    return Err(invalid_option(
                        self.method_name,
                        format!(
                            "Unsupported option \"{opt_name}\"; valid names are: \"{OPTION_ENCODING}\", \"{OPTION_PARSE}\"."
                        ),
                    ));
                }
            }
        }

        // Java :138-145：getTemplateForInclusion(absName, encoding, parse, true)
        // —— ignoreMissing=true，缺失不报错
        let found = env.lookup_template(&abs_template_name, parse, encoding.as_deref())?;

        let mut result = indexmap::IndexMap::new();
        match found {
            // Java :149-150：模板缺失 → 仅 exists=false（include/import 缺失，
            // 便于 `<@optTemp.include!myDefaultMacro />` 提供默认值）
            LookupOutcome::Missing(_) => {
                result.insert(RESULT_EXISTS.to_string(), TModel::from_boolean(false));
            }
            LookupOutcome::Found(acq, found_t) => {
                result.insert(RESULT_EXISTS.to_string(), TModel::from_boolean(true));
                result.insert(
                    RESULT_INCLUDE.to_string(),
                    TModel::from_directive(IncludeDirective {
                        found: (acq.clone(), found_t.clone()),
                    }),
                );
                result.insert(
                    RESULT_IMPORT.to_string(),
                    TModel::from_method(ImportMethod {
                        found: (acq, found_t),
                    }),
                );
            }
        }
        Ok(TModel::from_hash(result))
    }
}

/// 结果哈希的 `include` 指令（Java :152-168：不支持参数/循环变量/嵌套内容，
/// 否则报错；随后 env.include(template)）
struct IncludeDirective {
    found: (String, LookupResult),
}

impl TemplateDirectiveModel for IncludeDirective {
    fn execute(
        &self,
        env: &mut Environment,
        params: &HashMap<String, TModel>,
        loop_vars: &mut [TModel],
        body: Option<&dyn TemplateDirectiveBody>,
    ) -> Result<()> {
        if !params.is_empty() {
            return Err(TemplateError::misc(
                "This directive supports no parameters.",
            ));
        }
        if !loop_vars.is_empty() {
            return Err(TemplateError::misc(
                "This directive supports no loop variables.",
            ));
        }
        if body.is_some() {
            return Err(TemplateError::misc(
                "This directive supports no nested content.",
            ));
        }
        match &self.found.1 {
            LookupResult::Parsed(t) => env.include_template(t),
            LookupResult::PlainText(text) => env.emit(text),
        }
    }
}

/// 结果哈希的 `import` 方法（Java :169-182：不支持参数；importLib(template, null)
/// 返回命名空间模型）
struct ImportMethod {
    found: (String, LookupResult),
}

impl TemplateMethodModelEx for ImportMethod {
    fn exec(&self, env: &mut Environment, args: Vec<TModel>) -> Result<TModel> {
        if !args.is_empty() {
            return Err(TemplateError::misc("This method supports no parameters."));
        }
        env.import_lib_loaded(&self.found.0, &self.found.1)
    }
}

/// 选项值错误包装（Java newMethodArgInvalidValueException：
/// `{methodName}(...) argument #2 had invalid value: {details}.`）
fn invalid_option(method_name: &str, details: String) -> TemplateError {
    TemplateError::misc(format!(
        "{method_name}(...) argument #2 had invalid value: {details}"
    ))
}

/// 类型描述（与 sequences.rs 的 ftl_type_desc 同约定，但按 Java
/// `_DelayedAOrAn` 区分 a/an：extended_hash → an；null → "a Null"）
fn ftl_type_desc(m: &TModel) -> String {
    if m.is_nothing() {
        return "a Null".to_string();
    }
    let t = m.type_name;
    if t.starts_with(['a', 'e', 'i', 'o', 'u']) {
        format!("an {t}")
    } else {
        format!("a {t}")
    }
}
