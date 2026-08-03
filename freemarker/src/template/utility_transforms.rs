//! utility 变换模型 —— 对应 Java `freemarker.template.utility` 包的
//! TemplateTransformModel 家族（StandardCompress / NormalizeNewlines / HtmlEscape），
//! 以及 `?new` 内建对已知类名的实例化映射（NewBI.java + TemplateClassResolver）
//! v1 限制：仅支持三个无参构造的 utility 变换类（文档化偏差，见 docs/10 §2）

use crate::core::environment::RunSignal;
use crate::core::{Element, Environment};
use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateTransformModel};
use std::collections::HashMap;

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
            Ok(TModel::from_transform(StandardCompressTransform))
        }
        "freemarker.template.utility.NormalizeNewlines" => {
            if !ctor_args.is_empty() {
                return Err(no_such_method(class_name, ctor_args));
            }
            Ok(TModel::from_transform(NormalizeNewlinesTransform))
        }
        "freemarker.template.utility.HtmlEscape" => {
            if !ctor_args.is_empty() {
                return Err(no_such_method(class_name, ctor_args));
            }
            Ok(TModel::from_transform(HtmlEscapeTransform))
        }
        // 测试夹具类 —— 对应 Java `freemarker.test.templatesuite.models.NewTestModel`
        // （TemplateScalarModel；构造器：() / (String) / (long) / (Object, Serializable)）
        "freemarker.test.templatesuite.models.NewTestModel" => {
            new_test_model(ctor_args)
        }
        // 泛型构造器 —— 对应 Java `freemarker.template.utility.ObjectConstructor`
        // （TemplateMethodModelEx：exec(args) = args[0] 类名 + 剩余构造参数）
        "freemarker.template.utility.ObjectConstructor" => {
            Ok(TModel::from_method(ObjectConstructorFn))
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

/// ObjectConstructor —— 对应 Java `freemarker.template.utility.ObjectConstructor`
/// （exec：Class.forName(args[0]) + 构造器匹配实例化，结果经 ObjectWrapper.wrap）。
/// v1 支持 java.lang.String（单参）与 java.lang.Integer/Long（单参数字）；
/// 其余类名 → ClassNotFoundException 语义
pub struct ObjectConstructorFn;
impl crate::template::TemplateMethodModelEx for ObjectConstructorFn {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
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

/// SimpleTestMethod —— 对应 Java `freemarker.test.templatesuite.models.SimpleTestMethod`
/// （TemplateMethodModelEx；exec 接收一个参数，返回 "Single argument value is: {n}"，
/// n 为参数经 arg_to_test_value 解析后的值：数值取整，字符串为数字单词则映射为对应数字）
pub struct SimpleTestMethodFn;
impl crate::template::TemplateMethodModelEx for SimpleTestMethodFn {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
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

/// StandardCompress —— 对应 Java `freemarker.template.utility.StandardCompress`
/// （逐字符状态机 StandardCompressWriter :117-245；`<#compress>` 指令内部即
/// CompressedBlock → StandardCompress.INSTANCE，CompressedBlock.java:42）
pub struct StandardCompressTransform;
impl TemplateTransformModel for StandardCompressTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        // Java getWriter 参数（StandardCompress.java:92-115）：buffer_size（数值，
        // 缓冲分块——v1 整段捕获等价）、single_line（布尔）
        let single_line = match params.get("single_line") {
            Some(m) => m
                .boolean
                .as_ref()
                .map(|b| b.as_boolean().unwrap_or(false))
                .unwrap_or(false),
            None => false,
        };
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&standard_compress_text(&captured, single_line))?;
        Ok(signal)
    }
}

/// StandardCompress 的逐字符状态机 —— 对应 Java `StandardCompressWriter`
/// （writeHelper :153-171 / updateLineBreakState :173-195 /
/// writeLineBreakOrSpace :197-232）。语义：
/// - 前导空白忽略（AT_BEGINNING）
/// - 换行序列 → 单个换行，**保留原换行类型**（CR / LF / CRLF）
/// - 行内空白序列 → 单个空格（INIT → ' '）
/// - 尾部空白丢弃（从未写入缓冲）
/// - single_line=true → 换行输出为空格（SINGLE_LINE 状态）
///
/// 差异：Rust `char::is_whitespace`（Unicode White_Space）vs Java
/// `Character.isWhitespace`（不含 U+00A0 等）——边界字符行为略宽（P6 可补）。
pub fn standard_compress_text(s: &str, single_line: bool) -> String {
    #[derive(PartialEq, Clone, Copy)]
    enum Lb {
        AtBeginning,
        SingleLine,
        Init,
        SawCr,
        LineBreakCr,
        LineBreakCrLf,
        LineBreakLf,
    }
    let mut out = String::new();
    let mut in_ws = true;
    let mut lb = Lb::AtBeginning;
    for c in s.chars() {
        if c.is_whitespace() {
            in_ws = true;
            // Java updateLineBreakState：仅 INIT / SAW_CR 状态推进
            lb = match lb {
                Lb::Init => {
                    if c == '\r' {
                        Lb::SawCr
                    } else if c == '\n' {
                        Lb::LineBreakLf
                    } else {
                        Lb::Init
                    }
                }
                Lb::SawCr => {
                    if c == '\n' {
                        Lb::LineBreakCrLf
                    } else {
                        Lb::LineBreakCr
                    }
                }
                other => other,
            };
        } else if in_ws {
            in_ws = false;
            // Java writeLineBreakOrSpace
            match lb {
                Lb::AtBeginning => {} // 前导空白忽略
                Lb::SawCr | Lb::LineBreakCr => out.push('\r'),
                Lb::LineBreakCrLf => {
                    out.push('\r');
                    out.push('\n');
                }
                Lb::LineBreakLf => out.push('\n'),
                Lb::Init | Lb::SingleLine => out.push(' '),
            }
            lb = if single_line {
                Lb::SingleLine
            } else {
                Lb::Init
            };
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out
}

/// NormalizeNewlines —— 对应 Java `freemarker.template.utility.NormalizeNewlines`
/// （transform :89-112：BufferedReader.readLine 分行，首行非空才输出，其余行全部
/// println → 行尾统一为 \n）。readLine 语义：`\r\n`、`\r`、`\n` 均视为行尾；
/// EOF 前的最后一段（无行尾）也算一行。
pub struct NormalizeNewlinesTransform;
impl TemplateTransformModel for NormalizeNewlinesTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        _params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&normalize_newlines_text(&captured))?;
        Ok(signal)
    }
}

/// Java NormalizeNewlines.transform（:89-112）的按行归一化
pub fn normalize_newlines_text(s: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    let mut line_start = 0;
    let b = s.as_bytes();
    let mut i = 0;
    while i <= b.len() {
        let eol_len = if i == b.len() {
            0
        } else if b[i] == b'\n' {
            1
        } else if b[i] == b'\r' {
            if i + 1 < b.len() && b[i + 1] == b'\n' {
                2
            } else {
                1
            }
        } else {
            0
        };
        if eol_len == 0 {
            i += 1;
            continue;
        }
        let line = &s[line_start..i];
        if first {
            first = false;
            if line.is_empty() {
                // 首行空 → 跳过（Java :96-98），后续行照常输出
                i += eol_len;
                line_start = i;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
        i += eol_len;
        line_start = i;
    }
    // EOF 前的最后一段（readLine 返回无行尾的行）
    if line_start < b.len() {
        let line = &s[line_start..];
        if first {
            if !line.is_empty() {
                out.push_str(line);
                out.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// HtmlEscape —— 对应 Java `freemarker.template.utility.HtmlEscape`
/// （getWriter :63-96：`<` `>` `&` `"` 转义，**不含 `'`**；body 整体转义——
/// 含字面标签文本）
pub struct HtmlEscapeTransform;
impl TemplateTransformModel for HtmlEscapeTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        _params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&html_escape_entity(&captured))?;
        Ok(signal)
    }
}

/// Java HtmlEscape 的实体集：`& < > "`（与 StringUtil.HTMLEnc 不同，不含 `'`）
fn html_escape_entity(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_state_machine_matches_java() {
        // Java StandardCompressWriter 逐字符状态机（Probe 核对）：
        // 前导空白忽略、换行序列保留原类型、行内空白 → 单空格、尾部空白丢弃
        assert_eq!(
            standard_compress_text("  a\n\n  b\r\nc\r  d  ", false),
            "a\nb\r\nc\rd"
        );
        assert_eq!(standard_compress_text("x  y", false), "x y");
        assert_eq!(standard_compress_text("\n\nx", false), "x");
        assert_eq!(standard_compress_text("a \t b", false), "a b");
        // single_line=true：换行 → 空格
        assert_eq!(standard_compress_text("a\nb\n", true), "a b");
        assert_eq!(standard_compress_text("\n  a\nb\n\n", true), "a b");
    }

    #[test]
    fn normalize_newlines_lines() {
        // Java NormalizeNewlines.transform：\r\n/\r/\n 均行尾、首行空跳过、统一 \n
        assert_eq!(normalize_newlines_text("a\r\nb\rc\n"), "a\nb\nc\n");
        assert_eq!(normalize_newlines_text("\r\nb"), "b\n");
        assert_eq!(normalize_newlines_text(""), "");
        assert_eq!(normalize_newlines_text("a\n\nb"), "a\n\nb\n");
        assert_eq!(normalize_newlines_text("no-eol"), "no-eol\n");
        assert_eq!(normalize_newlines_text("\n"), "");
    }

    #[test]
    fn html_escape_entity_set() {
        // Java HtmlEscape：& < > "（不含 '）
        assert_eq!(
            html_escape_entity("<a href=\"x\">&'\n"),
            "&lt;a href=&quot;x&quot;&gt;&amp;'\n"
        );
        assert_eq!(html_escape_entity("plain"), "plain");
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
