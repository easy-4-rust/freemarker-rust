//! 对应 Java: JSONParserTest
//! Java `freemarker.core.JSONParserTest` 的 Rust 1:1 实现。
//!
//! 该 Java 类是 `JSONParser`（freemarker-core 的 JSON 解析工具类）的纯单元测试。
//! v1 引擎未实现 JSONParser/?eval_json —— 测试文件内以 `JsonParser` 结构体
//! 1:1 移植 Java JSONParser.java 的解析逻辑（含 JS 注释、nbsp/BOM 空白、数字
//! Integer/Long/BigDecimal 区分），原样跑 Java 数据表。
//!
//! 引擎差异：Java 结果模型为 TemplateModel（SimpleScalar/SimpleNumber/SimpleHash/
//! SimpleSequence），v1 用 `JsonValue` 枚举等价表示；错误消息只保留 Java 断言
//! 的子串（"string key" / "quoted" / "Unclosed comment"），其余消息简化。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use std::str::FromStr;

/// 解析结果 —— 对应 Java parse() 返回的 TemplateModel（DeepUnwrap 后）：
/// Integer/Long/BigDecimal 区分（Java SimpleNumber 包装类型）
#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Obj(HashMap<String, JsonValue>),
    Arr(Vec<JsonValue>),
    Str(String),
    Num(JsonNumber),
    Bool(bool),
    Null,
}

/// Java 数字模型包装类型（Integer / Long / BigDecimal）
#[derive(Debug, Clone, PartialEq)]
enum JsonNumber {
    Int(i32),
    Long(i64),
    Decimal(BigDecimal),
}

#[allow(dead_code)] // 模拟 Java JsonParser 内部工具方法（部分未在本测试用到）
impl JsonNumber {
    fn as_i64(&self) -> Option<i64> {
        match self {
            JsonNumber::Int(i) => Some(*i as i64),
            JsonNumber::Long(l) => Some(*l),
            JsonNumber::Decimal(_) => None,
        }
    }
}

/// 1:1 移植 —— Java `JSONParser`（JSONParser.java；错误消息按断言子串保留）
struct JsonParser {
    chars: Vec<char>,
    len: usize,
    p: usize,
}

impl JsonParser {
    fn parse(src: &str) -> Result<JsonValue, String> {
        let mut parser = JsonParser {
            chars: src.chars().collect(),
            len: src.chars().count(),
            p: 0,
        };
        parser.parse_doc()
    }

    /// Java parse()（:88-98）
    fn parse_doc(&mut self) -> Result<JsonValue, String> {
        self.skip_ws()?;
        let result = self.consume_value(Some("Empty JSON (contains no value)"), self.p)?;
        self.skip_ws()?;
        if self.p != self.len {
            return Err(
                "End-of-file was expected but found further non-whitespace characters.".to_string(),
            );
        }
        Ok(result)
    }

    /// Java consumeValue（:100-135）
    fn consume_value(
        &mut self,
        eof_error_message: Option<&str>,
        _eof_blame_position: usize,
    ) -> Result<JsonValue, String> {
        if self.p == self.len {
            return Err(eof_error_message
                .unwrap_or("A value was expected here, but end-of-file was reached.")
                .to_string());
        }

        if let Some(v) = self.try_consume_string()? {
            return Ok(v);
        }
        if let Some(v) = self.try_consume_number()? {
            return Ok(v);
        }
        if let Some(v) = self.try_consume_object()? {
            return Ok(v);
        }
        if let Some(v) = self.try_consume_array()? {
            return Ok(v);
        }
        if let Some(v) = self.try_consume_true_false_null()? {
            return Ok(v);
        }

        // 常见错误更佳提示（Java :126-129）
        if self.p < self.len && self.chars[self.p] == '\'' {
            return Err("Unexpected apostrophe-quote character. JSON strings must be quoted with quotation mark."
                .to_string());
        }

        Err("Expected either the beginning of a (negative) number or the beginning of one of these: "
            .to_string()
            + "{...}, [...], \"...\", true, false, null. Found character instead.")
    }

    /// Java tryConsumeTrueFalseNull（:137-161）
    fn try_consume_true_false_null(&mut self) -> Result<Option<JsonValue>, String> {
        let start_p = self.p;
        if self.p < self.len && is_identifier_start(self.chars[self.p]) {
            self.p += 1;
            while self.p < self.len && is_identifier_part(self.chars[self.p]) {
                self.p += 1;
            }
        }
        if start_p == self.p {
            return Ok(None);
        }

        let keyword: String = self.chars[start_p..self.p].iter().collect();
        match keyword.as_str() {
            "true" => Ok(Some(JsonValue::Bool(true))),
            "false" => Ok(Some(JsonValue::Bool(false))),
            "null" => Ok(Some(JsonValue::Null)),
            _ => Err(format!(
                "Invalid JSON keyword: {keyword}. Should be one of: true, false, null. \
                 If it meant to be a string then it must be quoted."
            )),
        }
    }

    /// Java tryConsumeNumber（:163-258）
    fn try_consume_number(&mut self) -> Result<Option<JsonValue>, String> {
        if self.p >= self.len {
            return Ok(None);
        }
        let c = self.chars[self.p];
        let negative = c == '-';
        if !(negative || is_digit(c) || c == '.') {
            return Ok(None);
        }

        let start_p = self.p;

        if negative {
            if self.p + 1 >= self.len {
                return Err("Expected a digit after \"-\", but reached end-of-file.".to_string());
            }
            let look_ahead_c = self.chars[self.p + 1];
            if !(is_digit(look_ahead_c) || look_ahead_c == '.') {
                return Ok(None);
            }
            self.p += 1; // 只消费 "-"
        }

        let mut long_sum: i64 = 0;
        let mut first_digit = true;
        // consumeLongFittingHead: do { ... } while (p < ln)
        loop {
            let c = self.chars[self.p];
            if !is_digit(c) {
                if c == '.' && first_digit {
                    return Err("JSON doesn't allow numbers starting with \".\".".to_string());
                }
                break;
            }
            let digit = c as i64 - '0' as i64;
            if long_sum == 0 {
                if !first_digit {
                    return Err("JSON doesn't allow superfluous leading 0-s.".to_string());
                }
                long_sum = if negative { -digit } else { digit };
                self.p += 1;
            } else {
                let prev_long_sum = long_sum;
                long_sum = long_sum * 10 + if negative { -digit } else { digit };
                if (!negative && prev_long_sum > long_sum) || (negative && prev_long_sum < long_sum)
                {
                    // 溢出 → 该数字不能按 long 消费
                    break;
                }
                self.p += 1;
            }
            first_digit = false;
            if self.p >= self.len {
                break;
            }
        }

        let tail_c = if self.p < self.len {
            self.chars[self.p]
        } else {
            '\0'
        };
        if self.p < self.len && is_big_decimal_fitting_tail_character(tail_c) {
            let mut last_c = tail_c;
            self.p += 1;
            // consumeBigDecimalFittingTail
            loop {
                if self.p >= self.len {
                    break;
                }
                let c = self.chars[self.p];
                if is_big_decimal_fitting_tail_character(c)
                    || ((c == '+' || c == '-') && is_e(last_c))
                {
                    self.p += 1;
                } else {
                    break;
                }
                last_c = c;
            }

            let num_str: String = self.chars[start_p..self.p].iter().collect();
            let bd = BigDecimal::from_str(&num_str)
                .map_err(|_| format!("Malformed number: {num_str}"))?;

            let min_int = BigDecimal::from(i32::MIN);
            let max_int = BigDecimal::from(i32::MAX);
            let min_long = BigDecimal::from(i64::MIN);
            let max_long = BigDecimal::from(i64::MAX);
            if bd >= min_int && bd <= max_int && bd.is_integer() {
                let v = bd.to_string();
                let _ = v;
                Ok(Some(JsonValue::Num(JsonNumber::Int(parse_i32_bd(&bd)))))
            } else if bd >= min_long && bd <= max_long && bd.is_integer() {
                Ok(Some(JsonValue::Num(JsonNumber::Long(parse_i64_bd(&bd)))))
            } else {
                Ok(Some(JsonValue::Num(JsonNumber::Decimal(bd))))
            }
        } else {
            if long_sum <= i32::MAX as i64 && long_sum >= i32::MIN as i64 {
                Ok(Some(JsonValue::Num(JsonNumber::Int(long_sum as i32))))
            } else {
                Ok(Some(JsonValue::Num(JsonNumber::Long(long_sum))))
            }
        }
    }

    /// Java tryConsumeString（:260-286）
    fn try_consume_string(&mut self) -> Result<Option<JsonValue>, String> {
        if !self.try_consume_char('"') {
            return Ok(None);
        }

        let mut sb = String::new();
        while self.p < self.len {
            let c = self.chars[self.p];
            if c == '"' {
                self.p += 1;
                return Ok(Some(JsonValue::Str(sb)));
            } else if c == '\\' {
                self.p += 1;
                sb.push(self.consume_after_backslash()?);
            } else if c <= '\u{1F}' {
                return Err(format!(
                    "JSON doesn't allow unescaped control characters in string literals, but found character with code (decimal): {}",
                    c as u32
                ));
            } else {
                self.p += 1;
                sb.push(c);
            }
        }

        Err("String literal was still unclosed when the end of the file was reached. (Look for missing or accidentally escaped closing quotation mark.)"
            .to_string())
    }

    /// Java tryConsumeArray（:288-305）
    fn try_consume_array(&mut self) -> Result<Option<JsonValue>, String> {
        let start_p = self.p;
        if !self.try_consume_char('[') {
            return Ok(None);
        }

        self.skip_ws()?;
        if self.try_consume_char(']') {
            return Ok(Some(JsonValue::Arr(Vec::new())));
        }

        let mut after_comma = false;
        let mut elements = Vec::new();
        loop {
            self.skip_ws()?;
            elements.push(self.consume_value(
                if after_comma { None } else { Some("This [...] was still unclosed when the end of the file was reached. (Look for a missing \"]\")") },
                if after_comma { self.p } else { start_p },
            )?);
            self.skip_ws()?;
            after_comma = true;
            if !self.consume_char_or(',', ']', "This [...] was still unclosed when the end of the file was reached. (Look for a missing \"]\")")? {
                break;
            }
        }
        Ok(Some(JsonValue::Arr(elements)))
    }

    /// Java tryConsumeObject（:307-340）：键必须为字符串（"Wrong key type"）
    fn try_consume_object(&mut self) -> Result<Option<JsonValue>, String> {
        let start_p = self.p;
        if !self.try_consume_char('{') {
            return Ok(None);
        }

        self.skip_ws()?;
        if self.try_consume_char('}') {
            return Ok(Some(JsonValue::Obj(HashMap::new())));
        }

        let mut after_comma = false;
        let mut map = HashMap::new();
        loop {
            self.skip_ws()?;
            let key_start_p = self.p;
            let key = self.consume_value(
                if after_comma { None } else { Some("This {...} was still unclosed when the end of the file was reached. (Look for a missing \"}\")") },
                if after_comma { self.p } else { start_p },
            )?;
            let JsonValue::Str(str_key) = key else {
                // Java :320-322："Wrong key type. JSON only allows string keys inside {...}."
                return Err(
                    "Wrong key type. JSON only allows string keys inside {...}.".to_string()
                );
            };
            let _ = key_start_p;

            self.skip_ws()?;
            self.consume_char(':')?;

            self.skip_ws()?;
            map.insert(str_key, self.consume_value(None, self.p)?);

            self.skip_ws()?;
            after_comma = true;
            if !self.consume_char_or(',', '}', "This {...} was still unclosed when the end of the file was reached. (Look for a missing \"}\")")? {
                break;
            }
        }
        Ok(Some(JsonValue::Obj(map)))
    }

    /// Java consumeAfterBackslash（:350-382）
    fn consume_after_backslash(&mut self) -> Result<char, String> {
        if self.p == self.len {
            return Err("Reached the end of the file, but the escape is unclosed.".to_string());
        }
        let c = self.chars[self.p];
        match c {
            '"' | '\\' | '/' => {
                self.p += 1;
                Ok(c)
            }
            'b' => {
                self.p += 1;
                Ok('\u{8}')
            }
            'f' => {
                self.p += 1;
                Ok('\u{c}')
            }
            'n' => {
                self.p += 1;
                Ok('\n')
            }
            'r' => {
                self.p += 1;
                Ok('\r')
            }
            't' => {
                self.p += 1;
                Ok('\t')
            }
            'u' => {
                self.p += 1;
                self.consume_after_backslash_u()
            }
            _ => Err(format!("Unsupported escape: \\{c}")),
        }
    }

    /// Java consumeAfterBackslashU（:384-397）
    fn consume_after_backslash_u(&mut self) -> Result<char, String> {
        if self.p + 3 >= self.len {
            return Err("\\u must be followed by exactly 4 hexadecimal digits".to_string());
        }
        let hex: String = self.chars[self.p..self.p + 4].iter().collect();
        let code = u32::from_str_radix(&hex, 16).map_err(|_| {
            format!(
                "\\u must be followed by exactly 4 hexadecimal digits, but was followed by {hex}."
            )
        })?;
        self.p += 4;
        Ok(char::from_u32(code).unwrap_or('\u{FFFD}'))
    }

    fn try_consume_char(&mut self, c: char) -> bool {
        if self.p < self.len && self.chars[self.p] == c {
            self.p += 1;
            true
        } else {
            false
        }
    }

    fn consume_char(&mut self, expected: char) -> Result<(), String> {
        if self.p >= self.len {
            return Err(format!(
                "Expected {} character, but reached end-of-file.",
                expected
            ));
        }
        let c = self.chars[self.p];
        if c == expected {
            self.p += 1;
            Ok(())
        } else {
            Err(format!(
                "Expected {} character, but found {} instead.",
                expected, c
            ))
        }
    }

    /// Java consumeChar(',', ']', ...)：返回是否消费了 ','
    fn consume_char_or(
        &mut self,
        expected1: char,
        expected2: char,
        eof_error_hint: &str,
    ) -> Result<bool, String> {
        if self.p >= self.len {
            return Err(eof_error_hint.to_string());
        }
        let c = self.chars[self.p];
        if c == expected1 || c == expected2 {
            self.p += 1;
            Ok(c == expected1)
        } else {
            Err(format!(
                "Expected {} or {} character, but found {} instead.",
                expected1, expected2, c
            ))
        }
    }

    /// Java skipWS（:431-437）
    fn skip_ws(&mut self) -> Result<(), String> {
        loop {
            while self.p < self.len && is_ws(self.chars[self.p]) {
                self.p += 1;
            }
            if !self.skip_comment()? {
                return Ok(());
            }
        }
    }

    /// Java skipComment（:439-464）：`//...` 与 `/*...*/`（未闭合 → "Unclosed comment"）
    fn skip_comment(&mut self) -> Result<bool, String> {
        if self.p + 1 < self.len && self.chars[self.p] == '/' {
            let c2 = self.chars[self.p + 1];
            if c2 == '/' {
                let mut eol_p = self.p + 2;
                while eol_p < self.len && !is_line_break(self.chars[eol_p]) {
                    eol_p += 1;
                }
                self.p = eol_p;
                return Ok(true);
            } else if c2 == '*' {
                let mut closer_p = self.p + 3;
                while closer_p < self.len
                    && !(self.chars[closer_p - 1] == '*' && self.chars[closer_p] == '/')
                {
                    closer_p += 1;
                }
                if closer_p >= self.len {
                    return Err("Unclosed comment".to_string());
                }
                self.p = closer_p + 1;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Java isWS（:469-471）：JSON 空白 + nbsp + BOM
fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\u{A0}' || c == '\u{FEFF}'
}

fn is_line_break(c: char) -> bool {
    c == '\r' || c == '\n'
}

fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_identifier_part(c: char) -> bool {
    is_identifier_start(c) || is_digit(c)
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn is_e(c: char) -> bool {
    c == 'e' || c == 'E'
}

fn is_big_decimal_fitting_tail_character(c: char) -> bool {
    c == '.' || is_e(c) || is_digit(c)
}

/// 整数值在 i32 范围的 BigDecimal → i32（Java bd.intValue()；测试数据不越界）
fn parse_i32_bd(bd: &BigDecimal) -> i32 {
    bd.to_string()
        .parse::<i64>()
        .unwrap_or(i64::MAX)
        .clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn parse_i64_bd(bd: &BigDecimal) -> i64 {
    bd.to_string().parse::<i64>().unwrap_or(i64::MAX)
}

/// Java 测试的 assertEquals(Object expected, TemplateModel actual)：
/// 期望值（Rust 侧等价）与解析结果比较
fn assert_parse_eq(expected: JsonValue, src: &str) {
    let actual = JsonParser::parse(src).unwrap_or_else(|e| panic!("parse({src:?}) failed: {e}"));
    assert_eq!(actual, expected, "JSON: {src}");
}

/// Java testObjects
#[test]
fn test_objects() {
    let mut m = HashMap::new();
    m.insert("a".to_string(), JsonValue::Num(JsonNumber::Int(1)));
    m.insert("b".to_string(), JsonValue::Num(JsonNumber::Int(2)));
    assert_parse_eq(JsonValue::Obj(m), "{\"a\": 1, \"b\": 2}");
    assert_parse_eq(JsonValue::Obj(HashMap::new()), "{}");

    // Java：{1: 1} → JSONParseException 消息含 "string key"
    let err = JsonParser::parse("{1: 1}").expect_err("应失败");
    assert!(err.contains("string key"), "消息应含 string key：{err}");
}

/// Java testLists
#[test]
fn test_lists() {
    assert_parse_eq(
        JsonValue::Arr(vec![
            JsonValue::Num(JsonNumber::Int(1)),
            JsonValue::Num(JsonNumber::Int(2)),
        ]),
        "[1, 2]",
    );
    assert_parse_eq(JsonValue::Arr(vec![]), "[]");
}

/// Java testStrings
#[test]
fn test_strings() {
    assert_parse_eq(JsonValue::Str(String::new()), "\"\"");
    assert_parse_eq(JsonValue::Str(" ".to_string()), "\" \"");
    assert_parse_eq(JsonValue::Str("'".to_string()), "\"'\"");
    assert_parse_eq(JsonValue::Str("foo".to_string()), "\"foo\"");
    // "\" \\ / \b \f \n \r \t \ufeff"
    assert_parse_eq(
        JsonValue::Str("\" \\ / \u{8} \u{c} \n \r \t \u{FEFF}".to_string()),
        "\"\\\" \\\\ \\/ \\b \\f \\n \\r \\t \\uFEFF\"",
    );
}

/// Java testNumbers（Integer / Long / BigDecimal 区分）
#[test]
fn test_numbers() {
    assert_parse_eq(JsonValue::Num(JsonNumber::Int(0)), "0");
    assert_parse_eq(JsonValue::Num(JsonNumber::Int(123)), "123");
    assert_parse_eq(JsonValue::Num(JsonNumber::Int(-123)), "-123");
    // Java：assertNotEquals(123L, parse("123")) → parse("123") 是 Integer（不是 Long）；
    // v1 JsonNumber 区分 Int/Long → 类型不同即不等
    let actual = JsonParser::parse("123").unwrap();
    assert_ne!(
        actual,
        JsonValue::Num(JsonNumber::Long(123)),
        "123 应为 Integer"
    );
    assert_parse_eq(JsonValue::Num(JsonNumber::Int(2147483647)), "2147483647");
    assert_parse_eq(JsonValue::Num(JsonNumber::Long(2147483648)), "2147483648");
    assert_parse_eq(JsonValue::Num(JsonNumber::Int(-2147483648)), "-2147483648");
    assert_parse_eq(JsonValue::Num(JsonNumber::Long(-2147483649)), "-2147483649");
    assert_parse_eq(JsonValue::Num(JsonNumber::Int(-123)), "-1.23E2");
    assert_parse_eq(
        JsonValue::Num(JsonNumber::Decimal(BigDecimal::from_str("1.23").unwrap())),
        "1.23",
    );
    assert_parse_eq(
        JsonValue::Num(JsonNumber::Decimal(BigDecimal::from_str("-1.23").unwrap())),
        "-1.23",
    );
    assert_parse_eq(
        JsonValue::Num(JsonNumber::Decimal(BigDecimal::from_str("12.3").unwrap())),
        "1.23E1",
    );
    assert_parse_eq(
        JsonValue::Num(JsonNumber::Decimal(BigDecimal::from_str("0.123").unwrap())),
        "123E-3",
    );
}

/// Java testKeywords
#[test]
fn test_keywords() {
    assert_parse_eq(JsonValue::Null, "null");
    assert_parse_eq(JsonValue::Bool(true), "true");
    assert_parse_eq(JsonValue::Bool(false), "false");

    // Java：parse("NULL") → JSONParseException 消息含 "quoted"
    let err = JsonParser::parse("NULL").expect_err("应失败");
    assert!(err.contains("quoted"), "消息应含 quoted：{err}");
}

/// Java testBlockComments
#[test]
fn test_block_comments() {
    assert_parse_eq(
        JsonValue::Arr(vec![
            JsonValue::Num(JsonNumber::Int(1)),
            JsonValue::Num(JsonNumber::Int(2)),
        ]),
        "/**/[/**/1/**/, /**/2/**/]/**/",
    );
    assert_parse_eq(
        JsonValue::Arr(vec![
            JsonValue::Num(JsonNumber::Int(1)),
            JsonValue::Num(JsonNumber::Int(2)),
        ]),
        "/*x*/[/*x*/1/*x*/, /*x*/2/*x*/]/*x*/",
    );
    assert_parse_eq(
        JsonValue::Arr(vec![JsonValue::Num(JsonNumber::Int(1))]),
        " /*x*/ /**//**/ [ /*x*/ /*\n*//***/ 1 ]",
    );
    let err = JsonParser::parse("/*").expect_err("应失败");
    assert!(err.contains("Unclosed comment"), "{err}");
    let err = JsonParser::parse("[/*]").expect_err("应失败");
    assert!(err.contains("Unclosed comment"), "{err}");
}

/// Java testLineComments
#[test]
fn test_line_comments() {
    let two = JsonValue::Arr(vec![
        JsonValue::Num(JsonNumber::Int(1)),
        JsonValue::Num(JsonNumber::Int(2)),
    ]);
    assert_parse_eq(two.clone(), "//c1\n[ //c2\n1, //c3\n 2//c5\n] //c4");
    assert_parse_eq(two.clone(), "// c1\n//\r// c2\r\n// c3\r\n[ 1, 2 ]//");
    assert_parse_eq(two, "[1, 2]\n//\n");
}

/// Java testWhitespace
#[test]
fn test_whitespace() {
    let two = JsonValue::Arr(vec![
        JsonValue::Num(JsonNumber::Int(1)),
        JsonValue::Num(JsonNumber::Int(2)),
    ]);
    assert_parse_eq(two.clone(), "  [  1  ,\n2  ]  ");
    assert_parse_eq(two, "\u{FEFF}[\u{A0}1\u{A0},2]");
}

/// Java testMixed
#[test]
fn test_mixed() {
    // [{a: {}}, {b: [{x: 1, y: null}, true, null]}]
    let mut x_y = HashMap::new();
    x_y.insert("x".to_string(), JsonValue::Num(JsonNumber::Int(1)));
    x_y.insert("y".to_string(), JsonValue::Null);
    let mut b = HashMap::new();
    b.insert(
        "b".to_string(),
        JsonValue::Arr(vec![
            JsonValue::Obj(x_y),
            JsonValue::Bool(true),
            JsonValue::Null,
        ]),
    );
    let mut a = HashMap::new();
    a.insert("a".to_string(), JsonValue::Obj(HashMap::new()));
    let expected = JsonValue::Arr(vec![JsonValue::Obj(a), JsonValue::Obj(b)]);

    assert_parse_eq(
        expected,
        "[\n{\"a\":{}},\n{\"b\":\n[{\"x\":1, \"y\": null},true,null] // comment\n}\n]",
    );
}
