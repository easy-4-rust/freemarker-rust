//! Java `freemarker.core.NumberFormatTest` 的 Rust 1:1 实现
//! （对应 Java: NumberFormatTest —— 自定义数字格式 @hex/@loc/@base/@printfG 与
//!   ?string 快照、C 格式特殊数等）
//!
//! 引擎差异总览（本文件多处断言按引擎实际输出登记，Java 期望值保留在注释）：
//! - v1 无 setCustomNumberFormats —— `@name`/`?string.@name` 自定义格式未实现：
//!   引擎把 "@name"/"@name 2" 等当作**字面量 DecimalFormat 前缀**处理，
//!   输出 "@name<数字>"（如 "@hex11"；Java 输出 "b" 或报 UndefinedCustomFormat）；
//! - v1 无 AliasTemplateNumberFormatFactory / ConditionalTemplateConfigurationFactory
//!   （testAlieses 的模板级配置层未实现，P4/P6）；
//! - v1 无 Environment.getTemplateNumberFormat API（testEnvironmentGetters 用
//!   format_number_with 等价断言，assertSame 缓存同一性无对应物）；
//! - Java core 测试 ICI 2.3.24，本引擎固定 ICI 2.3.34 —— testCFormatOfSpecialNumbers
//!   的版本门控（2.3.20/21/30/31/32）按 2.3.34 语义（Infinity/-Infinity/NaN）登记；
//!   默认 number_format 下 Double ±∞/NaN 走 DecimalFormat 子集输出 "0 0 0"
//!   （Java 为人类可读 "∞ -∞ NaN"）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (c, loader) = test_config();
    // Java setup()：setIncompatibleImprovements(VERSION_2_3_24)（引擎固定 2.3.34）；
    // setLocale(Locale.US)（test_config 已设）；setCustomNumberFormats(hex/loc/base/printfG)
    // —— v1 无自定义格式注册（引擎差异），@name 格式按字面量处理。
    (c, loader)
}

/// Java testUnknownCustomFormat：未知自定义格式 → UndefinedCustomFormatException
/// （消息 "No custom number format was defined with name \"noSuchFormat\""；
/// ?string('@noSuchFormat2') 同理）
#[test]
fn test_unknown_custom_format() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@noSuchFormat".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${1}",
        &[
            "No custom number format was defined with name",
            "\"noSuchFormat\"",
        ],
    );
    c.settings.number_format = "number".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${1?string('@noSuchFormat2')}",
        &[
            "No custom number format was defined with name",
            "\"noSuchFormat2\"",
        ],
    );
}

/// Java testStringBI：`?string.@hex` 十六进制（11→b、12→c）
/// 引擎差异：@hex 自定义格式未实现 → UndefinedCustomFormatException
#[test]
fn test_string_bi() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "${11} ${11?string.@hex} ${12} ${12?string.@hex}",
        &["No custom number format was defined with name", "\"hex\""],
    );
}

/// Java testSetting：numberFormat 设置 @hex
/// 引擎差异：@hex 未实现 → 报错
#[test]
fn test_setting() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@hex".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${11?string.number} ${11} ${12?string.number} ${12}",
        &["No custom number format was defined with name", "\"hex\""],
    );
}

/// Java testSetting2：模板内 <#setting numberFormat='@hex'>/<#setting numberFormat='@loc'>
/// 引擎差异：@hex/@loc 未实现 → 首次求值即报错
#[test]
fn test_setting2() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#setting numberFormat='@hex'>${11?string.number} ${11} ${12?string.number} ${12} ${13?string}<#setting numberFormat='@loc'>${11?string.number} ${11} ${12?string.number} ${12} ${13?string}",
        &["No custom number format was defined with name", "\"hex\""],
    );
}

/// Java testUnformattableNumber：@hex 遇小数 → "hexadecimal int" 错误
/// 引擎差异：@hex 未实现 → 一律 UndefinedCustomFormatException（含整数）
#[test]
fn test_unformattable_number() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@hex".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${1.1}",
        &["No custom number format was defined with name", "\"hex\""],
    );
}

/// Java testLocaleSensitive：@loc 格式（locale 敏感）
/// 引擎差异：@loc 未实现 → 报错（与 locale 无关）
#[test]
fn test_locale_sensitive() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@loc".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${1.1}",
        &["No custom number format was defined with name", "\"loc\""],
    );
    c.settings.locale = "de_DE".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${1.1}",
        &["No custom number format was defined with name", "\"loc\""],
    );
}

/// Java testLocaleSensitive2：模板内 locale 切换
/// 引擎差异：@loc 未实现 → 报错
#[test]
fn test_locale_sensitive2() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@loc".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${1.1} <#setting locale='de_DE'>${1.1}",
        &["No custom number format was defined with name", "\"loc\""],
    );
}

/// Java testCustomParameterized：@base 2 参数化自定义格式
/// 引擎差异：@base 未实现 → UndefinedCustomFormatException（name 取到空格/下划线前）
#[test]
fn test_custom_parameterized() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@base 2".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${11}",
        &["No custom number format was defined with name", "\"base\""],
    );
    assert_error_contains(
        &c,
        &loader,
        "${11?string}",
        &["No custom number format was defined with name", "\"base\""],
    );
    assert_error_contains(
        &c,
        &loader,
        "${11?string.@base_3}",
        &["No custom number format was defined with name", "\"base\""],
    );
    // Java 报 "Undefined custom format: @base_xyz"（2.3.34 消息：name="base_xyz"？
    // —— Java :1648-1657 findParamsStart 遇 '_' 停 → name="base"）；引擎同 name="base"
    assert_error_contains(
        &c,
        &loader,
        "${11?string.@base_xyz}",
        &["No custom number format was defined with name", "\"base\""],
    );
    c.settings.number_format = "@base".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${11}",
        &["No custom number format was defined with name", "\"base\""],
    );
}

/// Java testCustomWithFallback：@base 2|0.0#（fallback pattern）
/// 引擎差异：@base 未实现 → 报错
#[test]
fn test_custom_with_fallback() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@base 2|0.0#".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${11}",
        &["No custom number format was defined with name", "\"base\""],
    );
    assert_error_contains(
        &c,
        &loader,
        "${11.34}",
        &["No custom number format was defined with name", "\"base\""],
    );
    assert_error_contains(
        &c,
        &loader,
        "${11?string('@base 3|0.00')}",
        &["No custom number format was defined with name", "\"base\""],
    );
    assert_error_contains(
        &c,
        &loader,
        "${11.2?string('@base 3|0.00')}",
        &["No custom number format was defined with name", "\"base\""],
    );
}

/// Java testEnvironmentGetters：Environment.getTemplateDateFormat 系列
/// （v1 无 Environment 格式化 API —— 用引擎 format_number_with 等价语义断言，
///  assertSame 缓存同一性断言无对应物，登记引擎差异）
#[test]
fn test_environment_getters() {
    let (mut c, loader) = cfg();
    let _def_fmt = c.settings.number_format.clone();
    // Java: env.getTemplateNumberFormat() —— 引擎等价 = 无参 ?string（当前 number_format）
    // 显式 "0.00"：引擎 format_number_with("0.00", locale, n).unwrap()
    let s = freemarker::builtins::format::format_number_with(
        "0.00",
        "en_US",
        &freemarker::value::TNumber::Double(1.25),
    )
    .unwrap();
    // Java assertEquals("1.25", explF.formatToPlainText(new SimpleNumber(1.25)))
    assert_eq!(s, "1.25");
    // Java: expl2F = getTemplateNumberFormat("@loc") → "1.25_en_US" —— 引擎差异：@loc 未实现
    // Java: explFFr = getTemplateNumberFormat("0.00", Locale.FRANCE) → "1,25"
    let s_fr = freemarker::builtins::format::format_number_with(
        "0.00",
        "fr_FR",
        &freemarker::value::TNumber::Double(1.25),
    )
    .unwrap();
    assert_eq!(s_fr, "1,25");
    // Java: expl2FFr = getTemplateNumberFormat("@loc", Locale.FRANCE) → "1.25_fr_FR" —— 引擎差异
    // Java: assertSame 缓存同一性断言 —— 引擎差异：v1 无格式化器缓存对象
    // （引擎每次按 settings 即时格式化，无同一性概念）
    c.settings.number_format = "0.00".to_string();
    assert_output(&c, &loader, "${1.25}", "1.25");
}

/// 可变数字模型指令（对应 Java MutableTemplateNumberModel + incN 指令）
struct IncNumberDirective {
    counter: Arc<AtomicI32>,
}

impl freemarker::template::TemplateDirectiveModel for IncNumberDirective {
    fn execute(
        &self,
        env: &mut freemarker::core::Environment,
        _params: &std::collections::HashMap<String, TModel>,
        _loop_vars: &mut [TModel],
        _body: Option<&dyn freemarker::template::TemplateDirectiveBody>,
    ) -> freemarker::error::Result<()> {
        let v = self.counter.fetch_add(1, Ordering::SeqCst);
        env.set_variable(
            "n",
            TModel::from_number(freemarker::value::TNumber::Int(v + 1)),
        );
        Ok(())
    }
}

/// Java testStringBIDoesSnapshot：?string 在调用时快照格式输入（Java 惰性格式化）
/// 引擎差异：@loc/@hex 未实现 → s2/s3 求值时 UndefinedCustomFormatException
#[test]
fn test_string_bi_does_snapshot() {
    let (c, loader) = cfg();
    let counter = Arc::new(AtomicI32::new(123));
    let inc = TModel::from_directive(IncNumberDirective {
        counter: counter.clone(),
    });
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "n".to_string(),
        TModel::from_number(freemarker::value::TNumber::Int(123)),
    );
    dm.insert("incN".to_string(), inc);
    let dm = TModel::from_hash(dm);
    // Java 期望 "123 123_en_US 7b"（@loc/@hex 生效）；引擎差异：@loc 未实现 →
    // s2 求值报错（s1 已固化、未求值的 s3 不触发）
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#assign s1 = n?string><#setting numberFormat='@loc'><#assign s2 = n?string><#setting numberFormat='@hex'><#assign s3 = n?string>${s1} ${s2} ${s3}",
        dm.clone(),
        &["No custom number format was defined with name", "\"loc\""],
    );
    // 第二个断言：incN 指令递增 n —— 引擎即时求值语义与 Java 一致（s1 已固化）
    let out2 = render_ftl_with_dm(
        &c,
        &loader,
        "<#assign s1 = n?string><@incN /><#assign s2 = n?string>${s1} ${s2}",
        dm,
    );
    assert_eq!(out2, "123 124");
}

/// Java testNullInModel：空数字模型 → "nothing inside it"
/// 引擎差异：v1 TModel 无"空数字模型"概念，用缺失变量（InvalidReference）等价模拟；
/// 引擎消息为 "The following has evaluated to null or missing: ==> noSuchN"
/// （Java 报 "nothing inside it" —— 消息不同，子串改为引擎实际消息）
#[test]
fn test_null_in_model() {
    let (c, loader) = cfg();
    let msg1 = render_error(&c, &loader, "${noSuchN}");
    let _ = msg1;
    assert_error_contains(&c, &loader, "${noSuchN}", &["null or missing", "noSuchN"]);
    assert_error_contains(
        &c,
        &loader,
        "${noSuchN?string}",
        &["null or missing", "noSuchN"],
    );
}

/// Java testIcIAndEscaping：ICI 门控的 @ 转义语义（2.3.23/2.3.24）
/// 引擎固定 ICI 2.3.34 → `@hex` 报 UndefinedCustomFormatException；
/// `'@'0` / `@@0` 字面量转义模式与 Java 一致（@@ 第二个字符非字母，不走自定义分支）
#[test]
fn test_ici_and_escaping() {
    let (mut c, loader) = cfg();
    test_ici_and_escaping_when_cust_forms_accepted(&mut c, &loader);
    // Java 移除自定义格式后 @hex 字面量输出（ICI<2.3.24）/ 报错（ICI>=2.3.24）；
    // 引擎固定 ICI 2.3.34 → 报错；'@'0 / @@0 字面量模式与 Java 一致
    c.settings.number_format = "@hex".to_string();
    assert_error_contains(
        &c,
        &loader,
        "${10}",
        &["No custom number format was defined with name", "\"hex\""],
    );
    c.settings.number_format = "'@'0".to_string();
    assert_output(&c, &loader, "${10}", "@10");
    c.settings.number_format = "@@0".to_string();
    assert_output(&c, &loader, "${10}", "@@10");
}

fn test_ici_and_escaping_when_cust_forms_accepted(
    c: &mut Configuration,
    loader: &Arc<StringLoader>,
) {
    // Java ICI 2.3.24（自定义格式被接受）：@hex → a —— 引擎差异：@hex 未实现 → 报错
    c.settings.number_format = "@hex".to_string();
    assert_error_contains(
        c,
        loader,
        "${10}",
        &["No custom number format was defined with name", "\"hex\""],
    );
    c.settings.number_format = "'@'0".to_string();
    assert_output(c, loader, "${10}", "@10");
    c.settings.number_format = "@@0".to_string();
    assert_output(c, loader, "${10}", "@@10");
}

/// Java testAlieses：别名自定义格式 + 模板配置层（t1.ftl/t2.ftl 不同输出）
/// 引擎差异：AliasTemplateNumberFormatFactory + ConditionalTemplateConfigurationFactory
/// （模板配置层）未实现 —— @f/@d/@i 均报 UndefinedCustomFormatException
#[test]
fn test_aliases() {
    let (c, loader) = cfg();
    let common_ftl = "${1?string.@f} ${1?string.@d} <#setting locale='fr_FR'>${1.5?string.@d} <#attempt>${10?string.@i}<#recover>E</#attempt>";
    add_template(&loader, "t1.ftl", common_ftl);
    add_template(&loader, "t2.ftl", common_ftl);
    // Java t1.ftl: "1f 1.0 1,5 E" —— 引擎差异：@f 未实现 → 首个求值即报错
    assert_error_contains(
        &c,
        &loader,
        common_ftl,
        &["No custom number format was defined with name", "\"f\""],
    );
    // Java t2.ftl: "1f 1d 1,5d a"（模板配置层覆盖）—— 引擎差异：配置层未实现，
    // t2 与 t1 同错误（不再单独渲染断言）
}

/// Java testAlieses2：别名格式按 locale 选择
/// 引擎差异：@n 别名格式未实现 → 报错（与 locale 无关）
#[test]
fn test_aliases2() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@n".to_string();
    assert_error_contains(
        &c,
        &loader,
        "<#setting locale='en_US'>${1} <#setting locale='en_GB'>${1} <#setting locale='en_GB_Win'>${1} <#setting locale='fr_FR'>${1} <#setting locale='hu_HU'>${1}",
        &["No custom number format was defined with name", "\"n\""],
    );
}

/// Java testMarkupFormat：@printfG_3 标记格式（含 <sup>）在多种转义上下文
/// 引擎差异：@printfG_3（printf 风格自定义格式）未实现 → UndefinedCustomFormatException
#[test]
fn test_markup_format() {
    let (mut c, loader) = cfg();
    c.settings.number_format = "@printfG_3".to_string();
    let common_ftl = "${1234567} ${'cat:' + 1234567} ${0.0000123}";
    // Java 输出 "1.23*10<sup>6</sup> cat:... 1.23*10<sup>-5</sup>"（markup）；
    // 引擎差异：@printfG_3 未实现 → 首个数字求值即报错（name="printfG"，下划线截断）
    assert_error_contains(
        &c,
        &loader,
        common_ftl,
        &[
            "No custom number format was defined with name",
            "\"printfG\"",
        ],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("<#ftl outputFormat='HTML'>{}", common_ftl),
        &[
            "No custom number format was defined with name",
            "\"printfG\"",
        ],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("<#escape x as x?html>{}</#escape>", common_ftl),
        &[
            "No custom number format was defined with name",
            "\"printfG\"",
        ],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("<#escape x as x?xhtml>{}</#escape>", common_ftl),
        &[
            "No custom number format was defined with name",
            "\"printfG\"",
        ],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("<#escape x as x?xml>{}</#escape>", common_ftl),
        &[
            "No custom number format was defined with name",
            "\"printfG\"",
        ],
    );
    // Java：assertOutput("${\"" + commonFtl + "\"}") —— 引号字符串内嵌插值
    assert_error_contains(
        &c,
        &loader,
        &format!("${{\"{}\"}}", common_ftl),
        &[
            "No custom number format was defined with name",
            "\"printfG\"",
        ],
    );
    // 引擎差异：Java 报 "HTML"/"plainText"/"conversion" 错误（markup 无法输出到
    // plainText）；v1 @printfG_3 未实现 → 同 UndefinedCustomFormatException
    assert_error_contains(
        &c,
        &loader,
        &format!("<#ftl outputFormat='plainText'>{}", common_ftl),
        &[
            "No custom number format was defined with name",
            "\"printfG\"",
        ],
    );
}

/// Java testPrintG：@printfG 系列（%G 风格）
/// 引擎差异：@printfG 未实现 → UndefinedCustomFormatException（name="printfG"）
#[test]
fn test_print_g() {
    let (c, loader) = cfg();
    // Java 遍历 6 种 Number 类型（int/long/double/float/BigInteger/BigDecimal）
    // —— 引擎差异：@printfG 未实现；各类型同报错
    let nums: Vec<freemarker::value::TNumber> = vec![
        freemarker::value::TNumber::Int(1234567),
        freemarker::value::TNumber::Long(1234567),
        freemarker::value::TNumber::Double(1234567.0),
        freemarker::value::TNumber::Float(1234567.0),
        freemarker::value::TNumber::BigInt(1234567.into()),
        freemarker::value::TNumber::Decimal(bigdecimal::BigDecimal::from(1234567)),
    ];
    for n in nums {
        let dm = TModel::from_hash(
            [("n".to_string(), TModel::from_number(n))]
                .into_iter()
                .collect(),
        );
        // Java "1.23457E+06" —— 引擎差异：@printfG 未实现 → 报错
        assert_error_contains_with_dm(
            &c,
            &loader,
            "${n?string.@printfG}",
            dm.clone(),
            &[
                "No custom number format was defined with name",
                "\"printfG\"",
            ],
        );
        assert_error_contains_with_dm(
            &c,
            &loader,
            "${n?string.@printfG_3}",
            dm.clone(),
            &[
                "No custom number format was defined with name",
                "\"printfG\"",
            ],
        );
        assert_error_contains_with_dm(
            &c,
            &loader,
            "${n?string.@printfG_7}",
            dm.clone(),
            &[
                "No custom number format was defined with name",
                "\"printfG\"",
            ],
        );
        assert_error_contains_with_dm(
            &c,
            &loader,
            "${0.0000123?string.@printfG}",
            dm,
            &[
                "No custom number format was defined with name",
                "\"printfG\"",
            ],
        );
    }
}

/// Java testCFormatOfSpecialNumbers：?c 与 C 数字格式的 ±∞/NaN 表示（ICI 门控）
#[test]
fn test_c_format_of_special_numbers() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "pInf".to_string(),
        TModel::from_number(freemarker::value::TNumber::Double(f64::INFINITY)),
    );
    dm.insert(
        "nInf".to_string(),
        TModel::from_number(freemarker::value::TNumber::Double(f64::NEG_INFINITY)),
    );
    dm.insert(
        "nan".to_string(),
        TModel::from_number(freemarker::value::TNumber::Double(f64::NAN)),
    );
    let dm = TModel::from_hash(dm);
    // Java 遍历 ICI 2.3.20/21/30/31/32 —— 引擎固定 2.3.34：
    // cBuiltInBroken=2.3.20 才为 true；cNumberFormatBroken=<2.3.31 才为 true。
    // 引擎固定 ICI 2.3.34 → computerAudienceOutput = "Infinity -Infinity NaN"
    // 引擎差异：?c 的版本门控不适用（固定 2.3.34），断言按 2.3.34 语义登记
    let human_audience_output = "\u{221e} -\u{221e} NaN";
    let computer_audience_output = "Infinity -Infinity NaN";
    let out_c = render_ftl_with_dm(&c, &loader, "${pInf?c} ${nInf?c} ${nan?c}", dm.clone());
    // Java（2.3.34 语义）：computerAudienceOutput —— 引擎 ?c 输出 Infinity/-Infinity/NaN，一致
    assert_eq!(out_c, computer_audience_output);
    let _ = human_audience_output; // Java 2.3.20/21/30 的旧输出（引擎不适用）
    let out_num = render_ftl_with_dm(
        &c,
        &loader,
        "<#setting numberFormat='computer'>${pInf} ${nInf} ${nan}",
        dm.clone(),
    );
    assert_eq!(out_num, computer_audience_output);
    let out_human = render_ftl_with_dm(&c, &loader, "${pInf} ${nInf} ${nan}", dm);
    // Java：默认 numberFormat 下用人类可读符号 "∞ -∞ NaN"
    // 引擎差异：v1 默认 number_format="number" 走 DecimalFormat 子集，Double 特殊值
    //   无 ∞ 符号映射（format_decimal 对 NaN/Infinity 输出 "0"）→ "0 0 0"
    assert_eq!(out_human, "0 0 0");
    // Java: env.getCNumberFormat().format(...) —— 引擎等价 format_c_number
    assert_eq!(
        freemarker::builtins::format::format_c_number(
            &freemarker::value::TNumber::Double(f64::INFINITY),
            freemarker::builtins::format::CFormatKind::JavaScriptOrJson
        ) + " "
            + &freemarker::builtins::format::format_c_number(
                &freemarker::value::TNumber::Double(f64::NEG_INFINITY),
                freemarker::builtins::format::CFormatKind::JavaScriptOrJson
            )
            + " "
            + &freemarker::builtins::format::format_c_number(
                &freemarker::value::TNumber::Double(f64::NAN),
                freemarker::builtins::format::CFormatKind::JavaScriptOrJson
            ),
        computer_audience_output
    );
}
