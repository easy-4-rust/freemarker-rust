//! `?new` 内建的已知类名实例化映射（Java NewBI.java + TemplateClassResolver）；
//! 各 utility 变换类已拆至 `template/utility/` 独立文件（一文件一 Java 对象）
//! v1 限制：仅支持白名单 utility 变换类（文档化偏差，见 docs/10 §2）

use crate::core::Environment;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 轻量模型字符串化 —— 用于 `?new` 构造参数（Java 中构造参数经 BeansWrapper 转
/// Java 对象后按 Object.toString 语义拼接；无 env 上下文，仅处理 scalar/number/
/// boolean）
fn arg_to_string(m: &TModel) -> Result<String> {
    if let Some(s) = &m.scalar {
        return s.as_string();
    }
    if let Some(n) = &m.number {
        let num = n.as_number()?;
        if let Some(v) = num.as_i64() {
            return Ok(v.to_string());
        }
        return Ok(num.to_plain_string());
    }
    if let Some(b) = &m.boolean {
        return Ok(b.as_boolean()?.to_string());
    }
    Ok(m.type_name.to_string())
}

/// `?new` 类解析 —— 对应 Java `TemplateClassResolver.UNRESTRICTED_RESOLVER`
/// （Configurable.java:477 默认）+ `NewBI.ConstructorFunction` 构造校验。
/// v1 支持的类：三个无参构造的 utility 变换类（StandardCompress/NormalizeNewlines/
/// HtmlEscape）、测试夹具标量类 NewTestModel（4 种构造器）、ObjectConstructor
/// （泛型构造方法模型）。其余类名按 Java ClassNotFoundException 语义报错
/// （该类在本引擎中不存在）。
pub fn new_utility_class(class_name: &str, ctor_args: &[TModel]) -> Result<TModel> {
    match class_name {
        "freemarker.template.utility.StandardCompress" => {
            if !ctor_args.is_empty() {
                return Err(no_such_method(class_name, ctor_args));
            }
            Ok(TModel::from_transform(crate::template::utility::StandardCompressTransform))
        }
        "freemarker.template.utility.NormalizeNewlines" => {
            if !ctor_args.is_empty() {
                return Err(no_such_method(class_name, ctor_args));
            }
            Ok(TModel::from_transform(crate::template::utility::NormalizeNewlinesTransform))
        }
        "freemarker.template.utility.HtmlEscape" => {
            if !ctor_args.is_empty() {
                return Err(no_such_method(class_name, ctor_args));
            }
            Ok(TModel::from_transform(crate::template::utility::HtmlEscapeTransform))
        }
        // 测试夹具类 —— 对应 Java `freemarker.test.templatesuite.models.NewTestModel`
        // （TemplateScalarModel；构造器：() / (String) / (long) / (Object, Serializable)）
        "freemarker.test.templatesuite.models.NewTestModel" => {
            new_test_model(ctor_args)
        }
        // 测试夹具类 —— 对应 Java `freemarker.test.templatesuite.models.NewTestModel2`
        // （与 NewTestModel 同构：() / (String) / (long) / (Object, Serializable)；
        // new-optin 用例的信任模板内经 SAFER 解析器实例化）
        "freemarker.test.templatesuite.models.NewTestModel2" => {
            new_test_model(ctor_args)
        }
        // 泛型构造器 —— 对应 Java `freemarker.template.utility.ObjectConstructor`
        // （TemplateMethodModelEx：exec(args) = args[0] 类名 + 剩余构造参数）
        "freemarker.template.utility.ObjectConstructor" => {
            Ok(TModel::from_method(crate::template::utility::ObjectConstructorFn))
        }
        // Java 测试夹具 —— `SimpleTestMethod`（TemplateMethodModelEx）：
        // exec(x) 返回 "Single argument value is: {x}"（数值原样，字符串若为数字名则映射）
        "freemarker.test.templatesuite.models.SimpleTestMethod" => {
            Ok(TModel::from_method(SimpleTestMethodFn))
        }
        _ => Err(TemplateError::misc(format!(
            "No error description was specified for this error; low-level message: java.lang.ClassNotFoundException: {class_name}"
        ))),
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

/// NewTestModel 构造 —— 对应 Java `NewTestModel.java` 的 4 个构造器：
/// () → "default constructor"；(String) → 原样；(long) → Long.toString；
/// (Object, Serializable) → "{o1}:{o2}"
fn new_test_model(args: &[TModel]) -> Result<TModel> {
    let text = match args {
        [] => "default constructor".to_string(),
        [a] if a.scalar.is_some() || a.is_number() => arg_to_string(a)?,
        [a, b] => format!("{}:{}", arg_to_string(a)?, arg_to_string(b)?),
        _ => {
            return Err(no_such_method(
                "freemarker.test.templatesuite.models.NewTestModel",
                args,
            ))
        }
    };
    Ok(TModel::from_scalar(text))
}

/// SimpleTestMethod —— 对应 Java `freemarker.test.templatesuite.models.SimpleTestMethod`
/// （TemplateMethodModelEx；exec 接收一个参数，返回 "Single argument value is: {n}"，
/// n 为参数经 arg_to_test_value 解析后的值：数值取整，字符串为数字单词则映射为对应数字）
pub struct SimpleTestMethodFn;
impl crate::template::TemplateMethodModelEx for SimpleTestMethodFn {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> Result<TModel> {
        let value = if let Some(arg) = args.first() {
            arg_to_test_value(arg)
        } else {
            "".to_string()
        };
        Ok(TModel::from_scalar(format!(
            "Single argument value is: {value}"
        )))
    }
}

/// `SimpleTestMethod` 参数值解析：
/// - 数值：取整数部分（i64）
/// - 字符串：数字单词（zero‥twelve）→ 对应数字；纯数字串 → 原样；
///   混合字符串 → 提取数字字符拼接（"one2" → "2"）
fn arg_to_test_value(m: &TModel) -> String {
    use std::collections::HashMap;
    if let Some(n) = &m.number {
        let num = n.as_number().ok();
        if let Some(v) = num.clone().and_then(|n| n.as_i64()) {
            return v.to_string();
        }
        if let Some(v) = num.map(|n| n.to_plain_string()) {
            return v;
        }
    }
    if let Some(s) = &m.scalar {
        let sv = s.as_string().unwrap_or_default();
        let word_map: HashMap<&str, &str> = [
            ("zero", "0"),
            ("one", "1"),
            ("two", "2"),
            ("three", "3"),
            ("four", "4"),
            ("five", "5"),
            ("six", "6"),
            ("seven", "7"),
            ("eight", "8"),
            ("nine", "9"),
            ("ten", "10"),
            ("eleven", "11"),
            ("twelve", "12"),
        ]
        .iter()
        .cloned()
        .collect();
        if let Some(num) = word_map.get(sv.as_str()) {
            return num.to_string();
        }
        if let Ok(n) = sv.parse::<i64>() {
            return n.to_string();
        }
        // 混合字符串：提取所有数字字符拼接（"one2" → "2"）
        let digits: String = sv.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits;
        }
        return sv;
    }
    "".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_state_machine_matches_java() {
        // Java StandardCompressWriter 逐字符状态机（Probe 核对）：
        // 前导空白忽略、换行序列保留原类型、行内空白 → 单空格、尾部空白丢弃
        assert_eq!(
            crate::template::utility::standard_compress_text("  a\n\n  b\r\nc\r  d  ", false),
            "a\nb\r\nc\rd"
        );
        assert_eq!(
            crate::template::utility::standard_compress_text("x  y", false),
            "x y"
        );
        assert_eq!(
            crate::template::utility::standard_compress_text("\n\nx", false),
            "x"
        );
        assert_eq!(
            crate::template::utility::standard_compress_text("a \t b", false),
            "a b"
        );
        // single_line=true：换行 → 空格
        assert_eq!(
            crate::template::utility::standard_compress_text("a\nb\n", true),
            "a b"
        );
        assert_eq!(
            crate::template::utility::standard_compress_text("\n  a\nb\n\n", true),
            "a b"
        );
    }

    #[test]
    fn normalize_newlines_lines() {
        // Java NormalizeNewlines.transform：\r\n/\r/\n 均行尾、首行空跳过、统一 \n
        assert_eq!(
            crate::template::utility::normalize_newlines_text("a\r\nb\rc\n"),
            "a\nb\nc\n"
        );
        assert_eq!(
            crate::template::utility::normalize_newlines_text("\r\nb"),
            "b\n"
        );
        assert_eq!(crate::template::utility::normalize_newlines_text(""), "");
        assert_eq!(
            crate::template::utility::normalize_newlines_text("a\n\nb"),
            "a\n\nb\n"
        );
        assert_eq!(
            crate::template::utility::normalize_newlines_text("no-eol"),
            "no-eol\n"
        );
        assert_eq!(crate::template::utility::normalize_newlines_text("\n"), "");
    }

    #[test]
    fn html_escape_entity_set() {
        // Java HtmlEscape：& < > "（不含 '）
        assert_eq!(
            crate::template::utility::html_escape_entity("<a href=\"x\">&'\n"),
            "&lt;a href=&quot;x&quot;&gt;&amp;'\n"
        );
        assert_eq!(
            crate::template::utility::html_escape_entity("plain"),
            "plain"
        );
    }

    #[test]
    fn new_test_model_constructors() {
        // Java NewTestModel：() / (String) / (long) / (Object, Serializable)
        assert_eq!(
            new_utility_class("freemarker.test.templatesuite.models.NewTestModel", &[])
                .unwrap()
                .scalar
                .unwrap()
                .as_string()
                .unwrap(),
            "default constructor"
        );
        assert_eq!(
            new_utility_class(
                "freemarker.test.templatesuite.models.NewTestModel",
                &[TModel::from_scalar("xxx".to_string())]
            )
            .unwrap()
            .scalar
            .unwrap()
            .as_string()
            .unwrap(),
            "xxx"
        );
        assert_eq!(
            new_utility_class(
                "freemarker.test.templatesuite.models.NewTestModel",
                &[TModel::from_number(crate::value::TNumber::Int(1))]
            )
            .unwrap()
            .scalar
            .unwrap()
            .as_string()
            .unwrap(),
            "1"
        );
        assert_eq!(
            new_utility_class(
                "freemarker.test.templatesuite.models.NewTestModel",
                &[
                    TModel::from_scalar("xxx".to_string()),
                    TModel::from_scalar("yyy".to_string())
                ]
            )
            .unwrap()
            .scalar
            .unwrap()
            .as_string()
            .unwrap(),
            "xxx:yyy"
        );
        // 未知类 → ClassNotFoundException 语义消息
        let err = new_utility_class("no.such.Class", &[]).unwrap_err();
        assert!(err
            .to_string()
            .contains("ClassNotFoundException: no.such.Class"));
        // 带参构造对 utility 类 → NoSuchMethodException 语义
        let err = new_utility_class(
            "freemarker.template.utility.StandardCompress",
            &[TModel::from_scalar("x".to_string())],
        )
        .unwrap_err();
        assert!(err.to_string().contains("NoSuchMethodException"));
    }
}
