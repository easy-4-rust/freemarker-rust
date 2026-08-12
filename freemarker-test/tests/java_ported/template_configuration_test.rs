//! Java `freemarker.core.TemplateConfigurationTest` 的 Rust 1:1 实现
//! （对应 Java: TemplateConfigurationTest —— TemplateConfiguration 配置合并/应用/
//!   parser 设置/自定义属性 + 模板层 autoImport 等）。
//!
//! 引擎差异总览：
//! - Java `TemplateConfiguration` 已实现（core/template_configuration.rs，v1 子集：
//!   Settings 对应项；setXxx/merge/apply 语义 → Option 字段 + merge/apply_to）
//!   —— testMergeBasicFunctionality 等反射遍历属性两两 merge 的测试仍 NA
//!   （Java 反射遍历；v1 用 apply_to/merge 单测代替，见 core 单测）；
//! - `DummyArithmeticEngine`（自定义算术引擎 add→22 等）引擎无（算术引擎固定）
//!   → testArithmeticEngine/testStringInterpolate/testInterpret/testEval 的
//!   "with" 档（"11 22 33"）无法翻译，整方法 NOT_APPLICABLE。
//! - 引擎有 `Configuration.auto_imports`（Vec<(ns, path)>，对应 Java addAutoImport/
//!   setAutoImports）→ testAutoImport 模板层等价翻译。
//! - `autoIncludes` 引擎无 → testAutoIncludes NOT_APPLICABLE。
//!
//! NOT_APPLICABLE: testMergeBasicFunctionality —— Java 反射遍历
//!   TemplateConfiguration 属性两两 merge；v1 merge 单测见 core/template_configuration.rs
//!   （apply_to_overrides_and_merge）。
//! NOT_APPLICABLE: testMergeMapSettings —— TemplateConfiguration.setCustomDateFormats/
//!   setCustomNumberFormats/setAutoImports + merge；引擎无该类。
//! NOT_APPLICABLE: testMergeListSettings —— TemplateConfiguration.setAutoIncludes +
//!   merge（ListUtils.union 去重语义）；引擎无该类。
//! NOT_APPLICABLE: testMergePriority —— TemplateConfiguration.merge 优先级（后 merge 覆盖）；
//!   v1 语义同 Java（merge 覆盖已设置项），单测见 core。
//! NOT_APPLICABLE: testMergeCustomAttributes / testMergeNullCustomAttributes ——
//!   CustomAttribute（SCOPE_TEMPLATE）+ merge 的 null 语义；引擎无该类。
//! NOT_APPLICABLE: applyOrder —— TemplateConfiguration.apply(Template) 的合并顺序；
//!   v1 渲染期应用（Environment::new）。
//! NOT_APPLICABLE: testConfigureNonParserConfig —— Java 反射（getWriteMethod/
//!   getReadMethod）逐一验证 apply 生效；v1 集成测试见本文件
//!   test_configurable_settings / test_locale（渲染级验证）。
//! NOT_APPLICABLE: testConfigureCustomAttributes —— CustomAttribute API + apply；
//!   引擎无该类。
//! NOT_APPLICABLE: testConfigureParser —— TemplateConfiguration 的 parser 设置
//!   （tagSyntax/interpolationSyntax/namingConvention/whitespaceStripping/
//!   arithmeticEngine/outputFormat/autoEscapingPolicy/strictSyntaxMode/ICI/
//!   recognizeStandardFileExtensions/tabSize）经 apply 生效；v1 仅渲染期设置
//!   （解析期参数无对应，见 core/template_configuration.rs 头注释）。
//! NOT_APPLICABLE: testConfigureParserTooLowIcI —— Java ICI 门控（Configurable 设置
//!   在 ICI < 2.3.22 时抛 IllegalStateException）；引擎无 per-setting ICI 门控。
//! NOT_APPLICABLE: testArithmeticEngine —— DummyArithmeticEngine 自定义算术引擎；
//!   引擎算术引擎固定。
//! NOT_APPLICABLE: testAutoIncludes —— setAutoIncludes 自动 include；引擎无。
//! NOT_APPLICABLE: testStringInterpolate —— DummyArithmeticEngine（`${'${1+1}'}`
//!   字符串插值中的自定义算术引擎）；引擎算术引擎固定。
//! NOT_APPLICABLE: testInterpret —— DummyArithmeticEngine + ?interpret 中的算术引擎；
//!   引擎算术引擎固定（?interpret 本身引擎支持）。
//! NOT_APPLICABLE: testEval —— DummyArithmeticEngine + ?eval + outputEncoding/
//!   namingConvention 交互（`.outputEncoding` 与 `.output_encoding` 的命名约定
//!   门控）；引擎命名约定恒宽松（camelCase/snake_case 双写均接受），无命名约定设置。
//! NOT_APPLICABLE: testSetParentConfiguration —— setParentConfiguration 的
//!   IllegalStateException/NullArgumentException 语义；v1 无绑定对象（Arc 持有）。
//! NOT_APPLICABLE: testIsSet —— 反射检查每个属性 isSet 方法；v1 用 Option 字段
//!   表示已设置，无独立 isSet API。
//! NOT_APPLICABLE: checkTestAssignments —— 反射自检 SETTING_ASSIGNMENTS 覆盖全部属性；
//!   引擎无该类。

#![allow(clippy::field_reassign_with_default)] // Java 风格：Default 后逐个 setter
#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::{
    AndMatcher, ConditionalTemplateConfigurationFactory, FileExtensionMatcher, FileNameGlobMatcher,
    FirstMatchTemplateConfigurationFactory, MergingTemplateConfigurationFactory, NotMatcher,
    OrMatcher, PathGlobMatcher, PathRegexMatcher, StringLoader, TemplateSourceMatcher,
};
use freemarker::core::TemplateConfiguration;
use freemarker::template::Configuration;

use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.locale = "en_US".to_string();
    (c, loader)
}

/// Java TemplateConfigurationWithTemplateCacheTest.testConfigurableSettings：
/// Merging factory 合并 locale/boolean_format/number_format 并渲染生效
#[test]
fn test_configurable_settings() {
    let (mut c, loader) = cfg();
    let mut tc_fr = TemplateConfiguration::default();
    tc_fr.locale = Some("fr_FR".to_string());
    let mut tc_yn = TemplateConfiguration::default();
    tc_yn.boolean_format = Some("Y,N".to_string());
    let mut tc_00 = TemplateConfiguration::default();
    tc_00.number_format = Some("0.00".to_string());
    c.set_template_configurations(Some(Arc::new(MergingTemplateConfigurationFactory::new(
        vec![
            Box::new(ConditionalTemplateConfigurationFactory::with_configuration(
                Box::new(FileNameGlobMatcher::new("*(fr)*")),
                Arc::new(tc_fr),
            )),
            Box::new(ConditionalTemplateConfigurationFactory::with_configuration(
                Box::new(FileNameGlobMatcher::new("*(yn)*")),
                Arc::new(tc_yn),
            )),
            Box::new(ConditionalTemplateConfigurationFactory::with_configuration(
                Box::new(FileNameGlobMatcher::new("*(00)*")),
                Arc::new(tc_00),
            )),
        ],
    ))));

    let common_ftl = "${.locale} ${true?string} ${1.2}";
    add_template(&loader, "default", common_ftl);
    add_template(&loader, "(fr)", common_ftl);
    add_template(&loader, "(yn)(00)", common_ftl);
    add_template(&loader, "(00)(fr)", common_ftl);

    assert_eq!(render_named(&c, &loader, "default"), "en_US true 1.2");
    assert_eq!(render_named(&c, &loader, "(fr)"), "fr_FR true 1,2");
    assert_eq!(render_named(&c, &loader, "(yn)(00)"), "en_US Y 1.20");
    assert_eq!(render_named(&c, &loader, "(00)(fr)"), "fr_FR true 1,20");
}

/// Java TemplateConfigurationWithTemplateCacheTest.testLocale：模板配置覆盖 locale
/// （`getTemplate(name, locale)` 的 per-call locale 覆盖无对应 API——跳过）
#[test]
fn test_locale() {
    let (mut c, loader) = cfg();
    let mut tc_de = TemplateConfiguration::default();
    tc_de.locale = Some("de_DE".to_string());
    c.set_template_configurations(Some(Arc::new(
        ConditionalTemplateConfigurationFactory::with_configuration(
            Box::new(FileNameGlobMatcher::new("*(de)*")),
            Arc::new(tc_de),
        ),
    )));
    add_template(&loader, "(de).ftl", "${.locale}");
    add_template(&loader, "default.ftl", "${.locale}");

    assert_eq!(render_named(&c, &loader, "(de).ftl"), "de_DE");
    assert_eq!(render_named(&c, &loader, "default.ftl"), "en_US");
    // 引擎差异：Java `<#ftl locale='fr_FR'>` 头参数 v1 不支持（"Unknown FTL
    // header parameter: locale"）→ 模板内 locale 覆盖档跳过
}

/// Java TemplateConfigurationFactoryTest.testCondition1/2：条件工厂命中/未命中
#[test]
fn test_conditional_factory() {
    let (c, loader) = cfg();
    let mut tc = TemplateConfiguration::default();
    tc.number_format = Some("0.00".to_string());
    let factory = ConditionalTemplateConfigurationFactory::with_configuration(
        Box::new(FileExtensionMatcher::new("ftl")),
        Arc::new(tc),
    );
    add_template(&loader, "a.ftl", "${1.234}");
    add_template(&loader, "a.txt", "${1.234}");
    let mut c2 = c.clone();
    c2.set_template_configurations(Some(Arc::new(factory)));
    assert_eq!(render_named(&c2, &loader, "a.ftl"), "1.23");
    assert_eq!(
        render_named(&c2, &loader, "a.txt"),
        "1.234",
        "未命中 → 全局设置"
    );
}

/// Java TemplateConfigurationFactoryTest.testFirstMatch：首个命中 + 无匹配报错
#[test]
fn test_first_match_factory() {
    let (c, loader) = cfg();
    let mut tc1 = TemplateConfiguration::default();
    tc1.boolean_format = Some("Y,N".to_string());
    let mut tc2 = TemplateConfiguration::default();
    tc2.boolean_format = Some("O,K".to_string());
    let factory = FirstMatchTemplateConfigurationFactory::new(vec![
        Box::new(ConditionalTemplateConfigurationFactory::with_configuration(
            Box::new(PathGlobMatcher::new("foo/**")),
            Arc::new(tc1),
        )),
        Box::new(ConditionalTemplateConfigurationFactory::with_configuration(
            Box::new(PathGlobMatcher::new("bar/**")),
            Arc::new(tc2),
        )),
    ])
    .no_match_error_details("no config for this file".to_string());

    add_template(&loader, "foo/a.ftl", "${true}");
    add_template(&loader, "bar/a.ftl", "${true}");
    add_template(&loader, "baz/a.ftl", "${true}");
    let mut c2 = c.clone();
    c2.set_template_configurations(Some(Arc::new(factory)));
    assert_eq!(render_named(&c2, &loader, "foo/a.ftl"), "Y");
    assert_eq!(render_named(&c2, &loader, "bar/a.ftl"), "O");
    // 无匹配 → TemplateConfigurationFactoryException 传播为加载错误
    let err = match c2.get_template("baz/a.ftl") {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert!(
        err.to_string().contains(
            "FirstMatchTemplateConfigurationFactory has found no matching choice for source name \"baz/a.ftl\""
        ),
        "{err}"
    );
    assert!(
        err.to_string()
            .contains("Error details: no config for this file"),
        "{err}"
    );
}

/// matcher 组合（Java TemplateSourceMatcherTest：And/Or/Not 组合 + 边界）
#[test]
fn test_matcher_combinations() {
    let and = AndMatcher::new(vec![
        Box::new(PathGlobMatcher::new("a/**")),
        Box::new(FileExtensionMatcher::new("ftlh")),
    ]);
    assert!(and.matches("a/b.ftlh"));
    assert!(!and.matches("b/b.ftlh"));
    assert!(!and.matches("a/b.ftl"));

    let or = OrMatcher::new(vec![
        Box::new(FileExtensionMatcher::new("ftl")),
        Box::new(FileExtensionMatcher::new("ftlh")),
    ]);
    assert!(or.matches("x.ftl"));
    assert!(or.matches("x.ftlh"));
    assert!(!or.matches("x.txt"));

    let not = NotMatcher::new(Box::new(FileExtensionMatcher::new("ftl")));
    assert!(!not.matches("x.ftl"));
    assert!(not.matches("x.txt"));

    let regex = PathRegexMatcher::new(r"^lib/.*\.ftl$");
    assert!(regex.matches("lib/a.ftl"));
    assert!(!regex.matches("a.ftl"));
}

/// Java testAutoImport —— autoImports 自动导入（引擎 Configuration.auto_imports
/// 等价 Java addAutoImport；默认档与配置档均按 Java 期望输出）
#[test]
fn test_auto_import() {
    let (mut c, l) = test_config();
    add_template(&l, "t1.ftl", "<#global loaded = (loaded!) + 't1;'>In t1;");
    add_template(&l, "t2.ftl", "<#global loaded = (loaded!) + 't2;'>In t2;");
    add_template(&l, "t3.ftl", "<#global loaded = (loaded!) + 't3;'>In t3;");

    // 对应 Java assertOutputWithoutAndWithTC 的 expectedDefaultOutput "t3;"
    assert_output(&c, &l, "<#import 't3.ftl' as t3>${loaded}", "t3;");

    // 对应 Java tc.setAutoImports(ImmutableMap.of("t1", "t1.ftl", "t2", "t2.ftl"))
    c.auto_imports
        .push(("t1".to_string(), "t1.ftl".to_string()));
    c.auto_imports
        .push(("t2".to_string(), "t2.ftl".to_string()));
    assert_output(&c, &l, "<#import 't3.ftl' as t3>${loaded}", "t1;t2;t3;");
}
