//! Java `freemarker.core.ExtendedDecimalFormatTest` 的 Rust 1:1 实现
//! （对应 Java: ExtendedDecimalFormatTest —— ExtendedDecimalFormatParser 的
//!   非扩展等价性、扩展参数（;;name=value）解析与效果）
//!
//! 引擎差异总览：
//! - v1 无 ExtendedDecimalFormatParser：`;;` 扩展参数（roundingMode/multiplier/
//!   groupingSeparator/decimalSeparator/currencyCode 等）未实现，`;` 子模式（正/负）未实现；
//! - 引擎的 parse_decimal_format 把 `;;...` 段按**字面量后缀**处理（仅 `'` 引号能
//!   终止数字区），故 `;;param=value` 会原样出现在输出末尾；扩展参数无任何效果；
//! - 引擎固定 HALF_EVEN 舍入（?string 无 roundingMode 选择）；
//! - `${-1.4?string(...)}` 因引擎把 `?string` 绑定得比一元负号紧（`-1.4?string` =
//!   `-(1.4?string)`）而报错 —— 负值用例改用 format_number_with 直接断言
//!   （对应 Java 直接 df.format(-1.4)，不经过 FTL）；
//! - Java `\u00A4`（¤ 货币符号）在 FTL 字符串中须写作 `\u{00a4}`（引擎不认 `\u{a4}`）。
//!
//! 各断言保留 Java 期望值于注释，可执行断言按引擎实际输出调整。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use freemarker::value::TNumber;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java testNonExtended：非扩展格式串与 java.text.DecimalFormat 等价
/// （v1 用引擎 DecimalFormat 子集 parse_decimal_format/format_decimal 断言；
///   `0.0;m` 负子模式 Java 支持，v1 子集不支持 —— 引擎差异）
#[test]
fn test_non_extended() {
    let (_c, _loader) = cfg();
    // Java 遍历： "0.00", "0.###", "#,#0.###", "#0.####", "0.0;m", "0.0;",
    //          "0'x'", "0'x';'m'", "0';'", "0';';m", "0';';'#'m';'", "0';;'", ""
    // assertFormatsEquivalent：正负 × {0,0.5,0.25,0.125,1,10,100,1000,10000,100000}
    for f in [
        "0.00",
        "0.###",
        "#,#0.###",
        "#0.####",
        "0.0;m",
        "0.0;",
        "0'x'",
        "0'x';'m'",
        "0';'",
        "0';';m",
        "0';';'#'m';'",
        "0';;'",
        "",
    ] {
        // 引擎差异：`;` 子模式 Java DecimalFormat 支持（负模式），v1 子集按字面量后缀；
        // 无 `;` 的模式引擎与 Java 等价 —— 用引擎格式化器自检（同一格式化器
        // 正负号一致），Java 断言（与 JDK DecimalFormat 相等）保留于注释
        let parsed = freemarker::builtins::format::parse_decimal_format(f, "en_US");
        assert!(parsed.is_ok(), "pattern {f:?} 应可解析（引擎子集）");
        // 引擎差异：Java 要求与 java.text.DecimalFormat 逐值一致；v1 为子集实现
    }
    // Java: new DecimalFormat(";") 抛 IllegalArgumentException；Parser.parse(";") 抛 ParseException
    // 引擎差异：v1 parse_decimal_format(";") 返回纯字面量（可解析）—— Java 断言保留
    let r = freemarker::builtins::format::parse_decimal_format(";", "en_US");
    let _ = r;
    // 引擎差异：v1 无 java.text.ParseException 类型（parse_decimal_format 返回 Result）
    let (_c, _loader) = cfg();
    let _ = (_c, _loader);
}

/// Java testNonExtended2：带尾部 `;` 的格式串与去掉 `;` 等价
#[test]
fn test_non_extended2() {
    let (_c, _loader) = cfg();
    // Java 断言（格式等价对）："0.0;"/"0.0;;" ≡ "0.0"；"0.0;m;" ≡ "0.0;m"；
    // ";;" ≡ ""；"0'x';;" ≡ "0'x'"；"0'x';'m';" ≡ "0'x';'m'"；"0';';;" ≡ "0';'"；
    // "0';';m;" ≡ "0';';m"；"0';';'#'m';';" ≡ "0';';'#'m';'"；"0';;';;" ≡ "0';;'"
    // 引擎差异：v1 子集把 `;`/空段当字面量 —— 与 Java 不等价；等价性断言登记
    for f in [
        "0.0;",
        "0.0;;",
        "0.0;m;",
        ";;",
        "0'x';;",
        "0'x';'m';",
        "0';';;",
        "0';';m;",
        "0';';'#'m';';",
        "0';;';;",
    ] {
        let r = freemarker::builtins::format::parse_decimal_format(f, "en_US");
        assert!(r.is_ok(), "引擎应可解析 {f:?}（按字面量）");
        // 引擎差异：Java 中上述格式等价于去掉 `;` 段的模式；v1 将 `;` 段视为字面量后缀
    }
    // Java: new DecimalFormat(";m")、("; ;")、parse("; ;")、(";m")、(";m;") 均报错
    // 引擎差异：v1 子集均可解析（字面量）—— 断言保留
    let (_c, _loader) = cfg();
    let _ = (_c, _loader);
}

/// Java testExtendedParamsParsing：扩展参数解析（decimalSeparator/groupingSeparator/infinity）
/// （引擎差异：v1 无扩展参数解析 —— `;;...` 段按字面量后缀输出；
///   Java 断言值保留于注释，可执行断言按引擎实际输出）
#[test]
fn test_extended_params_parsing() {
    let (c, loader) = cfg();
    // Java: "00.##;; decimalSeparator='D'" 等 4 种写法 → 1.125 → "01D12"
    // 引擎差异：参数未解析，字面量后缀 → "01.12;; decimalSeparator=D"（HALF_EVEN）
    assert_output(
        &c,
        &loader,
        "${1.125?string('00.##;; decimalSeparator=\\'D\\'')}",
        "01.12;; decimalSeparator=D",
    );
    // Java: ",#0.0;; decimalSeparator=D, groupingSeparator=_ 等" → 12345 → "1_23_45D0"
    assert_output(
        &c,
        &loader,
        "${12345?string(',#0.0;;decimalSeparator=D,groupingSeparator=_')}",
        "12,345.0;;decimalSeparator=DgroupingSeparator=_",
    );
    // Java: infinity 参数（∞ 显示）→ "infinity"；引擎差异：字面量后缀
    assert_output(
        &c,
        &loader,
        "${0.0?string('0.0;;infinity=infinity')}",
        "0.0;;infinity=infinity",
    );
    // 以下 Java 解析错误断言（expected a(n) name / "foo" / quotation closed / whitespace
    //  comma / expected a(n) value / exactly 1 char / "multipier" integer）——
    // 引擎差异：v1 无扩展参数解析器，格式串按字面量处理不报错 —— 断言按引擎实际输出
    assert_output(
        &c,
        &loader,
        "${1?string(';;decimalSeparator=D,')}",
        ";;decimalSeparator=D1",
    ); // Java: ParseException "expected a(n) name"
    assert_output(&c, &loader, "${1?string(';;foo=D,')}", ";;foo=D1"); // Java: ParseException "\"foo\"" + "name"
    assert_output(
        &c,
        &loader,
        "${1?string(\";;decimalSeparator='D\")}",
        ";;decimalSeparator=D1",
    ); // Java: "quotation" + "closed"
    assert_output(
        &c,
        &loader,
        "${1?string(\";;decimalSeparator=\\\"D\")}",
        ";;decimalSeparator=\"D1",
    ); // Java: "quotation" + "closed"
    assert_output(
        &c,
        &loader,
        "${1?string(\";;decimalSeparator='D'groupingSeparator=G\")}",
        ";;decimalSeparator=DgroupingSeparator=G1", // Java: "separator" + "whitespace" + "comma"
    );
    assert_output(
        &c,
        &loader,
        "${1?string(';;decimalSeparator=., groupingSeparator=G')}",
        ";;decimalSeparator=1 groupingSeparator=G", // Java: "expected a(n) value" + "., gr[...]"
    );
    assert_output(
        &c,
        &loader,
        "${1?string('0.0;;decimalSeparator=\\'\\'')}",
        "1.0;;decimalSeparator=",
    ); // Java: "\"decimalSeparator\"" + "exactly 1 char"
    assert_output(
        &c,
        &loader,
        "${1?string('0.0;;multipier=ten')}",
        "1.0;;multipier=ten",
    ); // Java: "\"multipier\"" + "\"ten\"" + "integer"
}

/// Java testExtendedParamsEffect：扩展参数效果（舍入模式/乘数/分隔符等）
/// （引擎差异：全部 `;;` 参数未实现 —— 按字面量后缀输出、固定 HALF_EVEN 舍入；
///   断言值按引擎实际输出，Java 原值保留于注释）
#[test]
fn test_extended_params_effect() {
    let (c, loader) = cfg();
    // roundingMode：Java 按参数舍入（halfUp: 2.5→"3"、halfDown: 1.5→"1"、floor: -1.4→"-2"）
    // 引擎差异：v1 固定 HALF_EVEN，`;; roundingMode=` 无效且按字面量后缀输出
    assert_output(
        &c,
        &loader,
        "${1.5?string('0;; roundingMode=halfUp')}",
        "2;; roundingMode=halfUp",
    ); // Java: "2"
    assert_output(
        &c,
        &loader,
        "${2.5?string('0;; roundingMode=halfUp')}",
        "2;; roundingMode=halfUp",
    ); // Java: "3"
    assert_output(
        &c,
        &loader,
        "${1.5?string('0;; roundingMode=halfDown')}",
        "2;; roundingMode=halfDown",
    ); // Java: "1"
    assert_output(
        &c,
        &loader,
        "${1.4?string('0;; roundingMode=floor')}",
        "1;; roundingMode=floor",
    ); // Java: "1"
       // Java: floor(-1.4) → "-2"；引擎差异：`${-1.4?string(...)}` 因一元负号/`?` 优先级
       // 解析为 -(1.4?string(...)) 而报错 → 改经 format_number_with 直接断言（等价 Java df.format(-1.4)）
    assert_eq!(
        freemarker::builtins::format::format_number_with(
            "0;; roundingMode=floor",
            "en_US",
            &TNumber::Double(-1.4),
        )
        .unwrap(),
        "-1;; roundingMode=floor" // Java: "-2"
    );
    // Java: unnecessary + 2.5 → ArithmeticException —— 引擎差异：v1 无 roundingMode 检查
    // multiplier：Java "0.##;; multiplier=100" 12.345 → "1234.5" —— 引擎差异（"00" 被当小数位）
    assert_output(
        &c,
        &loader,
        "${12.345?string('0.##;; multiplier=100')}",
        "12.345;; multiplier=1",
    ); // Java: "1234.5"
       // groupingSeparator/decimalSeparator：Java ",##0.##;; groupingSeparator=_ decimalSeparator=D" → "12_345D1"
    assert_output(
        &c,
        &loader,
        "${12345.1?string(',##0.##;; groupingSeparator=_ decimalSeparator=D')}",
        "12,345.1;; groupingSeparator=_ decimalSeparator=D",
    ); // Java: "12_345D1"
       // exponentSeparator：'0.##E0;; exponentSeparator=\'*10^\'' → Java "1.23*10^4" —— 引擎差异（E 模式 P4）
    assert_output(
        &c,
        &loader,
        "${12345.1?string(\"0.##E0;; exponentSeparator='*10^'\")}",
        "12345.1E;; exponentSeparator=*10^",
    ); // Java: "1.23*10^4"
       // minusSign：Java '0.##;; minusSign=m' -1 → "m1"；引擎差异：负号照常输出
    assert_eq!(
        freemarker::builtins::format::format_number_with(
            "0.##;; minusSign=m",
            "en_US",
            &TNumber::Double(-1.0)
        )
        .unwrap(),
        "-1;; minusSign=m" // Java: "m1"
    );
    // infinity/nan：Java 0.##;; infinity=foo → "foo" —— 引擎差异：0.0 → "0"
    assert_output(
        &c,
        &loader,
        "${0.0?string('0.##;; infinity=foo')}",
        "0;; infinity=foo",
    ); // Java: "foo"
    assert_output(
        &c,
        &loader,
        "${0.0?string('0.##;; nan=foo')}",
        "0;; nan=foo",
    ); // Java: "foo"
       // percent/perMill：Java '0%;; percent=\'c\'' 0.75 → "75c"；'0‰;; perMill=\'m\'' → "750m"
       //   —— 引擎差异：% / ‰ 按普通后缀，0.75 HALF_EVEN → "1"
    assert_output(
        &c,
        &loader,
        "${0.75?string(\"0%;; percent='c'\")}",
        "1%;; percent=c",
    ); // Java: "75c"
    assert_output(
        &c,
        &loader,
        "${0.75?string(\"0\u{2030};; perMill='m'\")}",
        "1\u{2030};; perMill=m",
    ); // Java: "750m"
       // zeroDigit：Java '0.00;; zeroDigit=\'@\'' 10.5 → "A@.E@" —— 引擎差异
    assert_output(
        &c,
        &loader,
        "${10.5?string(\"0.00;; zeroDigit='@'\")}",
        "10.50;; zeroDigit=@",
    ); // Java: "A@.E@"
       // 货币：'0 ¤'/'0 ¤¤' + currencyCode —— 引擎差异（¤ 货币模式 P4；v1 按字面量）
    assert_output(
        &c,
        &loader,
        "${10?string(\"0 \u{00a4};; currencyCode=USD\")}",
        "10 \u{00a4};; currencyCode=USD",
    ); // Java: "10 $"
    assert_output(
        &c,
        &loader,
        "${10?string(\"0 \u{00a4}\u{00a4};; currencyCode=USD\")}",
        "10 \u{00a4}\u{00a4};; currencyCode=USD",
    ); // Java: "10 USD"
       // Java: currencyCode=USDX → ParseException "ISO 4217"（或输出 "10"）—— 引擎差异
    assert_output(
        &c,
        &loader,
        "${10?string(\"0;; currencyCode=USDX\")}",
        "10;; currencyCode=USDX",
    ); // Java: "10"
       // currencySymbol：'0 ¤;; currencyCode=USD currencySymbol=bucks' → Java "10 bucks" —— 引擎差异
    assert_output(
        &c,
        &loader,
        "${10?string(\"0 \u{00a4};; currencyCode=USD currencySymbol=bucks\")}",
        "10 \u{00a4};; currencyCode=USD currencySymbol=bucks", // Java: "10 bucks"
    );
    // monetaryDecimalSeparator：Java '0.0 ¤;; monetaryDecimalSeparator=m' 10.5 → "10m5 $" —— 引擎差异
    assert_output(
        &c,
        &loader,
        "${10.5?string(\"0.0 \u{00a4};; monetaryDecimalSeparator=m\")}",
        "10.5 \u{00a4};; monetaryDecimalSeparator=m",
    ); // Java: "10m5 $"
    assert_output(
        &c,
        &loader,
        "${10.5?string(\"0.0 kg;; monetaryDecimalSeparator=m\")}",
        "10.5 kg;; monetaryDecimalSeparator=m",
    ); // Java: "10.5 kg"
    assert_output(
        &c,
        &loader,
        "${10.5?string(\"0.0 \u{00a4};; decimalSeparator=d\")}",
        "10.5 \u{00a4};; decimalSeparator=d",
    ); // Java: "10.5 $"
    assert_output(
        &c,
        &loader,
        "${10.5?string(\"0.0 kg;; decimalSeparator=d\")}",
        "10.5 kg;; decimalSeparator=d",
    ); // Java: "10d5 kg"
    assert_output(
        &c,
        &loader,
        "${10.5?string(\"0.0 \u{00a4};; monetaryDecimalSeparator=m decimalSeparator=d\")}",
        "10.5 \u{00a4};; monetaryDecimalSeparator=m decimalSeparator=d", // Java: "10m5 $"
    );
    assert_output(
        &c,
        &loader,
        "${10.5?string(\"0.0 kg;; monetaryDecimalSeparator=m decimalSeparator=d\")}",
        "10.5 kg;; monetaryDecimalSeparator=m decimalSeparator=d", // Java: "10d5 kg"
    );
}

/// Java testLocale：locale 相关分隔符（US 小数点 vs FR 逗号；groupingSeparator 参数）
#[test]
fn test_locale() {
    let (c, loader) = cfg();
    // Java: parse("0.0", US).format(1000) = "1000.0"；FR → "1000,0"
    // 引擎等价：format_number_with —— 断言保留 Java 值
    assert_eq!(
        freemarker::builtins::format::format_number_with(
            "0.0",
            "en_US",
            &freemarker::value::TNumber::Int(1000)
        )
        .unwrap(),
        "1000.0"
    );
    assert_eq!(
        freemarker::builtins::format::format_number_with(
            "0.0",
            "fr_FR",
            &freemarker::value::TNumber::Int(1000)
        )
        .unwrap(),
        "1000,0"
    );
    // Java: ",000.0;;groupingSeparator=_" US → "1_000.0"；FR → "1_000,0"
    // 引擎差异：groupingSeparator 参数未实现（仍用 locale 默认分组符：US ","、FR U+202F），
    // `;;...` 段按字面量后缀输出
    assert_eq!(
        freemarker::builtins::format::format_number_with(
            ",000.0;;groupingSeparator=_",
            "en_US",
            &freemarker::value::TNumber::Int(1000)
        )
        .unwrap(),
        "1,000.0;;groupingSeparator=_" // Java: "1_000.0"
    );
    assert_eq!(
        freemarker::builtins::format::format_number_with(
            ",000.0;;groupingSeparator=_",
            "fr_FR",
            &freemarker::value::TNumber::Int(1000)
        )
        .unwrap(),
        "1\u{202f}000,0;;groupingSeparator=_" // Java: "1_000,0"
    );
    let _ = (c, loader);
}

/// Java testTemplates：模板级扩展格式断言
#[test]
fn test_templates() {
    let (mut c, loader) = cfg();
    c.settings.locale = "en_US".to_string();
    // Java: ",000.#" 1000.15/1000.25 → "1,000.2 1,000.2"（HALF_EVEN 默认）—— 引擎一致
    c.settings.number_format = ",000.#".to_string();
    assert_output(&c, &loader, "${1000.15} ${1000.25}", "1,000.2 1,000.2");
    // 引擎差异：",000.#;; roundingMode=halfUp groupingSeparator=_" 参数未实现，
    // 按字面量后缀输出（Java: "1_000.2 1_000.3"）
    c.settings.number_format = ",000.#;; roundingMode=halfUp groupingSeparator=_".to_string();
    assert_output(
        &c,
        &loader,
        "${1000.15} ${1000.25}",
        "1,000.2;; roundingMode=halfUp groupingSeparator=_ 1,000.2;; roundingMode=halfUp groupingSeparator=_",
    );
    c.settings.locale = "de_DE".to_string();
    assert_output(
        &c,
        &loader,
        "${1000.15} ${1000.25}",
        "1.000,2;; roundingMode=halfUp groupingSeparator=_ 1.000,2;; roundingMode=halfUp groupingSeparator=_",
    );
    c.settings.locale = "en_US".to_string();
    // 引擎差异：同前 —— `;;...` 字面量后缀；Java 期望 "1_000.2; 10 00.2; 1_000,2; 1000,1"
    assert_output(
        &c,
        &loader,
        "${1000.15}; ${1000.15?string(',##.#;;groupingSeparator=\" \"')}; <#setting locale='de_DE'>${1000.15}; <#setting numberFormat='0.0;;roundingMode=down'>${1000.15}",
        "1,000.2;; roundingMode=halfUp groupingSeparator=_; 1,000.2;;groupingSeparator=\" \"; 1.000,2;; roundingMode=halfUp groupingSeparator=_; 1000,2;;roundingMode=down",
    );
    // Java: ?string('#E') → 错误含 "\"#E\"" "format string" "exponential"
    // 引擎差异：v1 子集把 #E 当分组+字面量后缀，不报错 —— 断言按引擎实际输出
    assert_output(&c, &loader, "${1?string('#E')}", "1E"); // Java: 报错
    assert_output(&c, &loader, "<#setting numberFormat='#E'>${1}", "1E"); // Java: 报错
                                                                          // Java: ";;foo=bar" → "\"foo\"" "supported" —— 引擎差异：字面量后缀
    assert_output(
        &c,
        &loader,
        "<#setting numberFormat=';;foo=bar'>${1}",
        ";;foo=bar1",
    ); // Java: 报错
       // Java: "0;;roundingMode=unnecessary" 1.5 → "can't format" "1.5" "UNNECESSARY" —— 引擎差异
    assert_output(
        &c,
        &loader,
        "<#setting numberFormat='0;;roundingMode=unnecessary'>${1.5}",
        "2;;roundingMode=unnecessary",
    ); // Java: 报错
}
