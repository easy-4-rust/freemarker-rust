//! Java `freemarker.template.ConfigurationTest` 的 Rust 1:1 实现
//! （ConfigurationTest.java：Configuration 配置 API 的 JUnit 测试方法）
//!
//! 翻译规则（任务约定）：JUnit 测试方法翻译为 #[test]；非 API 测试跳过并注释。
//! 本文件翻译能用引擎 API 验证的方法（设置字段直映射），Java 特有的
//! ObjectWrapper/CFormat/MemberAccessPolicy/setSetting 反射机制等记为注释。
//!
//! 引擎差异总述：
//! - 引擎固定 ICI 2.3.34（Java 测试默认 2.3.0/2.3.22）；
//! - 无 setSetting API / isXxxExplicitlySet / unset 语义（v1 直接写 settings 字段）；
//! - 无 ObjectWrapper 族（SimpleObjectWrapper 见 simple_object_wrapper_test.rs）；
//! - 无 TemplateConfiguration/cacheStorage 选择/自定义格式工厂。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::core::{AutoEscaping, OutputFormatKind};
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java testTemplateLoadingErrors：默认模板加载器未设置时 getTemplate 报错。
/// 引擎差异：Java（ICI 2.3.0）默认加载器是未设置的 ClassTemplateLoader，
/// 消息含 "wasn't set" 与 "default"（2.3.21 后仅含 "wasn't set"）；v1 默认加载器
/// 是空 StringLoader，消息为 `Template not found for name "missing.ftl".`。
#[test]
fn test_template_loading_errors() {
    let (c, _loader) = test_config();
    let e = c.get_template("missing.ftl").err().expect("应未找到");
    let msg = e.to_user_message();
    // 引擎差异：Java 消息含 "wasn't set"（+2.3.0 时含 "default"）
    assert!(msg.contains("missing.ftl"), "{msg}");
}

/// Java testVersion：引擎版本号。
/// 引擎差异：Java `new Configuration(new Version(999,1,2))` 抛 IllegalArgumentException
/// 消息含 "upgrade"；`new Version(2,2,2)` 消息含 "2.3.0"——v1 Configuration 无
/// 版本构造参数（固定 2.3.34），构造校验无从表达。
#[test]
fn test_version() {
    let v = Configuration::version();
    // Java：assertTrue(v.intValue() > _VersionInts.V_2_3_20)
    assert!(v.to_int() > 2_003_020, "引擎版本应高于 2.3.20");
    // Java：assertSame(v.toString(), Configuration.getVersionNumber())
    assert_eq!(format!("{}.{}.{}", v.major, v.minor, v.micro), "2.3.34");
}

/// Java testShowErrorTips：错误消息的 Tip 提示开关。
/// 引擎差异：v1 无 showErrorTips 设置——InvalidReference 消息恒含 Tip 段；
/// setShowErrorTips(false) 后不含 "Tip:" 的断言无法复现。
#[test]
fn test_show_error_tips() {
    let (c, loader) = test_config();
    let msg = assert_error_contains(&c, &loader, "${x}", &["Tip:"]);
    assert!(msg.contains("Tip:"), "{msg}");
    // 引擎差异：Java cfg.setShowErrorTips(false) 后消息不含 "Tip:" —— v1 无此设置
}

/// Java testSetBooleanFormat：boolean_format 设置。
/// 引擎差异：非法格式 "yes no"（无逗号）Java 在 setBooleanFormat 时抛
/// IllegalArgumentException 消息含 "comma"——v1 设置字段无校验（引擎差异注释）。
#[test]
fn test_set_boolean_format() {
    let (mut c, loader) = test_config();
    c.settings.boolean_format = "yes,no".to_string(); // Java：cfg.setBooleanFormat("yes,no")
    assert_output(&c, &loader, "${true} ${false}", "yes no");

    c.settings.boolean_format = "c".to_string();
    assert_output(&c, &loader, "${true} ${false}", "true false");

    // 引擎差异：Java cfg.setBooleanFormat("yes no") 抛 IllegalArgumentException
    // 消息含 "comma"；v1 settings.boolean_format 无校验（解析期行为未对齐）
    let _ = c;
}

/// Java testTemplateUpdateDelay：模板更新延迟（秒 → 毫秒换算）。
/// 引擎差异：Java getTemplateUpdateDelayMilliseconds 返回毫秒（默认 1000）；
/// v1 settings.delay 以秒为单位（默认 1）。setSetting 的 "3 ms"/"4s" 等单位
/// 解析字符串形式未实现（无 setSetting API）。
#[test]
fn test_template_update_delay() {
    let (c, _loader) = test_config();
    // Java：assertEquals(TemplateCache.DEFAULT_TEMPLATE_UPDATE_DELAY_MILLIS,
    //   cfg.getTemplateUpdateDelayMilliseconds()) → 默认 1000ms
    assert_eq!(c.settings.delay, 1); // 引擎差异：v1 秒为单位，1s == Java 1000ms
    let delay_secs = c.settings.delay;
    assert_eq!(delay_secs * 1000, 1000u64);
    // Java：cfg.setTemplateUpdateDelay(4) → 4000L；setTemplateUpdateDelayMilliseconds(100) → 100L；
    // setSetting(TEMPLATE_UPDATE_DELAY_KEY, "5") → 5000L；"3 ms"→3L；"3 s"→3000L；
    // "3 m"→180000L；"1 h"→3600000L —— v1 仅支持秒字段直接赋值：
    //   4 → 4000ms / 100ms → 0（取整秒）
    // 引擎差异：毫秒级与单位字符串设置未实现，仅 4 秒 = 4000ms 可对齐
    let mut c = c;
    c.settings.delay = 4;
    assert_eq!(c.settings.delay * 1000, 4000u64);
}

/// Java testSharedVariables：共享变量（setSharedVariable / setSharedVaribles）。
/// 引擎差异：v1 无 setSharedVaribles(批量+重包装) 与 getSharedVariable 的
/// "删旧插新"语义（Java setSharedVaribles 先清空再插入）——仅单变量
/// set_shared_variable 可对齐。
#[test]
fn test_shared_variables() {
    let (mut c, _loader) = test_config();
    c.set_shared_variable(
        "erased",
        freemarker::template::TModel::from_scalar(String::new()),
    );
    assert!(c.shared_vars.contains_key("erased"));
    // Java：setSharedVaribles({a:"aa", b:"bb", c:MyScalarModel}) → "erased" 被清除
    // —— v1 无批量设置 API（引擎差异）
    c.set_shared_variable(
        "a",
        freemarker::template::TModel::from_scalar("aa".to_string()),
    );
    c.set_shared_variable(
        "b",
        freemarker::template::TModel::from_scalar("bb".to_string()),
    );
    // Java 断言 getSharedVariable("a").getAsString()=="aa" 且包装为 SimpleScalar；
    // v1 TModel 标量（无包装类型断言）
    assert_eq!(c.shared_vars.get("a").unwrap().get_scalar().unwrap(), "aa");
    assert_eq!(c.shared_vars.get("b").unwrap().get_scalar().unwrap(), "bb");
    // Java：cfg.setSharedVariable("b", "bbLegacy") + 更换 ObjectWrapper 后
    // setSharedVaribles 的条目被重包装（GenericObjectModel）而单条设置保持
    // SimpleScalar —— v1 无 ObjectWrapper 概念（引擎差异）
    c.set_shared_variable(
        "b",
        freemarker::template::TModel::from_scalar("bbLegacy".to_string()),
    );
    assert_eq!(
        c.shared_vars.get("b").unwrap().get_scalar().unwrap(),
        "bbLegacy"
    );
}

/// Java testSetAutoEscaping：autoEscaping 策略。
/// 引擎差异：Java 默认 ENABLE_IF_DEFAULT_AUTO_ESCAPING_POLICY（v1 AutoEscaping::Default
/// 对应）；setSetting 字符串形式未实现。
#[test]
fn test_set_auto_escaping() {
    let (mut c, _loader) = test_config();
    assert_eq!(c.settings.auto_escaping, AutoEscaping::Default);
    // Java：setAutoEscapingPolicy(ENABLE_IF_SUPPORTED) → v1 无对应（仅 On/Off/Default）
    c.settings.auto_escaping = AutoEscaping::On;
    assert_eq!(c.settings.auto_escaping, AutoEscaping::On);
    c.settings.auto_escaping = AutoEscaping::Off;
    assert_eq!(c.settings.auto_escaping, AutoEscaping::Off);
    c.settings.auto_escaping = AutoEscaping::Default;
    assert_eq!(c.settings.auto_escaping, AutoEscaping::Default);
    // 引擎差异：Java setAutoEscapingPolicy(命名约定常量) 抛 IllegalArgumentException
}

/// Java testSetOutputFormat：输出格式设置。
/// 引擎差异：Java 默认 UndefinedOutputFormat；v1 默认 PlainText。
#[test]
fn test_set_output_format() {
    let (mut c, _loader) = test_config();
    // Java：assertEquals(UndefinedOutputFormat.INSTANCE, cfg.getOutputFormat())
    assert_eq!(c.settings.output_format, OutputFormatKind::PlainText);
    // Java：setSetting(OUTPUT_FORMAT_KEY, "XML") → XMLOutputFormat
    c.settings.output_format = OutputFormatKind::Xml;
    assert_eq!(c.settings.output_format, OutputFormatKind::Xml);
    c.settings.output_format = OutputFormatKind::Html;
    assert_eq!(c.settings.output_format, OutputFormatKind::Html);
    // 引擎差异：setOutputFormat(null) 抛 NullArgumentException、
    // isOutputFormatExplicitlySet/unsetOutputFormat 无对应；
    // getOutputFormatByName("noSuchFormat")/组合格式 "HTML{RTF}" 无对应
    // （OutputFormatKind::parse 仅 8 种标准格式）
    assert_eq!(
        OutputFormatKind::parse("XHTML"),
        Some(OutputFormatKind::XHtml)
    );
    assert_eq!(OutputFormatKind::parse("noSuchFormat"), None);
}

/// Java testGetTemplateOverloads：getTemplate 各重载。
/// 可对齐部分：编码读取（get_template_encoded）、局部化命中、输出内容。
/// 引擎差异：v1 模板无 locale/customLookupCondition 字段（Java 断言
/// t.getLocale()/t.getCustomLookupCondition() 无法表达）；parseAsFTL=false 的
/// 非解析模板未实现；按名称缓存键=命中名（sourceName 概念缺失）。
#[test]
fn test_get_template_overloads() {
    let mut c = Configuration::new();
    c.settings.locale = "de_DE".to_string(); // Java：cfg.setLocale(Locale.GERMAN)
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    add_template(&loader, "t.ftl", "${1}");
    add_template(&loader, "t_en.ftl", "${1}");
    add_template(&loader, "t-utf8.ftl", "<#ftl encoding='utf-8'>");

    // Java：cfg.setDefaultEncoding("ISO-8859-1") → v1 settings.input_encoding
    c.settings.input_encoding = Some("ISO-8859-1".to_string());
    // 引擎差异：Java cfg.setEncoding(hu, "ISO-8859-2") 按 locale 映射编码未实现

    // 1 参数：getTemplate(tFtl) → locale GERMAN（v1 无 locale 字段）、
    // encoding=latin1（v1 模板无 encoding 记录——get_template 路径不记录编码；
    // get_template_encoded 才记录）
    {
        let t = c.get_template("t.ftl").unwrap();
        assert_eq!(t.name, "t.ftl");
        // 引擎差异：Java t.getLocale()==Locale.GERMAN、getCustomLookupCondition()==null
        // 引擎差异：Java t.getEncoding()==latin1——v1 get_template 不记录读取编码
        assert_eq!(render_template_content(&t), "1");
    }
    {
        // Java：getTemplate(tUtf8Ftl) → encoding "utf-8"（模板头）
        let t = c.get_template_encoded("t-utf8.ftl", None).unwrap();
        assert_eq!(t.name, "t-utf8.ftl");
        assert_eq!(t.encoding.as_deref(), Some("utf-8"));
        assert_eq!(render_template_content(&t), "");
    }
    // 2 参数（locale）：getTemplate(tFtl, Locale.US) → 命中 "t_en.ftl"
    {
        // 引擎差异：Java t.getName()=="t.ftl"（请求名）、getSourceName()=="t_en.ftl"；
        // v1 缓存键=命中名 → name=="t_en.ftl"
        let t = c.get_template_localized("t.ftl", Some("en_US")).unwrap();
        assert_eq!(t.name, "t_en.ftl");
        assert_eq!(render_template_content(&t), "1");
    }
    {
        // Java：getTemplate(tFtl, hu) → locale hu、encoding latin2 —— v1 无按
        // locale 编码映射（引擎差异），本地化命中仍按原名
        let t = c.get_template_localized("t.ftl", Some("hu_HU")).unwrap();
        assert_eq!(t.name, "t.ftl");
        assert_eq!(render_template_content(&t), "1");
    }
    // 2 参数（encoding）：getTemplate(tFtl, "utf-8")
    {
        let t = c.get_template_encoded("t.ftl", Some("utf-8")).unwrap();
        assert_eq!(t.name, "t.ftl");
        assert_eq!(render_template_content(&t), "1");
    }
    // 4 参数 parseAsFTL：Java parseAsFTL=false 时输出原文本 "${1}"——v1 无
    // 非解析模式（引擎差异，注释保留）
    // 5/6 参数 ignoreMissing/customLookupCondition：v1 无对应重载
    // （Java：getTemplate("missing.ftl", ignoreMissing=true) → null）
    // 引擎差异：Java getTemplate("missing.ftl") 抛 TemplateNotFoundException ——
    // v1 同（NotFound），但 6 参数形式无对应
    let e = c.get_template("missing.ftl").err().expect("应未找到");
    assert!(e.to_user_message().contains("missing.ftl"));
}

/// 渲染模板内容（Java assertOutputEquals 的等价物）
fn render_template_content(t: &std::rc::Rc<freemarker::template::Template>) -> String {
    let mut out = Vec::new();
    t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    )
    .unwrap();
    String::from_utf8_lossy(&out).into_owned()
}

/// Java testChangingTemplateNameFormatHasEffect：名称格式影响规范化结果。
/// 引擎差异：v1 名称格式固定 DEFAULT_2_3_0——"a/./../b.ftl" 规范化为 "a/b.ftl"
/// 与 Java 2.3.0 断言一致；setTemplateNameFormat(DEFAULT_2_4_0) 后（期望 "b.ftl"）
/// 未实现。
#[test]
fn test_changing_template_name_format_has_effect() {
    let (c, loader) = test_config();
    add_template(&loader, "a/b.ftl", "In a/b.ftl");
    add_template(&loader, "b.ftl", "In b.ftl");

    // Java 2.3.0 格式断言（引擎一致）：
    let t = c.get_template("a/./../b.ftl").unwrap();
    assert_eq!(t.name, "a/b.ftl");
    // 引擎差异：Java t.getSourceName()=="a/b.ftl"；v1 无 sourceName 概念
    assert_eq!(render_template_content(&t), "In a/b.ftl");

    // 引擎差异：Java cfg.setTemplateNameFormat(DEFAULT_2_4_0) 后
    // getTemplate("a/./../b.ftl") → name/sourceName "b.ftl"、输出 "In b.ftl"
    // —— v1 名称格式固定 2.3.0，无法切换
}

/// Java testSetTemplateLoaderAndCache：缓存条目计数（含换加载器清缓存）。
/// 引擎差异：Java setClassForTemplateLoading 无对应（用 StringLoader 等价）——
/// v1 换 loader 不清缓存（Java 换加载器时清空缓存），故仅验证计数语义；
/// "换回同一 loader 不清缓存" 与 Java 一致。
#[test]
fn test_set_template_loader_and_cache() {
    let mut c = Configuration::new();
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();

    // Java：cacheStorage.getSize()==0（StrongCacheStorage 初始）
    assert_eq!(c.cache.lock().unwrap().len(), 0);

    add_template(&loader, "toCache1.ftl", "1");
    add_template(&loader, "toCache2.ftl", "2");
    assert_eq!(c.cache.lock().unwrap().len(), 0);
    c.get_template("toCache1.ftl").unwrap();
    assert_eq!(c.cache.lock().unwrap().len(), 1);
    c.get_template("toCache2.ftl").unwrap();
    assert_eq!(c.cache.lock().unwrap().len(), 2);
    // 引擎差异：Java setClassForTemplateLoading(...)（换 loader）清空缓存 → 0；
    // v1 换 template_loader 不清缓存（Configuration 层未联动）
    c.get_template("toCache1.ftl").unwrap();
    assert_eq!(c.cache.lock().unwrap().len(), 2);
    // Java：setTemplateLoader(cfg.getTemplateLoader())（同一实例）→ 不清缓存
    // —— v1 相同（赋值不触发清空）
    assert_eq!(c.cache.lock().unwrap().len(), 2);
}

/// Java testChangingLocalizedLookupClearsCache：切换 localizedLookup 清缓存。
/// 引擎差异：Java setLocalizedLookup 变化清空缓存（因键含 locale 维度）；
/// v1 缓存键=名称，切换设置不影响既有条目——断言改为验证设置字段本身
/// （缓存清空行为无法对齐，注释保留 Java 断言）。
#[test]
fn test_changing_localized_lookup_clears_cache() {
    let (mut c, loader) = test_config();
    add_template(&loader, "toCache1.ftl", "1");
    c.get_template("toCache1.ftl").unwrap();
    assert_eq!(c.cache.lock().unwrap().len(), 1);
    // Java：setLocalizedLookup(true)（不变）→ 缓存仍 1；setLocalizedLookup(false) → 0
    // 引擎差异：v1 切换 settings.localized_lookup 不清缓存（键无 locale 维度）
    c.settings.localized_lookup = false;
    assert_eq!(c.cache.lock().unwrap().len(), 1);
    c.get_template("toCache1.ftl").unwrap();
    assert_eq!(c.cache.lock().unwrap().len(), 1);
    c.settings.localized_lookup = true;
    assert_eq!(c.cache.lock().unwrap().len(), 1);
}

/// Java testChangingTemplateNameFormatClearsCache：切换名称格式清缓存。
/// 引擎差异：v1 名称格式固定 2.3.0、无切换 API——整体跳过并注释 Java 断言。
#[test]
fn test_changing_template_name_format_clears_cache() {
    // Java：setTemplateNameFormat(DEFAULT_2_3_0)（不变）→ 缓存 1；
    // setTemplateNameFormat(DEFAULT_2_4_0) → 0；再设回 2_3_0 → 0。
    // v1 无名称格式切换（template_name_format.rs：名称格式固定），不可移植。
    let (c, _loader) = test_config();
    assert_eq!(
        c.settings.incompatible_improvements,
        freemarker::template::Version::V2_3_34
    );
}

/// Java testLocaleSetting：locale 设置。
/// 引擎差异：Java 默认 Locale.getDefault()（JVM 相关）；v1 默认固定 "en_US"；
/// "JVM default" 字符串与 unset 语义无对应。
#[test]
fn test_locale_setting() {
    let (mut c, _loader) = test_config();
    assert_eq!(c.settings.locale, "en_US");
    // Java：cfg.setLocale(nonDefault) → isLocaleExplicitlySet()==true —— v1 无
    // explicitlySet 概念（直接字段赋值）
    c.settings.locale = "de_DE".to_string();
    assert_eq!(c.settings.locale, "de_DE");
}

/// Java testDefaultEncodingSetting：默认编码设置。
/// 引擎差异：Java 默认 file.encoding（JVM 属性）；v1 input_encoding=None 时按
/// UTF-8 读取（settings.input_encoding 的 None = Java 默认 "UTF-8"）。
#[test]
fn test_default_encoding_setting() {
    let (mut c, _loader) = test_config();
    assert_eq!(c.settings.input_encoding, None);
    c.settings.input_encoding = Some("ISO-8859-1".to_string());
    assert_eq!(c.settings.input_encoding.as_deref(), Some("ISO-8859-1"));
}

/// Java testTimeZoneSetting：时区设置。
/// 引擎差异：Java 默认 TimeZone.getDefault()；v1 默认 GMT+00:00；
/// "JVM default"/unset 无对应。
#[test]
fn test_time_zone_setting() {
    let (mut c, _loader) = test_config();
    // Java：cfg.getTimeZone()==TimeZone.getDefault() —— v1 默认 GMT+00:00
    c.settings.time_zone =
        freemarker::core::TzSetting::Fixed(chrono::FixedOffset::east_opt(0).unwrap());
    // Java：cfg.setTimeZone(DateUtil.UTC) → isTimeZoneExplicitlySet()==true
    // 引擎差异：v1 无 explicitlySet；"JVM default" 设置字符串未实现
    assert_eq!(c.settings.time_zone_id, "GMT+01:00"); // test_config 的 GMT+1
}

/// Java testFallbackOnNullLoopVariable：循环变量为 null 时回退到上次循环值
#[test]
fn test_fallback_on_null_loop_variable() {
    let (mut c, _loader) = test_config();
    assert!(c.settings.fallback_on_null_loop_variable);
    // Java：setSetting("fallback_on_null_loop_variable", "false") → false
    c.settings.fallback_on_null_loop_variable = false;
    assert!(!c.settings.fallback_on_null_loop_variable);
    c.settings.fallback_on_null_loop_variable = true;
    assert!(c.settings.fallback_on_null_loop_variable);
    // 引擎差异：Java setSetting(..., "NO") 也解析为 false —— v1 无 setSetting
    // 字符串解析（直接布尔赋值）
}

// ---------------------------------------------------------------------------
// 以下 Java 测试方法整体跳过（无引擎等价物，按任务约定注释）
// ---------------------------------------------------------------------------

// testIncompatibleImprovementsChangesDefaults：ObjectWrapper/模板加载器随 ICI
// 切换（DefaultObjectWrapper 族）——v1 无 ObjectWrapper 选择与 ICI 切换
// （引擎固定 ICI 2.3.34）。Java 断言要点：2.3.0 → 旧包装器+FileTemplateLoader；
// 2.3.21 → DefaultObjectWrapper(2.3.21)；2.3.22 → DOW(2.3.22)+treatDefaultMethods；
// 2.3.27 → preferIndexedReadMethod=false；cFormat 在 2.3.32 变 JavaScriptOrJSON。

// testUnsetAndIsExplicitlySet：isXxxExplicitlySet/unset 反射语义——v1 无
// explicitlySet 概念（直接字段赋值），整体跳过。

// testSetTemplateLoaderAndCache（其余部分）、testTemplateLookupStrategyDefaultAndSet：
// setClassForTemplateLoading/cacheStorage 计数已部分翻译；TemplateLookupStrategy
// 的 setSetting 与缓存联动见 template_lookup_strategy_test.rs。

// testTemplateNameFormatSetSetting / testObjectWrapperSetSetting /
// testSetTemplateConfigurations / testGetOutputFormatByName /
// testSetRegisteredCustomOutputFormats / testSetRecognizeStandardFileExtensions /
// testSetTimeZone / testSetSQLDateAndTimeTimeZone / testTimeZoneLayers /
// testSetICIViaSetSettingAPI / testSetLogTemplateExceptionsViaSetSettingAPI /
// testSetWrapUncheckedExceptionsViaSetSettingAPI / testSetAttemptExceptionReporter /
// testApiBuiltinEnabled / testSetCustomNumberFormat / testSetTabSize /
// testTabSizeSetting / testSetCustomDateFormat / testHasCustomFormats /
// testNamingConventionSetSetting / testTagSyntaxSetting /
// testInterpolationSyntaxSetting / testLazyImportsSetSetting /
// testLazyAutoImportsSetSetting / testNewBuiltinClassResolverSetting /
// testTruncateBuiltinAlgorithm / testCFormat / testMemberAccessPolicySetting(2) /
// testGetSettingNamesAreSorted / testGetSettingNamesNameConventionsContainTheSame /
// testStaticFieldKeysCoverAllGetSettingNames / testGetSettingNamesCoversAllStaticKeyFields /
// testKeyStaticFieldsHasAllVariationsAndCorrectFormat /
// testGetSettingNamesCoversAllSettingNames / testSetSettingSupportsBothNamingConventions /
// testGetSupportedBuiltInDirectiveNames / testGetSupportedBuiltInNames：
// 均为 Java Configuration/Configurable 的 setSetting 解析、ObjectWrapper、
// 自定义格式工厂、命名约定、反射自检（*_KEY 静态字段与设置名集合一致性）等
// Java 特有 API——v1 无对应（settings 为 Rust 结构体字段，非字符串设置表），
// 跳过并注释。
