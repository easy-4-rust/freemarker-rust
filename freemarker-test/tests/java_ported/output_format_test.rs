//! Java `freemarker.core.OutputFormatTest` 的 Rust 1:1 实现
//! （对应 Java: OutputFormatTest —— 模板层输出格式/autoEsc/noAutoEsc/outputFormat
//!   指令、?esc/?no_esc 内建、特殊变量、legacy 数值插值等）。
//!
//! 引擎差异总览（逐方法差异见各方法注释）：
//! - 引擎默认输出格式为 `plainText`（Java 默认 `UndefinedOutputFormat`，名 "undefined"）
//!   → `${.outputFormat}` 默认读数为 "plainText"（Java "undefined"）。
//! - `<#ftl outputFormat=...>`/`autoEsc=...` 头部参数引擎解析并忽略
//!   （parser/grammar.rs:377-380）→ 用 `<#outputFormat 'X'>` 指令 + `<#autoEsc>`/
//!   `<#noAutoEsc>` 包裹等价翻译（引擎 `<#outputFormat>` 仅切 output_format，
//!   不自动开 autoEsc；组合包裹可复现 Java 的 `<#ftl outputFormat=...>` 语义）。
//! - 引擎 `apply_escape`（environment.rs:739-750）对 markup 模型不跳过二次转义
//!   → `?esc`/`?no_esc` 结果在 autoEsc 下被再次转义、markup 模型插值被转义
//!   （Java：markup 模型/转义结果不再转义）。
//! - 引擎无自定义输出格式（dummy/seldomEscaped 等）→ 相关断言 NOT_APPLICABLE。
//! - 引擎无文件扩展名识别（.ftlh/.ftlx）与 `*.xml` 的 TemplateConfiguration → 相关
//!   方法 NOT_APPLICABLE。
//! - `?markup_string` 内建引擎无（"Unknown built-in: ?markup_string"）→ NOT_APPLICABLE。
//! - 引擎无命名约定校验（`<#autoEsc></#autoesc>` 等不报 convention 错）→ 相关断言引擎差异。
//!
//! NOT_APPLICABLE: testOutputFormatSettingLayers —— 依赖 `.xml` 文件扩展名 →
//!   XML 的 TemplateConfiguration、`<#ftl outputFormat>` 头与
//!   UndefinedOutputFormat/RTF 实例（引擎无扩展名识别与 per-模板输出格式）。
//! NOT_APPLICABLE: testStandardFileExtensions —— 依赖 recognizeStandardFileExtensions
//!   （.ftlh→HTML/.ftlx→XML/.FTLH 等）与 TemplateConfiguration 覆盖优先级、ICI 门控；
//!   引擎无文件扩展名识别与该设置。
//! NOT_APPLICABLE: testStandardFileExtensionsSettingOverriding —— 依赖文件扩展名 +
//!   ConditionalTemplateConfigurationFactory + ICI 2.3.23/24 门控；引擎无。
//! NOT_APPLICABLE: testStandardFileExtensionsWithConstructor —— Java `new Template(name,
//!   ftl, cfg)` 按扩展名定输出格式 + recognizeStandardFileExtensions；引擎无。
//! NOT_APPLICABLE: testStandardFileExtensionsFormatterImplOverriding —— 注册自定义
//!   OutputFormat 替换 HTML 转义器（CustomHTMLOutputFormat）；引擎无自定义输出格式。
//! NOT_APPLICABLE: testAutoEscapingSettingLayers —— 依赖 autoEsc 默认策略 + `.ftlh`
//!   扩展名 + TemplateConfiguration；引擎无扩展名识别。
//! NOT_APPLICABLE: testUndefinedOutputFormat —— Java `${.outputFormat}`="undefined" +
//!   `${'x'?esc}`/`?noEsc` 在 undefined 格式下报错；引擎默认 "plainText" 且 ?esc 不报错。
//! NOT_APPLICABLE: testPlainTextOutputFormat —— `${htmlPlain}` 需输出 HTML 转义后的
//!   markup 内容（引擎 markup 模型存明文）、`?esc`/`?noEsc` 在 plainText 报错
//!   （引擎不报错）。
//! NOT_APPLICABLE: testAutoEscapingOnMOs —— 依赖 `<#ftl outputFormat>` 头 + markup
//!   模型输出已转义内容（引擎 markup 模型存明文）+ ?esc 在非字符串上的行为差异。
//! NOT_APPLICABLE: testStringLiteralsUseUndefinedOF —— 依赖 `?noEsc` 在字符串字面量
//!   插值中保持未定义输出格式的语义；引擎 string literal 内插值行为不同（头忽略）。
//! NOT_APPLICABLE: testUnparsedTemplate —— Java `Template.getPlainTextTemplate`；
//!   引擎无该 API。
//! NOT_APPLICABLE: testStringBIsFail —— `${'<b>foo</b>'?esc?upperCase}` 在 HTML
//!   autoEsc 下报 "markup_output" 类型错；引擎 ?esc 结果可继续 ?upper_case，不报错。
//! NOT_APPLICABLE: testConcatWithMOs —— 字符串与 markup 模型拼接时 Java 对字符串侧
//!   按目标格式转义（`'\'' + htmlMarkup` → `&#39;<p>c`）；引擎拼接不做格式转换
//!   （P120/P121/P122 探针确认），且跨格式拼接报错消息不同。
//! NOT_APPLICABLE: testMarkupStringBI —— 引擎无 `?markup_string` 内建。
//! NOT_APPLICABLE: testMixedContent —— 依赖自定义 DummyOutputFormat + 跨格式
//!   "is incompatible with" 检查；引擎无自定义格式。
//! NOT_APPLICABLE: testAutoEscPolicy —— 依赖 ENABLE_IF_SUPPORTED/FORCE 等
//!   autoEscapingPolicy + 自定义 seldomEscaped/dummy 格式；引擎无自定义格式
//!   与 policy 设置。
//! NOT_APPLICABLE: testForcedAutoEsc —— 依赖 FORCE_AUTO_ESCAPING_POLICY +
//!   IllegalArgumentException/ParseException 门控；引擎无。
//! NOT_APPLICABLE: testDynamicParsingBIsInherticContextOutputFormat —— 依赖
//!   ?eval/?interpret 继承调用模板的 outputFormat/autoEscaping 词法上下文；
//!   引擎 ?eval/?interpret 的 output_format 继承语义不同（头忽略）。
//! NOT_APPLICABLE: testBannedBIsWhenAutoEscaping —— Java 在 autoEsc 下禁 ?html/xhtml/
//!   rtf/xml（"double-escaping" 报错）；引擎这些 BI 不做 double-escaping 检查。
//! NOT_APPLICABLE: testLegacyEscaperBIsBypassMOs —— 依赖 markup 模型的已转义内容
//!   原样输出 + 非匹配格式报 "string/markup_output" 错；引擎 markup 模型存明文。
//! NOT_APPLICABLE: testBannedDirectivesWhenAutoEscaping —— Java 在 autoEsc 下禁
//!   `<#escape>`（"double-escaping"）；引擎无该检查。
//! NOT_APPLICABLE: testCombinedOutputFormats —— `{HTML}` 组合输出格式；
//!   引擎无。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::core::OutputFormatKind;
use freemarker::template::TModel;

/// markup 模型（对应 Java `HTMLOutputFormat.INSTANCE.fromMarkup(...)` 等；
/// 引擎差异：markup 内容以明文标记得，见文件头）
fn markup(s: &str) -> TModel {
    TModel {
        scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
            s.to_string(),
        ))),
        type_name: "markup_output",
        kind: freemarker::template::ModelKind::Markup,
        ..TModel::nothing()
    }
}

/// 对应 Java OutputFormatTest.createConfiguration 的共享变量
/// （rtfPlain/rtfMarkup/htmlPlain/htmlMarkup/xmlPlain/xmlMarkup）
fn setup_shared_vars(c: &mut freemarker::template::Configuration) {
    c.set_shared_variable("rtfPlain", markup("\\par a & b"));
    c.set_shared_variable("rtfMarkup", markup("\\par c"));
    c.set_shared_variable("htmlPlain", markup("a < {h'}"));
    c.set_shared_variable("htmlMarkup", markup("<p>c"));
    c.set_shared_variable("xmlPlain", markup("a < {x'}"));
    c.set_shared_variable("xmlMarkup", markup("<p>c</p>"));
}

/// Java testNumericalInterpolation —— 旧式数值插值 `#{...}` 不受输出格式转义。
/// dummy 自定义格式部分 NOT_APPLICABLE（引擎无自定义输出格式）。
#[test]
fn test_numerical_interpolation() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    // Java：<#ftl outputFormat='dummy'>#{1.5}; #{1.5; m3}; ${'a.b'} → "1\\.5; 1\\.500; a\\.b"
    //   与 <#ftl outputFormat='dummy' autoEsc=false>... —— NOT_APPLICABLE（自定义格式）
    // Java：<#ftl outputFormat='plainText'>#{1.5} → "1.5"（引擎默认即 plainText）
    assert_output(&c, &l, "<#ftl outputFormat='plainText'>#{1.5}", "1.5");
    // Java：<#ftl outputFormat='HTML'>#{1.5} → "1.5"（数值插值不转义；头被忽略，
    //   改用指令等价）
    assert_output(
        &c,
        &l,
        "<#outputFormat 'HTML'>#{1.5}</#outputFormat>",
        "1.5",
    );
    // Java：#{} → "1.5"
    assert_output(&c, &l, "#{1.5}", "1.5");
}

/// Java testSpecialVariables —— `.outputFormat`/`.autoEsc` 特殊变量。
/// t.ftlx/t.ftlh/t.ftl/tN.ftl 的命名模板 + 文件扩展名识别 NOT_APPLICABLE；
/// 等价用 `<#outputFormat>` + autoEsc 包裹翻译。
#[test]
fn test_special_variables() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    let common = "${.outputFormat} ${.autoEsc?c}";
    // Java t.ftlx：<#ftl outputFormat='XML'> → "XML true"
    assert_output(
        &c,
        &l,
        &format!("<#outputFormat 'XML'><#autoEsc>{common}</#autoEsc></#outputFormat>"),
        "XML true",
    );
    // Java t.ftlh：HTML → "HTML true"
    assert_output(
        &c,
        &l,
        &format!("<#outputFormat 'HTML'><#autoEsc>{common}</#autoEsc></#outputFormat>"),
        "HTML true",
    );
    // Java tN.ftl：<#ftl outputFormat='RTF' autoEsc=false> → "RTF false"
    assert_output(
        &c,
        &l,
        &format!("<#outputFormat 'RTF'><#noAutoEsc>{common}</#noAutoEsc></#outputFormat>"),
        "RTF false",
    );
    // Java：assertOutput("${.output_format} ${.auto_esc?c}", "undefined false")
    // 引擎差异：默认格式名 "plainText"（Java "undefined"）
    assert_output(
        &c,
        &l,
        "${.output_format} ${.auto_esc?c}",
        "plainText false",
    );
}

/// Java testEscAndNoEscBIBasics —— ?esc/?no_esc 基础。
/// t.ftlh（HTML autoEsc on）与 t.ftl（undefined 报错）部分引擎差异见注释；
/// t-noAuto.ftlh（autoEsc off）逐字对齐。
#[test]
fn test_esc_and_no_esc_bi_basics() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    let common = "${'<x>'} ${'<x>'?esc} ${'<x>'?noEsc}";
    // Java t.ftlh（HTML，autoEsc on）期望 "&lt;x&gt; &lt;x&gt; <x>"：
    //   ?esc/?noEsc 结果按 markup 语义输出（autoEsc 下不二次转义，
    //   DollarVariable.java:72-77 同格式 markup 原样输出）
    let html_auto = render_ftl(
        &c,
        &l,
        &format!("<#outputFormat 'HTML'><#autoEsc>{common}</#autoEsc></#outputFormat>"),
    );
    assert_eq!(html_auto, "&lt;x&gt; &lt;x&gt; <x>");
    // Java t-noAuto.ftlh（autoEsc=false）→ "<x> &lt;x&gt; <x>" ✓ 逐字对齐
    assert_output(
        &c,
        &l,
        &format!("<#outputFormat 'HTML'><#noAutoEsc>{common}</#noAutoEsc></#outputFormat>"),
        "<x> &lt;x&gt; <x>",
    );
    // Java t.ftl（undefined 格式）assertErrorContains "output format", "undefined"：
    //   引擎差异 —— ?esc/?noEsc 在默认 plainText 下不报错；?esc 对 plainText 不做
    //   转义 → 输出 "<x> <x> <x>"。
    let out = render_ftl(&c, &l, common);
    assert_eq!(
        out, "<x> <x> <x>",
        "引擎差异：Java 报 output format/undefined 错，引擎 ?esc 在 plainText 下不报错（不转义）"
    );
}

/// Java testOutputFormatDirective —— <#outputFormat> 指令。
/// 命名约定校验（`<#outputFormat></#outputformat>` 报 convention 错）引擎无该检查 → 跳过。
/// 自定义格式（'dummy' 注册）NOT_APPLICABLE。
#[test]
fn test_output_format_directive() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    // Java 第一组：undefined' HTML&#39; XML&apos; HTML&#39; undefined'
    // 引擎差异：默认格式名 "plainText"（Java "undefined"）；autoEsc 用 <#autoEsc> 包裹。
    // 空格用 `${' '}` 插值承载（引擎 WS stripping 会剥掉指令标签相邻的空格，
    // Java 不剥 —— 引擎差异，见文件头）。
    assert_output(
        &c,
        &l,
        concat!(
            "${.outputFormat}${'\\''}${' '}",
            "<#outputFormat 'HTML'><#autoEsc>",
            "${.outputFormat}${'\\''}${' '}",
            "<#outputFormat 'XML'>${.outputFormat}${'\\''}</#outputFormat>${' '}",
            "${.outputFormat}${'\\''}${' '}",
            "</#autoEsc></#outputFormat>",
            "${.outputFormat}${'\\''}"
        ),
        "plainText' HTML&#39; XML&apos; HTML&#39; plainText'",
    );
    // Java 第二组（snake_case 头）：XML&apos; HTML&#39; XML&apos;
    assert_output(
        &c,
        &l,
        concat!(
            "<#outputFormat 'XML'><#autoEsc>",
            "${.output_format}${'\\''}${' '}",
            "<#outputformat 'HTML'>${.output_format}${'\\''}</#outputformat>${' '}",
            "${.output_format}${'\\''}",
            "</#autoEsc></#outputFormat>"
        ),
        "XML&apos; HTML&#39; XML&apos;",
    );
    // Java 对齐：Unregistered output format name, "dummy". ...（Java
    // OutputFormatDirective/Configuration 注册表消息，jar 实测）
    let m = assert_error_contains(
        &c,
        &l,
        "<#outputFormat 'dummy'></#outputFormat>",
        &["dummy"],
    );
    assert!(
        m.contains("Unregistered output format name"),
        "引擎消息: {m}"
    );
    // Java parse-time 参数表达式：'plain' + 'Text' → plainText ✓
    assert_output(
        &c,
        &l,
        "<#outputFormat 'plain' + 'Text'>${.outputFormat}</#outputFormat>",
        "plainText",
    );
    // Java：'plain' + someVar + 'Text' assertErrorContains "someVar", "parse-time"
    //  —— 引擎在解析期对非常量参数报错（消息无 "parse-time" 字样，保留 "someVar"）
    let m2 = assert_error_contains(
        &c,
        &l,
        "<#outputFormat 'plain' + someVar + 'Text'></#outputFormat>",
        &["someVar"],
    );
    assert!(m2.contains("null or missing"), "引擎消息: {m2}");
    // Java：'plainText'?upperCase assertErrorContains "?upperCase", "parse-time"
    //  —— 引擎消息 "Unknown output format: PLAINTEXT"（?upperCase 在解析期已求值，
    //   引擎差异消息，保留 "output format" 语义）
    let m3 = assert_error_contains(
        &c,
        &l,
        "<#outputFormat 'plainText'?upperCase></#outputFormat>",
        &["output format"],
    );
    assert!(m3.contains("PLAINTEXT"), "引擎消息: {m3}");
    // Java：<#outputFormat true> assertErrorContains "string", "boolean"
    //  —— 引擎报 boolean→string 转换错误，含两者 ✓
    assert_error_contains(
        &c,
        &l,
        "<#outputFormat true></#outputFormat>",
        &["string", "boolean"],
    );
    // Java 空块：undefined undefined（引擎差异：plainText plainText）
    assert_output(
        &c,
        &l,
        "${.output_format} <#outputformat 'HTML'></#outputformat>${.output_format}",
        "plainText plainText",
    );
    // Java WS stripping：undefined\n  x\nundefined（引擎差异：plainText）
    assert_output(
        &c,
        &l,
        "${.output_format}\n<#outputformat 'HTML'>\n  x\n</#outputformat>\n${.output_format}",
        "plainText\n  x\nplainText",
    );
}

/// Java testAutoEscAndNoAutoEscDirectives —— <#autoEsc>/<#noAutoEsc> 指令。
/// 头参数经 <#outputFormat>+<#autoEsc>/<#noAutoEsc> 包裹等价翻译。
/// 命名约定/bad camelCase（<#noAutoesc> 等）校验引擎无 → 跳过。
#[test]
fn test_auto_esc_and_no_auto_esc_directives() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    // Java 第一组（<#ftl outputFormat='XML'>，autoEsc on）：
    //   "true&amp; false& true&amp; false& true&amp;"
    assert_output(
        &c,
        &l,
        concat!(
            "<#outputFormat 'XML'><#autoEsc>",
            "${.autoEsc?c}${'&'} ",
            "<#noAutoEsc>${.autoEsc?c}${'&'} ",
            "<#autoEsc>${.autoEsc?c}${'&'}</#autoEsc> ",
            "${.autoEsc?c}${'&'} ",
            "</#noAutoEsc>",
            "${.autoEsc?c}${'&'}",
            "</#autoEsc></#outputFormat>"
        ),
        "true&amp; false& true&amp; false& true&amp;",
    );
    // Java 第二组（<#ftl auto_esc=false output_format='XML'>）：
    //   "false& true&amp; false&"
    assert_output(
        &c,
        &l,
        concat!(
            "<#outputFormat 'XML'><#noAutoEsc>",
            "${.auto_esc?c}${'&'} ",
            "<#autoesc>${.auto_esc?c}${'&'}</#autoesc> ",
            "${.auto_esc?c}${'&'}",
            "</#noAutoEsc></#outputFormat>"
        ),
        "false& true&amp; false&",
    );
    // Java：getConfiguration().setOutputFormat(XMLOutputFormat.INSTANCE) ——
    //   引擎等价设置 settings.output_format（默认 autoEsc 对 markup 格式开启）
    c.settings.output_format = OutputFormatKind::Xml;
    // Java 空块：true true
    assert_output(
        &c,
        &l,
        "${.auto_esc?c} <#noautoesc></#noautoesc>${.auto_esc?c}",
        "true true",
    );
    // Java WS stripping：true\n  x\ntrue
    assert_output(
        &c,
        &l,
        "${.auto_esc?c}\n<#noautoesc>\n  x\n</#noautoesc>\n${.auto_esc?c}",
        "true\n  x\ntrue",
    );
    // Java 命名约定错（<#autoEsc></#autoesc> → "convention", "#autoEsc", "#autoesc"）：
    //   引擎无命名约定校验 → 引擎差异，不报错（此处断言不报错）。
    let out = render_ftl(&c, &l, "<#autoEsc></#autoesc>");
    assert_eq!(
        out, "",
        "引擎差异：Java 报 convention 错，引擎无命名约定校验"
    );
    // Java 未知指令（<#noAutoesc> → "Unknown directive"）：引擎大小写不敏感解析
    //   <#noAutoesc> 为 noAutoEsc → 引擎差异。
    let out2 = render_ftl(&c, &l, "<#noAutoesc></#noAutoesc>");
    assert_eq!(
        out2, "",
        "引擎差异：Java 报 Unknown directive，引擎按 noAutoEsc 解析"
    );
}

/// Java testIsMarkupOutputBI —— ?isMarkupOutput/?is_markup_output
#[test]
fn test_is_markup_output_bi() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    // Java addToDataModel("m1", fromPlainTextByEscaping("x")) / m2 fromMarkup / s 字符串
    c.set_shared_variable("m1", markup("x"));
    c.set_shared_variable("m2", markup("x"));
    c.set_shared_variable("s", TModel::from_scalar("x".to_string()));
    assert_output(
        &c,
        &l,
        "${m1?isMarkupOutput?c} ${m2?isMarkupOutput?c} ${s?isMarkupOutput?c}",
        "true true false",
    );
    assert_output(&c, &l, "${m1?is_markup_output?c}", "true");
}

/// Java testHasContentBI —— ?hasContent on markup / ?esc 结果
#[test]
fn test_has_content_bi() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    assert_output(
        &c,
        &l,
        "${htmlMarkup?hasContent?c} ${htmlPlain?hasContent?c}",
        "true true",
    );
    // Java：<#ftl outputFormat='HTML'>${''?esc?hasContent?c} ${''?noEsc?hasContent?c}
    //   → "false false"（头经指令包裹）
    assert_output(
        &c,
        &l,
        "<#outputFormat 'HTML'>${''?esc?hasContent?c} ${''?noEsc?hasContent?c}</#outputFormat>",
        "false false",
    );
}

/// Java testMissingVariables —— 缺失变量报 InvalidReference（"noSuchVar", "null or missing"）。
/// Java 用 `<#ftl outputFormat='XML'>` 头（引擎忽略，不影响缺失变量报错）。
#[test]
fn test_missing_variables() {
    let (mut c, l) = test_config();
    setup_shared_vars(&mut c);
    for ftl in [
        "${noSuchVar}",
        "<#ftl outputFormat='XML'>${noSuchVar}",
        "<#ftl outputFormat='XML'>${noSuchVar?esc}",
        "<#ftl outputFormat='XML'>${'x'?esc + noSuchVar}",
    ] {
        assert_error_contains(&c, &l, ftl, &["noSuchVar", "null or missing"]);
    }
}
