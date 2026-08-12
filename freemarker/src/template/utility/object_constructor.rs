//! 对象构造器 —— 对应 Java `freemarker.template.utility.ObjectConstructor`
//! （TemplateMethodModelEx：exec(args) = args[0] 类名 + 剩余构造参数实例化，
//! 结果经 ObjectWrapper.wrap；v1 支持 java.lang.String（单参）与
//! java.lang.Integer/Long（单参数字）；其余类名 → ClassNotFoundException 语义）

use crate::core::Environment;
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::template::TemplateMethodModelEx;

/// ObjectConstructor（对应 ObjectConstructor.java）
pub struct ObjectConstructorFn;

impl TemplateMethodModelEx for ObjectConstructorFn {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> Result<TModel> {
        let Some(first) = args.first() else {
            return Err(TemplateError::misc(
                "No error description was specified for this error; low-level message: java.lang.IllegalArgumentException: Object constructor needs at least 1 argument.",
            ));
        };
        let class_name = arg_to_string(first)?;
        let rest = &args[1..];
        match class_name.as_str() {
            "java.lang.String" => {
                let Some(s) = rest.first() else {
                    return Err(no_such_method(&class_name, rest));
                };
                Ok(TModel::from_scalar(arg_to_string(s)?))
            }
            "java.lang.Integer" | "java.lang.Long" => {
                let Some(n) = rest.first() else {
                    return Err(no_such_method(&class_name, rest));
                };
                if n.is_number() {
                    Ok(TModel::from_scalar(arg_to_string(n)?))
                } else {
                    Err(no_such_method(&class_name, rest))
                }
            }
            _ => Err(TemplateError::misc(format!(
                "No error description was specified for this error; low-level message: java.lang.ClassNotFoundException: {class_name}"
            ))),
        }
    }
}

/// Java NoSuchMethodException 语义（BeansWrapper.newInstance 构造器不匹配）
fn no_such_method(class_name: &str, ctor_args: &[TModel]) -> TemplateError {
    let arg_desc: Vec<&str> = ctor_args.iter().map(|m| m.type_name).collect();
    TemplateError::misc(format!(
        "No error description was specified for this error; low-level message: java.lang.NoSuchMethodException: {class_name}.<init>({}).",
        arg_desc.join(", ")
    ))
}

/// 模型 → 字符串（Java 语义：标量直取、数字 canonical、其余报错）
pub(crate) fn arg_to_string(m: &TModel) -> Result<String> {
    if let Some(s) = &m.scalar {
        return s.as_string();
    }
    if let Some(n) = &m.number {
        return n.as_number().map(|n| n.to_plain_string());
    }
    Err(TemplateError::misc(format!(
        "Expected a string or number, but this has evaluated to a {}",
        m.type_name
    )))
}
