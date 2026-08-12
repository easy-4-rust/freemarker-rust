//! Java `freemarker.core.TemplateProcessingTracerTest` 的 Rust 1:1 实现
//! （对应 Java: TemplateProcessingTracerTest —— `env.setTemplateProcessingTracer`
//!   注入 tracer，逐元素收集 leaf 元素源码片段与带缩进的元素描述列表，
//!   验证 if/list/else/sep/foreach/attempt/recover/interpret/switch/case/on/
//!   default/macro/assign 的执行轨迹）。
//!
//! ENGINE_GAP: 引擎没有 `TemplateProcessingTracer` API（Java
//!   Environment.setTemplateProcessingTracer + TemplateProcessingTracer 接口 +
//!   TracedElement：getDescription/isLeaf/getBeginLine|Column/getEndLine|Column/
//!   getTemplate + Template.getSource(column,line,column,line)），环境执行栈
//!   不暴露元素级轨迹。本测试唯一方法 test 的全部断言（leafElementSourceSnippets
//!   36 项 + indentedElementDescriptions 84 项）都来自 tracer 回调，无法 1:1
//!   迁移，登记 NOT_APPLICABLE（Java 原文保留为注释，含 TEMPLATE_TEXT 与两个
//!   期望列表）。
//!
//! NOT_APPLICABLE: test —— 依赖 setTemplateProcessingTracer/TracedElement 接口。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（TemplateProcessingTracerTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java 模板文本（Java 原文 TEMPLATE_TEXT）：
//   "<#if 0 == 1>Nope.\n</#if>" +
//   "<#if 1 == 1>Yup.\n</#if>" +
//   "Always.\n" +
//   "<#list [1, 2, 3] as item>\n" +
//   "${item}<#else>\n" +
//   "Nope.\n" +
//   "</#list>\n" +
//   "<#list [] as item>\n" +
//   "${item}<#else>" +
//   "Yup.\n" +
//   "</#list>\n" +
//   "<#list 1..2 as i>${i}</#list>" +
//   "<#list 1..3 as j>${j}<#sep>, </#list>" +
//   "<#foreach k in 1..2>k=${k}</#foreach>" +
//   "<#attempt>succeed<#recover>not visited</#attempt>" +
//   "<#attempt>will fail${fail}<#recover>recover</#attempt>" +
//   "<@('x'?interpret) />" +
//   "<#if true>t<#else>f</#if>" +
//   "<#if false>t<#else>f</#if>" +
//   "<#if false>t1<#elseif false>f1<#else>f2</#if>" +
//   "<#if false>t1<#elseif true>t2<#else>f2</#if>" +
//   "<#switch 2>" +
//   "<#case 1>C1<#break>" +
//   "<#case 2>C2<#break>" +
//   "<#case 3>C3<#break>" +
//   "<#default>D" +
//   "</#switch>" +
//   "<#switch 3>" +
//   "<#case 1>C1<#break>" +
//   "<#case 2>C3<#break>" +
//   "<#default>D" +
//   "</#switch>" +
//   "<#switch 4>" +
//   "<#on 1>O1" +
//   "<#on 4>O4" +
//   "<#default>D" +
//   "</#switch>" +
//   "<#switch 5>" +
//   "<#on 1>O1" +
//   "<#default>OD" +
//   "</#switch>" +
//   "<#macro m>Hello from m!</#macro>" +
//   "Calling macro: <@m />" +
//   "<#assign t>captured</#assign>" +
//   "\n";

// Java test（Java 原文）：
//   Configuration cfg = new Configuration(Configuration.VERSION_2_3_32);
//   Template t = new Template("test.ftl", TEMPLATE_TEXT, cfg);
//   TestTemplateProcessingTracer tracer = new TestTemplateProcessingTracer();
//   Environment env = t.createProcessingEnvironment(null, NullWriter.INSTANCE);
//   env.setTemplateProcessingTracer(tracer);
//   env.process();
//   assertEquals( /* leafElementSourceSnippets */ List.of(
//           "Yup.\n", "Always.\n", "${item}", "${item}", "${item}", "Yup.\n",
//           "${i}", "${i}", "${j}", ", ", "${j}", ", ", "${j}", "k=", "${k}",
//           "k=", "${k}", "succeed", "will fail", "${fail}", "recover",
//           "<@('x'?interpret) />", "x", "t", "f", "f2", "t2", "C2",
//           "<#break>", "D", "O4", "OD", "Calling macro: ", "<@m />",
//           "Hello from m!", "captured", "\n"),
//           tracer.leafElementSourceSnippets);
//   assertEquals( /* indentedElementDescriptions */ List.of(
//           "root",
//           " #if 0 == 1",
//           " #if 1 == 1",
//           "  text \"Yup.\\n\"",
//           " text \"Always.\\n\"",
//           " #list-#else-container",
//           "  #list [1, 2, 3] as item",
//           "   ${item}", "   ${item}", "   ${item}",
//           " #list-#else-container",
//           "  #list [] as item",
//           "  #else",
//           "   text \"Yup.\\n\"",
//           " #list 1..2 as i",
//           "  ${i}", "  ${i}",
//           " #list 1..3 as j",
//           "  ${j}", "  #sep", "   text \", \"", "  ${j}", "  #sep",
//           "   text \", \"", "  ${j}", "  #sep",
//           " #foreach k in 1..2",
//           "  text \"k=\"", "  ${k}", "  text \"k=\"", "  ${k}",
//           " #attempt",
//           "  text \"succeed\"",
//           " #attempt",
//           "  #mixed_content",
//           "   text \"will fail\"", "   ${fail}",
//           "  #recover",
//           "   text \"recover\"",
//           " @(\"x\"?interpret)",
//           "  text \"x\"",
//           " #if-#elseif-#else-container",
//           "  #if true", "   text \"t\"",
//           " #if-#elseif-#else-container",
//           "  #else", "   text \"f\"",
//           " #if-#elseif-#else-container",
//           "  #else", "   text \"f2\"",
//           " #if-#elseif-#else-container",
//           "  #elseif true", "   text \"t2\"",
//           " #switch 2",
//           "  #case 2", "   text \"C2\"", "   #break",
//           " #switch 3",
//           "  #default", "   text \"D\"",
//           " #switch 4",
//           "  #on 4", "   text \"O4\"",
//           " #switch 5",
//           "  #default", "   text \"OD\"",
//           " #macro m",
//           " text \"Calling macro: \"",
//           " @m",
//           "  #macro m", "   text \"Hello from m!\"",
//           " #assign t = .nested_output",
//           "  text \"captured\"",
//           " text \"\\n\""),
//           tracer.indentedElementDescriptions);

// Java TestTemplateProcessingTracer（Java 原文）：
//   enterElement(env, tracedElement)：缩进 +1 空格记录 description；leaf 元素
//     按 begin/end 行列取 getTemplate().getSource(...)（跨行时 end 行=begin 行、
//     end 列=Integer.MAX_VALUE、后缀 "[...]"）追加 leafElementSourceSnippets；
//   exitElement：缩进 -1。两列表即上述断言的输入。
