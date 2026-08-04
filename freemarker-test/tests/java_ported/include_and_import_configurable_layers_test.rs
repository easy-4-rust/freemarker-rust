//! Java `freemarker.core.IncludeAndImportConfigurableLayersTest` 的 Rust 1:1 实现
//! （对应 Java: IncludeAndImportConfigurableLayersTest —— 三层（Configuration /
//!   TemplateConfiguration / Environment）autoImport/autoInclude 的叠加、冲突
//!   覆盖、去重与 lazyImports/lazyAutoImports 惰性语义）。
//!
//! ENGINE_GAP: 引擎缺少该测试依赖的分层机制：
//!   - `Configuration.autoIncludes`（Java addAutoInclude）无 —— 只有
//!     auto_imports（Vec<(ns, path)>，configuration.rs:27）；
//!   - `Environment.addAutoImport`/`addAutoInclude`（env 层 auto imports，
//!     Java Environment.java:3298-3327）无；
//!   - `TemplateConfiguration.addAutoImport`/`addAutoInclude`（模板配置层，
//!     TemplateConfiguration 无 autoImports/autoIncludes 字段，
//!     template_configuration.rs:13-30）无；
//!   - `lazyImports`/`lazyAutoImports` 设置（Configurable）无（Settings 无该字段；
//!     v1 import 恒立即初始化）。
//!   本测试类全部 6 个方法都验证这些分层的组合语义，无法 1:1 迁移，
//!   全部登记 NOT_APPLICABLE（Java 原文保留为注释）。
//!
//! NOT_APPLICABLE: test3LayerImportNoClashes —— cfg/tc/env 三层 addAutoImport
//!   叠加与 removeAutoImport；引擎仅支持 cfg 层 auto_imports。
//! NOT_APPLICABLE: test3LayerImportClashes —— 三层同名 autoImport 的覆盖顺序
//!   （env 层覆盖 tc 层覆盖 cfg 层）。
//! NOT_APPLICABLE: test3LayerIncludesNoClashes / test3LayerIncludeClashes /
//!   test3LayerIncludesClashes2 —— autoInclude 三层叠加/冲突/去重
//!   （引擎无 autoIncludes，包含 addAutoInclude 的调用）。
//! NOT_APPLICABLE: test3LayerLazyness —— lazyImports/lazyAutoImports 三层矩阵
//!   （引擎无 lazy 设置，import 恒立即初始化）。
//!
//! （Java 辅助方法 test3LayerLazyness(layer, lazyImports, lazyAutoImports,
//!   setLazyAutoImports, expectedOutput) 按 layer 在 Configuration/Template/
//!   Environment 三对象上 setLazyImports/setLazyAutoImports；setLazynessOfConfigurable
//!   为设置包装；addCommonTemplates 注册 main/main2/t1..t3/t1b..t3b.ftl ——
//!   全部依赖上述缺失机制，不逐行保留。）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（IncludeAndImportConfigurableLayersTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java 公共模板（Java 原文 addCommonTemplates）：
//   addTemplate("main.ftl", "In main: ${loaded}");
//   addTemplate("main2.ftl", "In main2: ${loaded}");
//   addTemplate("t1.ftl", "<#global loaded = (loaded!) + 't1;'>T1;");
//   addTemplate("t2.ftl", "<#global loaded = (loaded!) + 't2;'>T2;");
//   addTemplate("t3.ftl", "<#global loaded = (loaded!) + 't3;'>T3;");
//   addTemplate("t1b.ftl", "<#global loaded = (loaded!) + 't1b;'>T1b;");
//   addTemplate("t2b.ftl", "<#global loaded = (loaded!) + 't2b;'>T2b;");
//   addTemplate("t3b.ftl", "<#global loaded = (loaded!) + 't3b;'>T3b;");

// Java test3LayerImportNoClashes（Java 原文，各块格式同）：
//   Configuration cfg = getConfiguration();
//   cfg.addAutoImport("t1", "t1.ftl");
//   TemplateConfiguration tc = new TemplateConfiguration();
//   tc.addAutoImport("t2", "t2.ftl");
//   cfg.setTemplateConfigurations(
//           new ConditionalTemplateConfigurationFactory(new FileNameGlobMatcher("main.ftl"), tc));
//   {
//       Template t = cfg.getTemplate("main.ftl");
//       StringWriter sw = new StringWriter();
//       Environment env = t.createProcessingEnvironment(null, sw);
//       env.addAutoImport("t3", "t3.ftl");
//       env.process();
//       assertEquals("In main: t1;t2;t3;", sw.toString());
//   }
//   { // 无 env 层 → "In main: t1;t2;"
//       ... assertEquals("In main: t1;t2;", sw.toString()); }
//   { // main2.ftl 不匹配 tc（FileNameGlobMatcher("main.ftl")）→ "In main2: t1;t3;"
//       ... assertEquals("In main2: t1;t3;", sw.toString()); }
//   cfg.removeAutoImport("t1");
//   { // → "In main: t2;t3;"
//       ... assertEquals("In main: t2;t3;", sw.toString()); }

// Java test3LayerImportClashes（Java 原文）：
//   cfg.addAutoImport("t1", "t1.ftl"); cfg.addAutoImport("t2", "t2.ftl"); cfg.addAutoImport("t3", "t3.ftl");
//   tc.addAutoImport("t2", "t2b.ftl"); // tc 层覆盖 cfg 层同名
//   env.addAutoImport("t3", "t3b.ftl"); // env 层覆盖 cfg 层同名
//   // main.ftl → "In main: t1;t2b;t3b;"
//   // main2.ftl（tc 不匹配）→ "In main2: t1;t2;t3b;"
//   // 无 env 层 → "In main: t1;t3;t2b;"

// Java test3LayerIncludesNoClashes（Java 原文，各块格式同）：
//   cfg.addAutoInclude("t1.ftl");
//   tc.addAutoInclude("t2.ftl");
//   // main.ftl + env.addAutoInclude("t3.ftl") → "T1;T2;T3;In main: t1;t2;t3;"
//   // 无 env 层 → "T1;T2;In main: t1;t2;"
//   // main2.ftl + env t3 → "T1;T3;In main2: t1;t3;"
//   cfg.removeAutoInclude("t1.ftl"); → "T2;T3;In main: t2;t3;"

// Java test3LayerIncludeClashes（Java 原文）：
//   cfg.addAutoInclude("t1.ftl"); cfg.addAutoInclude("t2.ftl"); cfg.addAutoInclude("t3.ftl");
//   tc.addAutoInclude("t2.ftl");
//   // main.ftl + env t3 → "T1;T2;T3;In main: t1;t2;t3;"
//   // main2.ftl + env t3 → "T1;T2;T3;In main2: t1;t2;t3;"
//   // 无 env 层 → "T1;T3;T2;In main: t1;t3;t2;"
//   // env.addAutoInclude("t1.ftl") → "T3;T2;T1;In main: t3;t2;t1;"

// Java test3LayerIncludesClashes2（Java 原文）：
//   cfg.addAutoInclude("t1.ftl"); cfg.addAutoInclude("t1.ftl"); // 同层重复
//   tc.addAutoInclude("t2.ftl"); tc.addAutoInclude("t2.ftl");
//   env.addAutoInclude("t3.ftl"); env.addAutoInclude("t3.ftl");
//   env.addAutoInclude("t1.ftl"); env.addAutoInclude("t1.ftl");
//   // 期望（去重后）："T2;T3;T1;In main: t2;t3;t1;"

// Java test3LayerLazyness（Java 原文）：
//   for (Class<?> layer : new Class<?>[] { Configuration.class, Template.class, Environment.class }) {
//       test3LayerLazyness(layer, null, null, false, "t1;t2;");
//       test3LayerLazyness(layer, null, null, true, "t1;t2;");
//       test3LayerLazyness(layer, null, false, true, "t1;t2;");
//       test3LayerLazyness(layer, null, true, true, "t2;");
//       test3LayerLazyness(layer, false, null, false, "t1;t2;");
//       test3LayerLazyness(layer, false, null, true, "t1;t2;");
//       test3LayerLazyness(layer, false, false, true, "t1;t2;");
//       test3LayerLazyness(layer, false, true, true, "t2;");
//       test3LayerLazyness(layer, true, null, false, "");
//       test3LayerLazyness(layer, true, null, true, "");
//       test3LayerLazyness(layer, true, false, true, "t1;");
//       test3LayerLazyness(layer, true, true, true, "");
//   }
//   其中 test3LayerLazyness(layer, ...)：
//       dropConfiguration();
//       Configuration cfg = getConfiguration();
//       cfg.addAutoImport("t1", "t1.ftl");
//       Template t = new Template(null, "<#import 't2.ftl' as t2>${loaded!}", cfg);
//       Environment env = t.createProcessingEnvironment(null, sw);
//       按 layer 设置 lazyImports/lazyAutoImports 后 env.process()；
//       assertEquals(expectedOutput, sw.toString());
