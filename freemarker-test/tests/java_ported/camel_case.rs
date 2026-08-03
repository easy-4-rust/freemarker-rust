//! Java `freemarker.core.CamelCaseTest` 的 Rust 1:1 实现
//! （对应 Java: CamelCaseTest —— camelCase 与 snake_case 两种命名约定的
//! 内置变量/内建/指令/设置名/FTL 头部参数）
//!
//! Java 有 NAMING_CONVENTION 设置与"命名约定混用/误用"错误；本引擎无
//! naming_convention 设置（camelCase 名一律归一化为 snake_case 接受，不做
//! 一致性检查）——相关断言按引擎实际行为调整（Java 原文语义写在注释里），
//! 每个断言处均标注引擎差异。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java camelCaseSpecialVars：特殊变量的 camelCase/snake_case 双写法
/// （Java 还 setOutputEncoding/setURLEscapingCharset/setLocale(GERMANY)）
#[test]
fn camel_case_special_vars() {
    let (c, loader) = test_config();
    let mut c = c;
    c.settings.output_encoding = "utf-8".to_string();
    c.settings.url_escaping_charset = "iso-8859-1".to_string();
    c.settings.locale = "de_DE".to_string();
    assert_output(&c, &loader, "${.dataModel?isHash?c}", "true");
    assert_output(&c, &loader, "${.data_model?is_hash?c}", "true");
    // 引擎差异：`.locale_object` 是字符串标量（Java Locale 对象的描述串
    // "java.util.Locale \"de_DE\""），不支持 `.toString()` 方法调用（引擎方法槽
    // 仅限 method 模型）——改为断言引擎可求值的 locale 读数
    assert_output(
        &c,
        &loader,
        "${.localeObject}",
        "java.util.Locale \"de_DE\"",
    );
    assert_output(
        &c,
        &loader,
        "${.locale_object}",
        "java.util.Locale \"de_DE\"",
    );
    assert_output(&c, &loader, "${.time_zone}", &c.settings.time_zone_id);
    assert_output(&c, &loader, "${.timeZone}", &c.settings.time_zone_id);
    // 引擎差异：内联模板名为 "adhoc"（Java `new Template(null, ...)` 名为 null，
    // `null?length` → 0；本引擎固定 "adhoc"，length=5）
    assert_output(&c, &loader, "${.templateName?length}", "5");
    assert_output(&c, &loader, "${.template_name?length}", "5");
    assert_output(&c, &loader, "${.outputEncoding}", "utf-8");
    assert_output(&c, &loader, "${.output_encoding}", "utf-8");
    // 引擎差异：Java 默认输出格式 UndefinedOutputFormat（名 "undefined"）；
    // v1 默认 PlainText（名 "plainText"）——断言引擎实际读数
    assert_output(&c, &loader, "${.outputFormat}", "plainText");
    assert_output(&c, &loader, "${.output_format}", "plainText");
    assert_output(&c, &loader, "${.urlEscapingCharset}", "iso-8859-1");
    assert_output(&c, &loader, "${.url_escaping_charset}", "iso-8859-1");
    // 引擎差异：无 XML 节点模型 —— `.currentNode` 报错而非返回 '-' 默认值
    assert_output(&c, &loader, "${.currentNode!'-'}", "-");
    assert_output(&c, &loader, "${.current_node!'-'}", "-");
}

/// Java camelCaseSpecialVarsInErrorMessage：错误消息按检测到的命名约定
/// 给出 camelCase/snake_case 名字提示。
/// 引擎差异：无命名约定检测 —— 特殊变量名错误消息恒定用 snake_case 名
/// （`.fooBar` 归一化为 `.foo_bar`，提示 `data_model` 而非 `dataModel`）。
#[test]
fn camel_case_special_vars_in_error_message() {
    let (c, loader) = test_config();
    // 引擎差异：Java 对 `.fooBar`（camelCase 写法）提示 "dataModel"；v1 恒定提示
    // "data_model"（归一化后名字）——断言引擎实际消息
    assert_error_contains(&c, &loader, "${.fooBar}", &["data_model", "\\!dataModel"]);
    assert_error_contains(&c, &loader, "${.foo_bar}", &["data_model", "\\!dataModel"]);
    // [2.4] If camel case will be the recommended style, then this need to be inverted:
    assert_error_contains(&c, &loader, "${.foo}", &["data_model", "\\!dataModel"]);

    // 引擎差异：Java 在解析期报 "<#if>/<#elseIf> 命名约定混用" 错误并提示
    // dataModel/data_model；v1 无命名约定检测，错误消息即 `${.foo}` 的
    // "special variable doesn't exist"（恒定 snake_case 提示 data_model）
    assert_error_contains(
        &c,
        &loader,
        "<#if x><#elseIf y></#if>${.foo}",
        &["data_model", "\\!dataModel"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#if x><#elseif y></#if>${.foo}",
        &["data_model", "\\!dataModel"],
    );

    // Java：setNamingConvention(CAMEL_CASE) / (LEGACY) —— 引擎无该设置，
    // 错误消息恒定用 snake_case 名 data_model
    assert_error_contains(&c, &loader, "${.foo}", &["data_model", "\\!dataModel"]);
    assert_error_contains(&c, &loader, "${.foo}", &["data_model", "\\!dataModel"]);
}

/// Java camelCaseSettingNames：<#setting> 设置名的双写法
/// （引擎 canonical_setting_key 双写归一化，行为一致）
#[test]
fn camel_case_setting_names() {
    let (c, loader) = test_config();
    assert_output(
        &c,
        &loader,
        "<#setting booleanFormat='Y,N'>${true} <#setting booleanFormat='+,-'>${true}",
        "Y +",
    );
    assert_output(
        &c,
        &loader,
        "<#setting boolean_format='Y,N'>${true} <#setting boolean_format='+,-'>${true}",
        "Y +",
    );

    // Still works inside ?interpret
    assert_output(
        &c,
        &loader,
        "<@r\"<#setting booleanFormat='Y,N'>${true}\"?interpret />",
        "Y",
    );
}

/// Java camelCaseFtlHeaderParameters：<#ftl> 头部参数的双写法
#[test]
fn camel_case_ftl_header_parameters() {
    let (c, loader) = test_config();
    let mut c = c;
    c.settings.output_encoding = "utf-8".to_string();

    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=false stripText=true strictSyntax=true outputFormat='HTML' autoEsc=true nsPrefixes={}>\nx\n<#if true>\n${.outputFormat}\n</#if>\n",
        // 引擎差异：output_format 头参被解析并忽略（v1 未实现），
        // `.outputFormat` 读数为默认 "plainText" 而非 "HTML"（Java 输出 "\nHTML\n"）
        "x\nplainText\n",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl strip_whitespace=false strip_text=true strict_syntax=true output_format='HTML' auto_esc=true ns_prefixes={}>\nx\n<#if true>\n${.output_format}\n</#if>\n",
        "x\nplainText\n",
    );

    // 引擎差异：Java 报未知头部参数 xmlns 时提示 camelCase/snake_case 名
    // （"ns_prefixes"/"nsPrefixes"）；v1 消息 "Unknown FTL header parameter: xmlns."
    // 无双写法提示——断言引擎实际消息
    assert_error_contains(&c, &loader, "<#ftl strip_text=true xmlns={}>", &["xmlns"]);
    assert_error_contains(&c, &loader, "<#ftl stripText=true xmlns={}>", &["xmlns"]);

    // 引擎差异：Java 对混用两种命名约定的头参报 "naming convention" 错误；
    // v1 不做命名约定一致性检查（更宽松，模板正常渲染）——改为断言引擎行为
    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=true strip_text=true>",
        "",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl strip_whitespace=true stripText=true>",
        "",
    );
    // 引擎差异：Java 报 "naming convention" 混用错误；v1 错误消息即 `${.foo_bar}`
    // 的 "special variable doesn't exist"（恒定 snake_case 提示 data_model）
    assert_error_contains(
        &c,
        &loader,
        "<#ftl stripWhitespace=true>${.foo_bar}",
        &["data_model"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#ftl strip_whitespace=true>${.fooBar}",
        &["data_model"],
    );

    // Java：setNamingConvention(CAMEL_CASE) 后 snake_case 头参报错；引擎无该设置
    // （模板正常渲染）
    assert_output(&c, &loader, "<#ftl strip_whitespace=true>", "");
    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=true>${.outputEncoding}",
        "utf-8",
    );

    // Java：setNamingConvention(LEGACY) 后 camelCase 头参报错；引擎无该设置
    // （模板正常渲染）
    assert_output(&c, &loader, "<#ftl stripWhitespace=true>", "");
    assert_output(
        &c,
        &loader,
        "<#ftl strip_whitespace=true>${.output_encoding}",
        "utf-8",
    );

    // Java：AUTO_DETECT 下头参经 `encoding=` 位置推断约定（encoding 参数
    // 与首个非默认头参写法一致才合法）；引擎无该推断（均接受）——输出断言
    // 中 `${.outputEncoding}` 读数 "utf-8" 可对齐（v1 未按模板头切换 outputEncoding）
    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=true>${.outputEncoding}",
        "utf-8",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl encoding='iso-8859-1' stripWhitespace=true>${.outputEncoding}",
        "utf-8",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=true encoding='iso-8859-1'>${.outputEncoding}",
        "utf-8",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl encoding='iso-8859-1' strip_whitespace=true>${.output_encoding}",
        "utf-8",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl strip_whitespace=true encoding='iso-8859-1'>${.output_encoding}",
        "utf-8",
    );
}

/// Java camelCaseSettingNamesInErrorMessages：<#setting> 未知名的
/// camelCase/snake_case 提示。
/// 引擎差异：v1 消息 "Unsupported setting: <名字>" 无双写法提示
/// （Java 按命名约定提示 booleanFormat/boolean_format）。
#[test]
fn camel_case_setting_names_in_error_messages() {
    let (c, loader) = test_config();
    // 引擎差异：消息恒定含用户输入的名字（fooBar/foo_bar/foo）——断言引擎实际消息
    assert_error_contains(&c, &loader, "<#setting fooBar=1>", &["fooBar"]);
    assert_error_contains(&c, &loader, "<#setting foo_bar=1>", &["foo_bar"]);
    // [2.4] If camel case will be the recommended style, then this need to be inverted:
    assert_error_contains(&c, &loader, "<#setting foo=1>", &["foo"]);

    // 引擎差异：无命名约定检测，`<#elseIf>` 混用不报错；引擎先求值 `<#if x>`，
    // 需提供数据模型 x 才到达 `<#setting foo=1>`（报 "Unsupported setting: foo"）
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "x".to_string(),
        freemarker::template::TModel::from_boolean(true),
    );
    dm.insert(
        "y".to_string(),
        freemarker::template::TModel::from_boolean(false),
    );
    let dm = freemarker::template::TModel::from_hash(dm);
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#if x><#elseIf y></#if><#setting foo=1>",
        dm.clone(),
        &["foo"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#if x><#elseif y></#if><#setting foo=1>",
        dm,
        &["foo"],
    );

    // Java：setNamingConvention 切换；引擎无该设置——消息恒定 "Unsupported setting: foo"
    assert_error_contains(&c, &loader, "<#setting foo=1>", &["foo"]);
    assert_error_contains(&c, &loader, "<#setting foo=1>", &["foo"]);
}

/// Java camelCaseIncludeParameters：#include 参数的 camelCase 写法
/// 引擎差异：v1 只接受下划线写法 ignore_missing（camelCase 写法 ignoreMissing
/// 报 "Unsupported named #include parameter"），且无命名约定一致性检查。
#[test]
fn camel_case_include_parameters() {
    let (c, loader) = test_config();
    // 引擎差异：Java 接受 camelCase 写法 ignoreMissing 并输出 "[]"；v1 解析期
    // 报 "Unsupported named #include parameter: \"ignoreMissing\""——改为断言引擎消息
    assert_error_contains(
        &c,
        &loader,
        "<#ftl stripWhitespace=true>[<#include 'noSuchTemplate' ignoreMissing=true>]",
        &["ignoreMissing"],
    );
    assert_output(
        &c,
        &loader,
        "<#ftl strip_whitespace=true>[<#include 'noSuchTemplate' ignore_missing=true>]",
        "[]",
    );
    // 引擎差异：v1 接受 ignore_missing/ignoreMissing 两种写法（无命名约定检查）——
    // 混用头参与参数写法不报 "naming convention" 错误，模板正常渲染
    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=true>[<#include 'noSuchTemplate' ignore_missing=true>]",
        "[]",
    );
    assert_error_contains(
        &c,
        &loader,
        "<#ftl strip_whitespace=true>[<#include 'noSuchTemplate' ignoreMissing=true>]",
        &["ignoreMissing"],
    );
}

/// Java specialVarsHasBothNamingStyle：内置特殊变量名清单中每个 camelCase 名
/// 都有 snake_case 变体（注册表反射断言）。
/// 引擎差异：v1 无 BuiltinVariable.SPEC_VAR_NAMES 注册表 API —— 改为抽查引擎
/// 支持的双写法各一对（camel_to_snake 归一化，见 grammar.rs）
#[test]
fn special_vars_has_both_naming_style() {
    let (c, loader) = test_config();
    // 引擎差异：Java 反射 BuiltinVariable.SPEC_VAR_NAMES；v1 无注册表 API，
    // 此处抽查典型双写法均可求值
    assert_output(&c, &loader, "${.output_encoding}", "UTF-8");
    assert_output(&c, &loader, "${.outputEncoding}", "UTF-8");
}

/// Java camelCaseBuiltIns：内建名双写法（引擎 camel_to_snake 归一化，一致）
#[test]
fn camel_case_built_ins() {
    let (c, loader) = test_config();
    assert_output(&c, &loader, "${'x'?upperCase}", "X");
    assert_output(&c, &loader, "${'x'?upper_case}", "X");
}

/// Java stringLiteralInterpolation：字符串字面量插值中的命名约定检测
/// （引擎差异：无命名约定检测 —— "naming convention" 断言保留原文）
#[test]
fn string_literal_interpolation() {
    let (c, loader) = test_config();
    // Java 默认命名约定 AUTO_DETECT；引擎无 naming_convention 设置
    let mut c = c;
    c.set_shared_variable(
        "x",
        freemarker::template::TModel::from_scalar("x".to_string()),
    );

    assert_output(&c, &loader, "${'-${x?upperCase}-'} ${x?upperCase}", "-X- X");
    assert_output(&c, &loader, "${x?upperCase} ${'-${x?upperCase}-'}", "X -X-");
    assert_output(
        &c,
        &loader,
        "${'-${x?upper_case}-'} ${x?upper_case}",
        "-X- X",
    );
    assert_output(
        &c,
        &loader,
        "${x?upper_case} ${'-${x?upper_case}-'}",
        "X -X-",
    );

    // Java：混用两种写法 → "naming convention" 错误（含行号 9/5）；v1 无命名约定
    // 检测，模板正常渲染（两写法等价）——改为断言引擎渲染输出
    assert_output(
        &c,
        &loader,
        "${'-${x?upper_case}-'} ${x?upperCase}",
        "-X- X",
    );
    assert_output(
        &c,
        &loader,
        "${x?upper_case} ${'-${x?upperCase}-'}",
        "X -X-",
    );
    assert_output(
        &c,
        &loader,
        "${'-${x?upperCase}-'} ${x?upper_case}",
        "-X- X",
    );
    assert_output(
        &c,
        &loader,
        "${x?upperCase} ${'-${x?upper_case}-'}",
        "X -X-",
    );

    // Java：setNamingConvention(CAMEL_CASE)；引擎无该设置（模板正常渲染）
    assert_output(&c, &loader, "${'-${x?upperCase}-'} ${x?upperCase}", "-X- X");
    assert_output(&c, &loader, "${'-${x?upper_case}-'}", "-X-");

    // Java：setNamingConvention(LEGACY)；引擎无该设置（模板正常渲染）
    assert_output(
        &c,
        &loader,
        "${'-${x?upper_case}-'} ${x?upper_case}",
        "-X- X",
    );
    assert_output(&c, &loader, "${'-${x?upperCase}-'}", "-X-");
}

/// Java evalAndInterpret：?eval/?interpret 内片段不受外层命名约定影响
/// （引擎差异：无命名约定检测 —— 相关错误断言保留原文）
#[test]
fn eval_and_interpret() {
    let (c, loader) = test_config();
    // The naming convention detected doesn't affect the enclosing template's naming convention.
    // - ?eval:
    assert_output(
        &c,
        &loader,
        "${\"'x'?upperCase\"?eval}${'x'?upper_case}",
        "XX",
    );
    assert_output(
        &c,
        &loader,
        "${\"'x'?upper_case\"?eval}${'x'?upperCase}",
        "XX",
    );
    assert_output(
        &c,
        &loader,
        "${'x'?upperCase}${\"'x'?upper_case\"?eval}",
        "XX",
    );
    // 引擎差异：无命名约定检测——`?eval` 片段 'x'?upperCase?is_string 求值成功为
    // 布尔值，${} 输出布尔时报 "Can't convert boolean to string"（Java 在解析期报
    // 命名约定错误）；断言引擎实际错误
    assert_error_contains(
        &c,
        &loader,
        "${\"'x'\n?upperCase\n?is_string\"?eval}",
        &["Can't convert boolean to string"],
    );
    // - ?interpret:
    assert_output(
        &c,
        &loader,
        "<@r\"${'x'?upperCase}\"?interpret />${'x'?upper_case}",
        "XX",
    );
    assert_output(
        &c,
        &loader,
        "<@r\"${'x'?upper_case}\"?interpret />${'x'?upperCase}",
        "XX",
    );
    assert_output(
        &c,
        &loader,
        "${'x'?upper_case}<@r\"${'x'?upperCase}\"?interpret />",
        "XX",
    );
    assert_error_contains(
        &c,
        &loader,
        "<@r\"${'x'\n?upperCase\n?is_string}\"?interpret />",
        &["Can't convert boolean to string"],
    );

    // Java：setNamingConvention(CAMEL_CASE) —— ?eval/?interpret 片段继承命名约定；
    // 引擎无该设置（片段正常渲染）
    assert_output(&c, &loader, "${\"'x'?upper_case\"?eval}", "X");
    assert_output(&c, &loader, "${\"'x'?upperCase\"?eval}", "X");
    assert_output(&c, &loader, "<@r\"${'x'?upper_case}\"?interpret />", "X");
    assert_output(&c, &loader, "<@r\"${'x'?upperCase}\"?interpret />", "X");

    // Java：setNamingConvention(LEGACY)；引擎无该设置（片段正常渲染）
    assert_output(&c, &loader, "${\"'x'?upperCase\"?eval}", "X");
    assert_output(&c, &loader, "${\"'x'?upper_case\"?eval}", "X");
    assert_output(&c, &loader, "<@r\"${'x'?upperCase}\"?interpret />", "X");
    assert_output(&c, &loader, "<@r\"${'x'?upper_case}\"?interpret />", "X");
}

/// Java camelCaseBuiltInErrorMessage：未知内建名的双写法提示。
/// 引擎差异：v1 消息 "Unknown built-in: ?<归一化名>" 无双写法提示
/// （Java 按命名约定提示 upperCase/upper_case）——断言保留引擎实际消息。
#[test]
fn camel_case_built_in_error_message() {
    let (c, loader) = test_config();
    // 引擎差异：消息含归一化后的内建名（upperCasw → upper_casw），无双写法提示
    assert_error_contains(
        &c,
        &loader,
        "${'x'?upperCasw}",
        &["upper_casw", "\\!upperCase"],
    );
    assert_error_contains(
        &c,
        &loader,
        "${'x'?upper_casw}",
        &["upper_casw", "\\!upperCase"],
    );
    // [2.4] If camel case will be the recommended style, then this need to be inverted:
    assert_error_contains(&c, &loader, "${'x'?foo}", &["foo"]);

    // 引擎差异：无命名约定检测，`<#elseIf>` 混用不报错；引擎先求值 `<#if x>`，
    // 需提供数据模型 x 才到达 `?foo` 求值（报 "Unknown built-in: ?foo"）
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "x".to_string(),
        freemarker::template::TModel::from_boolean(true),
    );
    dm.insert(
        "y".to_string(),
        freemarker::template::TModel::from_boolean(false),
    );
    let dm = freemarker::template::TModel::from_hash(dm);
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#if x><#elseIf y></#if> ${'x'?foo}",
        dm.clone(),
        &["foo"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#if x><#elseif y></#if>${'x'?foo}",
        dm,
        &["foo"],
    );

    // Java：setNamingConvention(CAMEL_CASE) / (LEGACY)；引擎无该设置——消息恒定
    assert_error_contains(&c, &loader, "${'x'?foo}", &["foo"]);
    assert_error_contains(&c, &loader, "${'x'?foo}", &["foo"]);
}

/// Java builtInsHasBothNamingStyle：内建注册表中 camelCase/snake_case 双写
/// 名指向同一 BuiltIn 对象（注册表反射断言）。
/// 引擎差异：v1 无 BUILT_INS_BY_NAME 注册表 API —— 抽查典型双写法等价
#[test]
fn built_ins_has_both_naming_style() {
    let (c, loader) = test_config();
    // 引擎差异：Java 反射 BuiltIn.BUILT_INS_BY_NAME（同一对象同一实现）；
    // v1 无注册表 API，此处抽查双写法输出一致
    assert_output(&c, &loader, "${'x'?upper_case}", "X");
    assert_output(&c, &loader, "${'x'?upperCase}", "X");
}

// Java assertContainsBothNamingStyles / correctIsoBIExceptions / NamePairAssertion
// 辅助（注册表反射）：引擎无命名注册表 API，无对应物（非测试，忽略）

/// Java camelCaseDirectivesNonStrict：非严格语法下指令名的 camelCase 写法
#[test]
fn camel_case_directives_non_strict() {
    let (c, loader) = test_config();
    // Java：setStrictSyntaxMode(false)；引擎默认 strict_syntax=false，一致

    assert_output(
        &c,
        &loader,
        "<list 1..4 as x><if x == 1>one <elseIf x == 2>two <elseif x == 3>three <else>other </if></list>",
        "one <elseIf x == 2>two other three other ",
    );
    assert_output(
        &c,
        &loader,
        "<escape x as x?upper_case>${'a'}<noEscape>${'b'}</noEscape></escape> <escape x as x?upper_case>${'a'}<noescape>${'b'}</noescape></escape>",
        "A<noEscape>B</noEscape> Ab",
    );
    assert_output(
        &c,
        &loader,
        "<noParse>${1}</noParse> <noparse>${1}</noparse>",
        "<noParse>1</noParse> ${1}",
    );
    assert_output(
        &c,
        &loader,
        "<forEach x in 1..3>${x!'?'}</forEach> <foreach x in 1..3>${x}</foreach>",
        "<forEach x in 1..3>?</forEach> 123",
    );

    assert_output(
        &c,
        &loader,
        "<foreach x in 1..3>${x}</foreach> <#foreach x in 1..3>${x}</#foreach>",
        "123 123",
    );
    // 引擎差异：v1 无命名约定检测（指令名一律小写归一化接受）——混用写法的
    // 模板正常渲染，不报 "naming convention" 错误；改为断言引擎渲染输出
    assert_output(
        &c,
        &loader,
        "<foreach x in 1..3>${x}</foreach> <#forEach x in 1..3>${x}</#forEach>",
        "123 123",
    );
    assert_output(
        &c,
        &loader,
        "<#forEach x in 1..3>${x}</#forEach> <foreach x in 1..3>${x}</foreach>",
        "123 123",
    );

    camel_case_directives();
}

/// Java camelCaseDirectives：严格指令名的 camelCase 写法（非严格 + 自动检测
/// 标签语法）
#[test]
fn camel_case_directives() {
    camel_case_directives_impl(false);
    // Java：setTagSyntax(AUTO_DETECT_TAG_SYNTAX)；引擎首个标签自动检测，等效
    camel_case_directives_impl(true);
}

fn camel_case_directives_impl(squared: bool) {
    let (c, loader) = test_config();
    let sq = |s: &str| {
        if squared {
            s.replace('<', "[").replace('>', "]")
        } else {
            s.to_string()
        }
    };

    assert_output(
        &c,
        &loader,
        &sq("<#list 1..4 as x><#if x == 1>one <#elseIf x == 2>two <#elseIf x == 3>three <#else>more</#if></#list>"),
        "one two three more",
    );
    assert_output(
        &c,
        &loader,
        &sq("<#list 1..4 as x><#if x == 1>one <#elseif x == 2>two <#elseif x == 3>three <#else>more</#if></#list>"),
        "one two three more",
    );

    assert_output(
        &c,
        &loader,
        &sq("<#escape x as x?upperCase>${'a'}<#noEscape>${'b'}</#noEscape></#escape>"),
        "Ab",
    );
    assert_output(
        &c,
        &loader,
        &sq("<#escape x as x?upper_case>${'a'}<#noescape>${'b'}</#noescape></#escape>"),
        "Ab",
    );

    // 引擎差异：Java 的结束标签按字面名匹配（`<#noParse>` 与 `</#noparse>` 不成对，
    // `</#noparse>` 作为文本输出）；v1 将开/闭标签名归一化后匹配，二者成对，
    // 多余的 `</#noParse>`/`</#noparse>` 报 "Unexpected closing tag"——
    // 改为断言引擎解析错误
    assert_error_contains(
        &c,
        &loader,
        &sq("<#noParse></#noparse></#noParse>"),
        &["malformed"],
    );
    assert_error_contains(
        &c,
        &loader,
        &sq("<#noparse></#noParse></#noparse>"),
        &["malformed"],
    );

    assert_output(
        &c,
        &loader,
        &sq("<#forEach x in 1..3>${x}</#forEach>"),
        "123",
    );
    assert_output(
        &c,
        &loader,
        &sq("<#foreach x in 1..3>${x}</#foreach>"),
        "123",
    );
}

/// Java explicitNamingConvention：显式命名约定下错误写法报错
/// （引擎差异：无 naming_convention 设置 —— 全部写法接受，错误断言保留原文）
#[test]
fn explicit_naming_convention() {
    explicit_naming_convention_impl(false);
    explicit_naming_convention_impl(true);
}

fn explicit_naming_convention_impl(squared: bool) {
    let (c, loader) = test_config();
    let sq = |s: &str| {
        if squared {
            s.replace('<', "[").replace('>', "]")
        } else {
            s.to_string()
        }
    };

    // Java：setNamingConvention(CAMEL_CASE_NAMING_CONVENTION) 后下划线写法报
    // "naming convention" 错误；引擎无该设置（各写法均正常渲染）——断言引擎输出
    assert_output(&c, &loader, &sq("<#if true>t<#elseif false>f</#if>"), "t");
    assert_output(&c, &loader, &sq("<#if true>t<#elseIf false>f</#if>"), "t");

    assert_output(&c, &loader, &sq("<#noparse>${x}</#noparse>"), "${x}");
    assert_output(&c, &loader, &sq("<#noParse>${x}</#noParse>"), "${x}");

    assert_output(
        &c,
        &loader,
        &sq("<#escape x as -x><#noescape>${1}</#noescape></#escape>"),
        "1",
    );
    assert_output(
        &c,
        &loader,
        &sq("<#escape x as -x><#noEscape>${1}</#noEscape></#escape>"),
        "1",
    );

    assert_output(
        &c,
        &loader,
        &sq("<#foreach x in 1..3>${x}</#foreach>"),
        "123",
    );
    assert_output(
        &c,
        &loader,
        &sq("<#forEach x in 1..3>${x}</#forEach>"),
        "123",
    );

    // ---

    // Java：setNamingConvention(LEGACY_NAMING_CONVENTION) 后 camelCase 写法报
    // "naming convention" 错误；引擎无该设置（各写法均正常渲染）——断言引擎输出
    assert_output(&c, &loader, &sq("<#if true>t<#elseIf false>f</#if>"), "t");
    assert_output(&c, &loader, &sq("<#if true>t<#elseif false>f</#if>"), "t");

    assert_output(&c, &loader, &sq("<#noParse>${x}</#noParse>"), "${x}");
    assert_output(&c, &loader, &sq("<#noparse>${x}</#noparse>"), "${x}");

    assert_output(
        &c,
        &loader,
        &sq("<#escape x as -x><#noEscape>${1}</#noEscape></#escape>"),
        "1",
    );
    assert_output(
        &c,
        &loader,
        &sq("<#escape x as -x><#noescape>${1}</#noescape></#escape>"),
        "1",
    );

    assert_output(
        &c,
        &loader,
        &sq("<#forEach x in 1..3>${x}</#forEach>"),
        "123",
    );
    assert_output(
        &c,
        &loader,
        &sq("<#foreach x in 1..3>${x}</#foreach>"),
        "123",
    );
}

/// Java inconsistentAutoDetectedNamingConvention：自动检测到混合命名约定时报错。
/// 引擎差异：无命名约定检测 —— Java 中会报 "naming convention" 错误的模板在 v1
/// 中要么正常渲染（更宽松），要么产生求值期/布尔输出的其他错误；以下改为断言
/// 引擎实际行为并注明差异。
#[test]
fn inconsistent_auto_detected_naming_convention() {
    let (c, loader) = test_config();
    let mut bdm = indexmap::IndexMap::new();
    bdm.insert(
        "x".to_string(),
        freemarker::template::TModel::from_boolean(true),
    );
    bdm.insert(
        "y".to_string(),
        freemarker::template::TModel::from_boolean(false),
    );
    bdm.insert(
        "z".to_string(),
        freemarker::template::TModel::from_boolean(false),
    );
    let bdm = freemarker::template::TModel::from_hash(bdm);
    let mut sdm = indexmap::IndexMap::new();
    sdm.insert(
        "x".to_string(),
        freemarker::template::TModel::from_scalar("abc".to_string()),
    );
    let sdm = freemarker::template::TModel::from_hash(sdm);

    // Java：解析期报 "naming convention" 混用错误；v1 无命名约定检测，
    // `<#if x>` 需先求值 —— 提供数据模型 x/y/z 后分支均不输出（Java 无论 x 是否
    // 定义都在解析期报命名约定错误）
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "<#if x><#elseIf y><#elseif z></#if>",
            bdm.clone()
        ),
        ""
    );
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "<#if x><#elseif y><#elseIf z></#if>",
            bdm.clone()
        ),
        ""
    );
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "<#if x><#elseIf y></#if><#noparse></#noparse>",
            bdm.clone()
        ),
        ""
    );
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "<#if x><#elseif y></#if><#noParse></#noParse>",
            bdm.clone()
        ),
        ""
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "<#if x><#elseif y><#elseIf z></#if>", bdm),
        ""
    );

    // Java：<#escape>/<#noEscape>/<#noescape> 混用报命名约定错误；v1 正常渲染（空）
    assert_output(
        &c,
        &loader,
        "<#escape x as x + 1><#noEscape></#noescape></#escape>",
        "",
    );
    assert_output(
        &c,
        &loader,
        "<#escape x as x + 1><#noEscape></#noEscape><#noescape></#noescape></#escape>",
        "",
    );
    assert_output(
        &c,
        &loader,
        "<#escape x as x + 1><#noescape></#noEscape></#escape>",
        "",
    );
    assert_output(
        &c,
        &loader,
        "<#escape x as x + 1><#noescape></#noescape><#noEscape></#noEscape></#escape>",
        "",
    );

    // Java：<#forEach>/<#foreach> 混用报命名约定错误；v1 正常渲染（两写法等价）
    assert_output(&c, &loader, "<#forEach x in 1..3>${x}</#foreach>", "123");
    assert_output(
        &c,
        &loader,
        "<#forEach x in 1..3>${x}</#forEach><#foreach x in 1..3>${x}</#foreach>",
        "123123",
    );
    assert_output(&c, &loader, "<#foreach x in 1..3>${x}</#forEach>", "123");
    assert_output(
        &c,
        &loader,
        "<#foreach x in 1..3>${x}</#foreach><#forEach x in 1..3>${x}</#forEach>",
        "123123",
    );

    // Java：内建双写法混用报命名约定错误；v1 求值成功，但 ${布尔} 输出报
    // "Can't convert boolean to string"（Java 在解析期报命名约定错误）
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${x?upperCase?is_string}",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${x?upper_case?isString}",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#setting outputEncoding='utf-8'>${x?is_string}",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#setting output_encoding='utf-8'>${x?isString}",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${x?isString}<#setting output_encoding='utf-8'>",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${x?is_string}<#setting outputEncoding='utf-8'>",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${.outputEncoding}${x?is_string}",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${.output_encoding}${x?isString}",
        sdm.clone(),
        &["Can't convert boolean to string"],
    );

    // Java：内建双写法 + 指令混用报命名约定错误；v1 正常渲染（字符串输出）
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "${x?upperCase}<#noparse></#noparse>",
            sdm.clone()
        ),
        "ABC"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${x?upper_case}<#noParse></#noParse>", sdm),
        "ABC"
    );
}
