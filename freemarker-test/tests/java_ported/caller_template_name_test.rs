//! Java `freemarker.core.CallerTemplateNameTest` 的 Rust 1:1 实现
//! （对应 Java: CallerTemplateNameTest —— `.callerTemplateName`/`.caller_template_name`
//!   特殊变量在宏/函数/include/import/嵌套/参数默认值/局部化查找等场景下的"调用方模板名"）。
//!
//! ENGINE_GAP: 引擎没有 `.callerTemplateName`/`.caller_template_name` 特殊变量
//!   （BuiltinVar 无该 variant；grammar.rs:3189 的报错清单列出全部允许的特殊变量名，
//!   不含 callerTemplateName）——本测试类全部 10 个方法都依赖该变量，无法 1:1
//!   迁移，全部登记 NOT_APPLICABLE（Java 原文保留为注释）。
//!
//! NOT_APPLICABLE: 全部 10 个方法 —— `.caller_template_name` 特殊变量引擎未实现。
//!   Java 的语义（CallerTemplateName 是 ICI 2.3.28+ 的 BuiltinVariable，
//!   返回"调用方"模板名：宏/函数定义所在模板？否——`<@m/>` 的调用点所在模板；
//!   宏内 `<#nested>` 时切回主调用方；include 链中为包含方模板名；import 为库
//!   模板名；局部化查找返回查找名而非实际文件名）依赖引擎的环境指令栈
//!   （Java Environment.getCurrentDirectiveCallPlace/调用方模板追踪），v1 无对应。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（CallerTemplateNameTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java testBaics（Java 原文）：
//   addTemplate("main.ftl", ""
//           + "<#macro m>${.callerTemplateName}</#macro>"
//           + "<#function f()><#return .callerTemplateName></#function>"
//           + "<@m /> ${f()} [<#include 'other.ftl'>] <@m /> ${f()}");
//   addTemplate("other.ftl", ""
//           + "<@m /> ${f()} [<#include 'yet-another.ftl'>] <@m /> ${f()}");
//   addTemplate("yet-another.ftl", ""
//           + "<@m /> ${f()}");
//   assertOutputForNamed("main.ftl", ""
//           + "main.ftl main.ftl "
//           + "[other.ftl other.ftl "
//           + "[yet-another.ftl yet-another.ftl] "
//           + "other.ftl other.ftl] "
//           + "main.ftl main.ftl");

// Java testNoCaller（Java 原文）：
//   assertErrorContains("${.callerTemplateName}", "no macro or function", ".callerTemplateName");
//   assertErrorContains("${.caller_template_name}", "no macro or function", ".caller_template_name");
//   assertErrorContains(""
//           + "<#macro m><#nested></#macro>"
//           + "<@m>${.callerTemplateName}</@>",
//           "no macro or function", ".callerTemplateName");
//   addTemplate("main.ftl", "${.callerTemplateName}");
//   assertErrorContainsForNamed("main.ftl", "no macro or function");

// Java testNamelessCaller（Java 原文）：
//   assertOutput(""
//           + "<#macro m2>${.callerTemplateName}</#macro>"
//           + "[<@m2/>]",
//           "[]");

// Java testNested（Java 原文）：
//   addTemplate("main.ftl", ""
//           + "<#include 'lib1.ftl'>"
//           + "<#include 'lib2.ftl'>"
//           + "<@m1 />");
//   addTemplate("lib1.ftl", ""
//           + "<#macro m1>"
//           + "${.callerTemplateName} [<@m2>${.callerTemplateName}</@m2>] ${.callerTemplateName}"
//           + "</#macro>");
//   addTemplate("lib2.ftl", ""
//           + "<#macro m2>"
//           + "${.callerTemplateName} [<#nested>] ${.callerTemplateName}"
//           + "</#macro>");
//   assertOutputForNamed("main.ftl", ""
//           + "main.ftl [lib1.ftl [main.ftl] lib1.ftl] main.ftl");

// Java testSelfCaller（Java 原文）：
//   addTemplate("main.ftl", ""
//           + "<#macro m>${.callerTemplateName}</#macro>"
//           + "<@m />");
//   assertOutputForNamed("main.ftl", "main.ftl");

// Java testImportedTemplateCaller（Java 原文）：
//   addTemplate("main.ftl", ""
//           + "<#import 'lib/foo.ftl' as foo>"
//           + "<@foo.m />, <@foo.m2 />");
//   addTemplate("lib/foo.ftl", ""
//           + "<#macro m>${.callerTemplateName}</#macro>"
//           + "<#macro m2><@m3/></#macro>"
//           + "<#macro m3>${.callerTemplateName}</#macro>");
//   assertOutputForNamed("main.ftl", "main.ftl, lib/foo.ftl");

// Java testNestedIntoNonUserDirectives（Java 原文）：
//   addTemplate("main.ftl", ""
//           + "<#macro m><#list 1..2 as _><#if true>${.callerTemplateName}</#if>;</#list></#macro>"
//           + "<@m/>");
//   assertOutputForNamed("main.ftl", "main.ftl;main.ftl;");

// Java testUsedInArgument（Java 原文）：
//   addTemplate("main.ftl", ""
//           + "<#include 'inc.ftl'>"
//           + "<#macro start>"
//           + "<@m .callerTemplateName />"
//           + "<@m2 />"
//           + "</#macro>"
//           + "<@start />");
//   addTemplate("inc.ftl", ""
//           + "<#macro m x y=.callerTemplateName>"
//           + "x: ${x}; y: ${y}; caller: ${.callerTemplateName};"
//           + "</#macro>"
//           + "<#macro m2><@m .callerTemplateName /></#macro>");
//   for (int i = 0; i < 2; i++) {
//       assertOutputForNamed("main.ftl", ""
//               + "x: main.ftl; y: main.ftl; caller: main.ftl;"
//               + "x: main.ftl; y: inc.ftl; caller: inc.ftl;");
//       getConfiguration().setIncompatibleImprovements(Configuration.VERSION_2_3_27); // Has no effect
//   }

// Java testReturnsLookupName（Java 原文）：
//   addTemplate("main_en.ftl", ""
//           + "<#macro m>${.callerTemplateName}</#macro>"
//           + "<@m />");
//   assertOutputForNamed("main.ftl", "main.ftl"); // Not main_en.ftl

// Java testLegacyCall（Java 原文）：
//   addTemplate("main_en.ftl", ""
//           + "<#macro m>${.callerTemplateName}</#macro>"
//           + "<#call m>");
//   assertOutputForNamed("main.ftl", "main.ftl"); // Not main_en.ftl
