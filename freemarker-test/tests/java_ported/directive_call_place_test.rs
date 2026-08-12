//! Java `freemarker.core.DirectiveCallPlaceTest` 的 Rust 1:1 实现
//! （对应 Java: DirectiveCallPlaceTest —— 自定义指令经 `env.getCurrentDirectiveCallPlace()`
//!   读取调用点位置（行/列）与 nested 输出缓存（getOrCreateCustomData +
//!   isNestedOutputCacheable，缓存按指令类型 identity 区分）。
//!
//! ENGINE_GAP: 引擎没有 `DirectiveCallPlace` API（Java Environment.
//!   getCurrentDirectiveCallPlace → DirectiveCallPlace：isNestedOutputCacheable /
//!   getOrCreateCustomData / getTemplate / getBeginLine|Column / getEndLine|Column，
//!   CallPlaceCustomDataInitializationException）——自定义指令无法读取调用点
//!   位置，也无 nested 输出缓存机制。本测试类全部 3 个方法都依赖该 API（
//!   testCustomDataBasics/testCustomDataProviderMismatch 的 "[cached N]" 计数、
//!   testPositions 的 "[name:line:col-line:col]" 定位），无法 1:1 迁移，
//!   全部登记 NOT_APPLICABLE（Java 原文保留为注释）。
//!
//! NOT_APPLICABLE: testCustomDataBasics —— CachingTextConverterDirective 的
//!   nested 输出缓存（getOrCreateCustomData + 静态 cacheRecreationCount 计数）。
//! NOT_APPLICABLE: testCustomDataProviderMismatch —— 同一调用点交替使用不同
//!   identity 指令时的缓存失效语义。
//! NOT_APPLICABLE: testPositions —— PositionAwareDirective/CurDirLineScalar 的
//!   调用点行列号（getCurrentDirectiveCallPlace().getBeginLine() 等）。
//!
//! （Java 辅助类 CachingTextConverterDirective/CachingUpperCaseDirective/
//!   CachingLowerCaseDirective/PositionAwareDirective/ArgPrinterDirective/
//!   CurDirLineScalar 均为自定义 TemplateDirectiveModel，直接读上述缺失 API，
//!   不逐行保留。）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（DirectiveCallPlaceTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java 测试数据模型（Java 原文 createDataModel）：
//   Map<String, Object> dm = new HashMap<>();
//   dm.put("uc", new CachingUpperCaseDirective());
//   dm.put("lc", new CachingLowerCaseDirective());
//   dm.put("pa", new PositionAwareDirective());
//   dm.put("argP", new ArgPrinterDirective());
//   dm.put("curDirLine", new CurDirLineScalar());
//   dm.put("x", 123);

// Java testCustomDataBasics（Java 原文）：
//   addTemplate(
//           "customDataBasics.ftl",
//           "<@uc>Abc</@uc> <@uc>x=${x}</@uc> <@uc>Ab<#-- -->c</@uc> <@lc/><@lc></@lc> <@lc>Abc</@lc>");
//   CachingTextConverterDirective.resetCacheRecreationCount();
//   for (int i = 0; i < 3; i++) {
//       assertOutputForNamed(
//               "customDataBasics.ftl",
//               "ABC[cached 1] X=123 ABC[cached 2]  abc[cached 3]");
//   }

// Java testCustomDataProviderMismatch（Java 原文）：
//   addTemplate(
//           "customDataProviderMismatch.ftl",
//           "<#list [uc, lc, uc] as d><#list 1..2 as _><@d>Abc</@d></#list></#list>");
//   CachingTextConverterDirective.resetCacheRecreationCount();
//   assertOutputForNamed(
//           "customDataProviderMismatch.ftl",
//           "ABC[cached 1]ABC[cached 1]abc[cached 2]abc[cached 2]ABC[cached 3]ABC[cached 3]");
//   assertOutputForNamed(
//           "customDataProviderMismatch.ftl",
//           "ABC[cached 3]ABC[cached 3]abc[cached 4]abc[cached 4]ABC[cached 5]ABC[cached 5]");

// Java testPositions（Java 原文）：
//   addTemplate(
//           "positions.ftl",
//           "<@pa />\n"
//           + "..<@pa\n"
//           + "/><@pa>xxx</@>\n"
//           + "<@pa>{<@pa/> <@pa/>}</@>\n"
//           + "${curDirLine}<@argP p=curDirLine?string>${curDirLine}</@argP>${curDirLine}\n"
//           + "<#macro m p>(p=${p}){<#nested>}</#macro>\n"
//           + "${curDirLine}<@m p=curDirLine?string>${curDirLine}</@m>${curDirLine}");
//   assertOutputForNamed(
//           "positions.ftl",
//           "[positions.ftl:1:1-1:7]"
//           + "..[positions.ftl:2:3-3:2]"
//           + "[positions.ftl:3:3-3:14]xxx\n"
//           + "[positions.ftl:4:1-4:24]{[positions.ftl:4:7-4:12] [positions.ftl:4:14-4:19]}\n"
//           + "-(p=5){-}-\n"
//           + "-(p=7){-}-");
