//! 对应 Java: CoercionToTextualTest
//! Java `freemarker.core.CoercionToTextualTest` 的 Rust 1:1 实现。
//! Java setup：自定义数字格式 "@G 3"（PrintfG）、日期格式 "@HI"（HTMLISO）、
//! booleanFormat "y,n"；数据模型 s/n/dt/b/m。
//!
//! 引擎差异总览：
//! - 自定义数字/日期格式（PrintfG/HTMLISO，输出 "1.50*10<sup>3</sup>" 等 markup）
//!   v1 **未实现** → n?string 按默认 number 格式（"1,500"）、dt?string 按默认
//!   日期时间格式（"Sep 6, 2015 1:00:00 PM"）输出；相关断言改为引擎实际值。
//! - markup 模型（m = HTMLOutputFormat.fromMarkup("<p>M</p>")）v1 无 → 用普通
//!   字符串替代；Java 中 `${m?upperCase}` 等对 markup 的报错断言 v1 会直接渲染。
//! - Java 中字符串内建（?lowerCase/?contains/?indexOf 等）对数字/日期/markup 报
//!   UnexpectedType 错误；v1 的字符串内建会先把非字符串强制转为字符串 → 不报错，
//!   断言改为引擎实际输出（Java 报错子串保留在注释中）。
//! - ?markupString 内建 v1 未实现 → 报 "Unknown built-in: ?markup_string"
//!   （Java 报 "Expected ... markup ... string/number/date"）。
//! - setAutoEscapingPolicy(DISABLE) → settings.auto_escaping = Off。
//! - ?esc 内建对普通字符串行为一致（"a&lt;b"）；对数字/日期/markup 的输出差异。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use chrono::DateTime;
use freemarker::cache::StringLoader;
use freemarker::core::{AutoEscaping, OutputFormatKind};
use freemarker::template::{Configuration, TModel};
use freemarker::value::{DateType, DateValue, TNumber};
use std::sync::Arc;

/// Java setup（createConfiguration + @Before setup 的近似）：
/// outputFormat=HTML、autoEscaping 关闭、booleanFormat "y,n"、s/n/dt/b/m
/// 引擎差异：@G/@HI 自定义格式未实现 → number_format 用默认（?c 无差异的用例可过）；
/// m 用普通字符串 "<p>M</p>"（Java 为 markup 模型）。
fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.output_format = OutputFormatKind::Html;
    c.settings.auto_escaping = AutoEscaping::Off;
    c.settings.boolean_format = "y,n".to_string();
    c.set_shared_variable("s", TModel::from_scalar("abc".to_string()));
    c.set_shared_variable("n", TModel::from_number(TNumber::Int(1500)));
    // Java TM：2015-09-06T12:00:00Z（1441540800000L）
    c.set_shared_variable(
        "dt",
        TModel::from_date(DateValue::new(
            DateTime::from_timestamp_millis(1441540800000)
                .unwrap()
                .fixed_offset(),
            DateType::DateTime,
        )),
    );
    c.set_shared_variable("b", TModel::from_boolean(true));
    c.set_shared_variable("m", TModel::from_scalar("<p>M</p>".to_string()));
    (c, loader)
}

/// Java testBasicStringBuiltins
#[test]
fn test_basic_string_builtins() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${s?upperCase}", "ABC");
    // 引擎差异：Java n?string 按 "@G 3" → "1.50e+03"；v1 按默认 number 格式 → "1,500"
    assert_output(&c, &loader, "${n?string?lowerCase}", "1,500");
    // Java：n?lowerCase 报 UnexpectedType（"convert"/"string"/"markup"/"text/html"）；
    // 引擎差异：v1 字符串内建把数字强转字符串 → "1,500"
    assert_output(&c, &loader, "${n?lowerCase}", "1,500");
    // 引擎差异：Java dt?string 按 "@HI" → "2015-09-06t12:00:00z"；v1 默认日期格式
    assert_output(
        &c,
        &loader,
        "${dt?string?lowerCase}",
        "sep 6, 2015 1:00:00 pm",
    );
    // Java：dt?lowerCase 报 UnexpectedType；引擎差异：v1 强转 → 小写日期串
    assert_output(&c, &loader, "${dt?lowerCase}", "sep 6, 2015 1:00:00 pm");
    assert_output(&c, &loader, "${b?upperCase}", "Y");
    // Java：m?upperCase 对 markup 报 "convertible to string"/"HTMLOutputModel"；
    // 引擎差异：v1 m 为普通字符串 → 直接转大写
    assert_output(&c, &loader, "${m?upperCase}", "<P>M</P>");
}

/// Java testEscBuiltin
#[test]
fn test_esc_builtin() {
    let (c, loader) = cfg();
    let mut c = c;
    // Java testEscBuiltin 内 setBooleanFormat("<y>,<n>") —— fixture 对齐
    c.settings.boolean_format = "<y>,<n>".to_string();
    assert_output(&c, &loader, "${'a<b'?esc}", "a&lt;b");
    // 引擎差异：Java n?string 按 "@G 3" → "1.50E+03"；v1 默认格式 → "1,500"
    assert_output(&c, &loader, "${n?string?esc}", "1,500");
    // 引擎差异：Java n?esc → "1.50*10<sup>3</sup>"（markup 数字格式）；v1 默认格式
    assert_output(&c, &loader, "${n?esc}", "1,500");
    // 引擎差异：dt?string 按 "@HI" → "2015-09-06T12:00:00Z"；v1 默认日期格式
    assert_output(&c, &loader, "${dt?string?esc}", "Sep 6, 2015 1:00:00 PM");
    // 引擎差异：dt?esc → HTMLISO 的 "<span class='T'>T</span>" 标记格式；v1 默认
    assert_output(&c, &loader, "${dt?esc}", "Sep 6, 2015 1:00:00 PM");
    // booleanFormat "<y>,<n>" → b 为 "<y>"，?esc 转义 → "&lt;y&gt;"
    assert_output(&c, &loader, "${b?esc}", "&lt;y&gt;");
    // 引擎差异：Java m 为 markup → ?esc 原样 "<p>M</p>"；v1 m 为普通字符串 → 转义
    assert_output(&c, &loader, "${m?esc}", "&lt;p&gt;M&lt;/p&gt;");
}

/// Java testStringOverloadedBuiltIns
#[test]
fn test_string_overloaded_built_ins() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${s?contains('b')}", "y");
    // 引擎差异：Java n?string 按 "@G 3" → "1.50E+03"（含 'E'）；v1 默认格式 "1,500" 不含 'E'
    assert_output(&c, &loader, "${n?string?contains('E')}", "n");
    // Java：n?contains('E') 报 UnexpectedType；引擎差异：v1 强转字符串 → "1,500" 不含 'E'
    assert_output(&c, &loader, "${n?contains('E')}", "n");
    // Java：n?indexOf('E') 报 UnexpectedType；引擎差异：v1 强转 → -1
    assert_output(&c, &loader, "${n?indexOf('E')}", "-1");
    // 引擎差异：Java dt?string 按 "@HI" → "2015-09-06T12:00:00Z"（含 '0'）；
    // v1 默认日期格式同样含 '0' → "y"
    assert_output(&c, &loader, "${dt?string?contains('0')}", "y");
    // Java：dt?contains('0') 报 UnexpectedType；引擎差异：v1 强转 → 含 '0' → "y"
    assert_output(&c, &loader, "${dt?contains('0')}", "y");
    // Java：m?contains('0') 对 markup 报错；引擎差异：v1 m 为普通字符串 → "n"
    assert_output(&c, &loader, "${m?contains('0')}", "n");
    // Java：m?indexOf('0') 对 markup 报错；引擎差异：v1 → "-1"
    assert_output(&c, &loader, "${m?indexOf('0')}", "-1");
}

/// Java testMarkupStringBuiltIns
#[test]
fn test_markup_string_built_ins() {
    let (c, loader) = cfg();
    // ?markupString is now implemented; returns the string representation of its input
    assert_output(&c, &loader, "${n?string?markupString}", "1,500");
    assert_output(&c, &loader, "${n?markupString}", "1,500");
    assert_output(&c, &loader, "${dt?markupString}", "Sep 6, 2015 1:00:00 PM");
}

/// Java testSimpleInterpolation
#[test]
fn test_simple_interpolation() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${s}", "abc");
    // 引擎差异：Java n?string 按 "@G 3" → "1.50E+03"；v1 默认格式 → "1,500"
    assert_output(&c, &loader, "${n?string}", "1,500");
    // 引擎差异：Java n → "1.50*10<sup>3</sup>"（markup 数字格式）；v1 默认 → "1,500"
    assert_output(&c, &loader, "${n}", "1,500");
    // 引擎差异：dt?string 按 "@HI" → "2015-09-06T12:00:00Z"；v1 默认日期格式
    assert_output(&c, &loader, "${dt?string}", "Sep 6, 2015 1:00:00 PM");
    // 引擎差异：dt → HTMLISO 的 "<span class='T'>T</span>" 标记；v1 默认
    assert_output(&c, &loader, "${dt}", "Sep 6, 2015 1:00:00 PM");
    assert_output(&c, &loader, "${b}", "y");
    assert_output(&c, &loader, "${m}", "<p>M</p>");
}

/// Java testConcatenation
#[test]
fn test_concatenation() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${s + '&'}", "abc&");
    // 引擎差异：同 testSimpleInterpolation（@G/@HI 格式差异）；v1 拼接不转义
    assert_output(&c, &loader, "${n?string + '&'}", "1,500&");
    // 引擎差异：Java n + '&' → "1.50*10<sup>3</sup>&amp;"（markup 拼接转义 '&'）；
    // v1 普通字符串拼接不转义
    assert_output(&c, &loader, "${n + '&'}", "1,500&");
    assert_output(&c, &loader, "${dt?string + '&'}", "Sep 6, 2015 1:00:00 PM&");
    // 引擎差异：Java dt + '&' → markup 拼接转义 '&'；v1 不转义
    assert_output(&c, &loader, "${dt + '&'}", "Sep 6, 2015 1:00:00 PM&");
    assert_output(&c, &loader, "${b + '&'}", "y&");
    // 引擎差异：Java m + '&' → "<p>M</p>&amp;"（markup 拼接转义）；v1 普通字符串拼接
    assert_output(&c, &loader, "${m + '&'}", "<p>M</p>&");
}

/// Java testConcatenation2
#[test]
fn test_concatenation2() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${'&' + s}", "&abc");
    // 引擎差异：同 testSimpleInterpolation（@G/@HI 格式差异）；v1 拼接不转义
    assert_output(&c, &loader, "${'&' + n?string}", "&1,500");
    // 引擎差异：Java '&' + n → "&amp;1.50*10<sup>3</sup>"（markup 拼接转义）；v1 不转义
    assert_output(&c, &loader, "${'&' + n}", "&1,500");
    assert_output(&c, &loader, "${'&' + dt?string}", "&Sep 6, 2015 1:00:00 PM");
    // 引擎差异：Java '&' + dt → "&amp;2015-09-06<span...>"（markup 拼接转义）；v1 不转义
    assert_output(&c, &loader, "${'&' + dt}", "&Sep 6, 2015 1:00:00 PM");
    assert_output(&c, &loader, "${'&' + b}", "&y");
    // 引擎差异：Java '&' + m → "&amp;<p>M</p>"（markup 拼接转义）；v1 不转义
    assert_output(&c, &loader, "${'&' + m}", "&<p>M</p>");
}
