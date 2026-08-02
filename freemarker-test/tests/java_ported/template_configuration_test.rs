//! Java `freemarker.core.TemplateConfigurationTest` 的 Rust 1:1 实现
//! （对应 Java: TemplateConfigurationTest —— TemplateConfiguration 配置合并/应用/
//!   parser 设置/自定义属性 + 模板层 autoImport 等）。
//!
//! 引擎差异总览：
//! - Java `TemplateConfiguration` 类（setXxx/merge/apply/CustomAttribute）引擎无
//!   （Configuration 仅 settings + auto_imports + shared_vars）→ 依赖它的测试
//!   NOT_APPLICABLE。
//! - `DummyArithmeticEngine`（自定义算术引擎 add→22 等）引擎无（算术引擎固定）
//!   → testArithmeticEngine/testStringInterpolate/testInterpret/testEval 的
//!   "with" 档（"11 22 33"）无法翻译，整方法 NOT_APPLICABLE。
//! - 引擎有 `Configuration.auto_imports`（Vec<(ns, path)>，对应 Java addAutoImport/
//!   setAutoImports）→ testAutoImport 模板层等价翻译。
//! - `autoIncludes` 引擎无 → testAutoIncludes NOT_APPLICABLE。
//!
//! NOT_APPLICABLE: testMergeBasicFunctionality —— Java 反射遍历
//!   TemplateConfiguration 属性两两 merge；引擎无该类。
//! NOT_APPLICABLE: testMergeMapSettings —— TemplateConfiguration.setCustomDateFormats/
//!   setCustomNumberFormats/setAutoImports + merge；引擎无该类。
//! NOT_APPLICABLE: testMergeListSettings —— TemplateConfiguration.setAutoIncludes +
//!   merge（ListUtils.union 去重语义）；引擎无该类。
//! NOT_APPLICABLE: testMergePriority —— TemplateConfiguration.merge 优先级（后 merge 覆盖）；
//!   引擎无该类。
//! NOT_APPLICABLE: testMergeCustomAttributes / testMergeNullCustomAttributes ——
//!   CustomAttribute（SCOPE_TEMPLATE）+ merge 的 null 语义；引擎无该类。
//! NOT_APPLICABLE: applyOrder —— TemplateConfiguration.apply(Template) 的合并顺序；
//!   引擎无该类。
//! NOT_APPLICABLE: testConfigureNonParserConfig —— Java 反射（getWriteMethod/
//!   getReadMethod）逐一验证 apply 生效；引擎无 TemplateConfiguration。
//! NOT_APPLICABLE: testConfigureCustomAttributes —— CustomAttribute API + apply；
//!   引擎无该类。
//! NOT_APPLICABLE: testConfigureParser —— TemplateConfiguration 的 parser 设置
//!   （tagSyntax/interpolationSyntax/namingConvention/whitespaceStripping/
//!   arithmeticEngine/outputFormat/autoEscapingPolicy/strictSyntaxMode/ICI/
//!   recognizeStandardFileExtensions/tabSize）经 apply 生效；引擎无该类
//!   （parser 设置经 Settings 全局配置，无 per-template 覆盖机制）。
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
//!   IllegalStateException/NullArgumentException 语义；引擎无该类。
//! NOT_APPLICABLE: testIsSet —— 反射检查每个属性 isSet 方法；引擎无该类。
//! NOT_APPLICABLE: checkTestAssignments —— 反射自检 SETTING_ASSIGNMENTS 覆盖全部属性；
//!   引擎无该类。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

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
