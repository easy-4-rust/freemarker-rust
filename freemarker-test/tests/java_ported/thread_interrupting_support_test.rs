//! Java `freemarker.core.ThreadInterruptingSupportTest` 的 Rust 1:1 实现
//! （对应 Java: ThreadInterruptingSupportTest —— `_CoreAPI.addThreadInterruptedChecks`
//!   给模板注册线程中断检查后，在另一个线程中渲染，主线程 interrupt() 应使
//!   list 循环/自定义指令循环/宏函数递归等深循环及时以
//!   TemplateProcessingThreadInterruptedException 终止）。
//!
//! ENGINE_GAP: 引擎没有线程中断支持（Java `_CoreAPI.addThreadInterruptedChecks`/
//!   ThreadInterruptionSupportTemplatePostProcessor：list/items/宏调用等热循环
//!   内检查 Thread.currentThread().isInterrupted()，命中抛
//!   TemplateProcessingThreadInterruptedException），v1 渲染为纯同步计算，
//!   无中断检查点。本测试唯一方法 test 的 13 个 assertCanBeInterrupted 用例
//!   全部依赖该机制 + Java 线程（Thread.interrupt/join），无法 1:1 迁移，
//!   登记 NOT_APPLICABLE（Java 原文保留为注释）。
//!
//! NOT_APPLICABLE: test —— 依赖 addThreadInterruptedChecks（引擎无）+ JVM
//!   线程中断机制（Thread.interrupt()/isInterrupted() 语义）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（ThreadInterruptingSupportTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java test（Java 原文）：
//   private static final int TEMPLATE_INTERRUPTION_TIMEOUT = 5000;
//   private final Configuration cfg = new Configuration(Configuration.VERSION_2_3_22);
//
//   assertCanBeInterrupted("<#list 1.. as x></#list>");
//   assertCanBeInterrupted("<#list 1.. as x>${x}</#list>");
//   assertCanBeInterrupted("<#list 1.. as x>t${x}</#list>");
//   assertCanBeInterrupted("<#list 1.. as x><#list 1.. as y>${y}</#list></#list>");
//   assertCanBeInterrupted("<#list 1.. as x>${x}<#else>nope</#list>");
//   assertCanBeInterrupted("<#list 1..>[<#items as x>${x}</#items>]<#else>nope</#list>");
//   assertCanBeInterrupted("<@customLoopDirective />");
//   assertCanBeInterrupted("<@customLoopDirective>x</@>");
//   assertCanBeInterrupted("<@customLoopDirective><#if true>x</#if></@>");
//   assertCanBeInterrupted("<#macro selfCalling><@sleepDirective/><@selfCalling /></#macro><@selfCalling />");
//   assertCanBeInterrupted("<#function selfCalling><@sleepDirective/>${selfCalling()}</#function>${selfCalling()}");
//   assertCanBeInterrupted("<#list 1.. as _><#attempt><@sleepDirective/><#recover>suppress</#attempt></#list>");
//   assertCanBeInterrupted("<#attempt><#list 1.. as _></#list><#recover>suppress</#attempt>");

// Java assertCanBeInterrupted(templateSourceCode)（Java 原文）：
//   TemplateRunnerThread trt = new TemplateRunnerThread(templateSourceCode); // 模板为
//       "<@startedDirective/>" + templateSourceCode，且 _CoreAPI.addThreadInterruptedChecks(template)
//   trt.start();
//   等 startedDirective 置 started=true（synchronized wait/notifyAll）；
//   失败则原样重抛；Thread.sleep(50) 后 trt.interrupt()、trt.join(5000)；
//   assertTrue(trt.isTemplateProcessingInterrupted())
//   —— 即模板在 interrupt 后 5 秒内以 TemplateProcessingThreadInterruptedException
//   结束（run() 捕获该异常置 templateProcessingInterrupted=true）。

// Java TemplateRunnerThread 辅助类（Java 原文）：
//   run()：template.process(this, NullWriter.INSTANCE)，
//     catch TemplateProcessingThreadInterruptedException → templateProcessingInterrupted=true；
//     catch Throwable → failedWith + notifyAll；finally 未 started → IllegalStateException。
//   getStartedDirective()：StartedDirective —— execute 中置 started=true + notifyAll。
//   getCustomLoopDirective()：CustomLoopDirective —— while(true) { body.render(NullWriter); }。
//   getSleepDirective()：SleepDirective —— Thread.sleep(100)，被中断则
//     Thread.currentThread().interrupt()（恢复中断标志）。
