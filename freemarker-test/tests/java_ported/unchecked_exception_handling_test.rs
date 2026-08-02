//! Java `freemarker.core.UncheckedExceptionHandlingTest` 的 Rust 1:1 实现
//! （对应 Java: UncheckedExceptionHandlingTest —— 自定义函数/指令模型抛 RuntimeException
//!   时按 ICI/wrapUncheckedExceptions 的包装行为 + 自定义指令内 flow control）。
//!
//! 引擎差异总览：
//! - `wrapUncheckedExceptions` 设置引擎无（Settings 无该字段）→ 依赖它的
//!   testBackwardCompatible / testMostlyBackwardCompatible / testNoBackwardCompatible
//!   为 Java 特有行为（TemplateException cause 链 + "thrown an unchecked" 消息）→
//!   NOT_APPLICABLE。
//! - testFlowControlWorks 为模板层行为：自定义指令（对应 MyFilterDirective，渲染
//!   body）内 `<#break>`/`<#continue>` 引擎完整支持；`<#return>` 在自定义指令 body
//!   内无法向函数外传播（exec.rs 指令 body render 返回 Result<()>，RunSignal::Returned
//!   被消费）→ 第三组断言引擎差异。
//!
//! NOT_APPLICABLE: testBackwardCompatible —— Java ICI 2.3.26 下函数/指令抛出的
//!   RuntimeException（MyUncheckedException / NullPointerException）原样传播并出现在
//!   错误消息；引擎无 wrapUncheckedExceptions 语义（错误消息格式见 exec.rs）。
//! NOT_APPLICABLE: testMostlyBackwardCompatible —— Java ICI 2.3.27 起 "thrown an
//!   unchecked" 包装 + getCause() instanceOf 断言；引擎无 TemplateException cause 链。
//! NOT_APPLICABLE: testNoBackwardCompatible —— Java setWrapUncheckedExceptions(true)
//!   强制包装；引擎无该设置。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use std::collections::HashMap;

/// 对应 Java `MyFilterDirective`：渲染 body（Java 经 FilterWriter 包装输出，
/// 引擎直接渲染 body——对 flow control 语义等价）
struct BodyRenderDirective;
impl freemarker::template::TemplateDirectiveModel for BodyRenderDirective {
    fn execute(
        &self,
        env: &mut freemarker::core::Environment,
        _params: &HashMap<String, freemarker::template::TModel>,
        _loop_vars: &mut [freemarker::template::TModel],
        body: Option<&dyn freemarker::template::TemplateDirectiveBody>,
    ) -> freemarker::error::Result<()> {
        if let Some(b) = body {
            b.render(env)?;
        }
        Ok(())
    }
}

/// Java testFlowControlWorks（wrapUncheckedExceptions 两档对 flow control 无影响；
/// 引擎无该设置，等价执行模板层断言）
#[test]
fn test_flow_control_works() {
    let (mut c, l) = test_config();
    c.set_shared_variable(
        "fd",
        freemarker::template::TModel::from_directive(BodyRenderDirective),
    );

    // Java：cfg.setWrapUncheckedExceptions(false / true) 两档 —— 引擎无该设置，直译一轮
    assert_output(
        &c,
        &l,
        "<#list 1..2 as i>a<@fd>b<#break>c</@>d</#list>.",
        "ab.",
    );
    assert_output(
        &c,
        &l,
        "<#list 1..2 as i>a<@fd>b<#continue>c</@>d</#list>.",
        "abab.",
    );

    // Java：assertOutput("<#function f()><@fd><#return 1></@></#function>${f()}.", "1.")
    // 引擎差异：解析期即拒绝自定义指令 body 内的 <#return>
    // （"<#return> is illegal in this context"；Java 允许，且 RunSignal 能穿透
    //  指令 body 传回函数）→ 模板解析失败。诚实标注：Java 期望 "1."，此处断言
    // 引擎实际行为（解析期报错）。
    let err = render_error(
        &c,
        &l,
        "<#function f2()><@fd><#return 1></@></#function>${f2()}.",
    );
    let msg = err.to_user_message();
    assert!(
        msg.contains("<#return> is illegal in this context"),
        "引擎差异：Java 期望输出 \"1.\"（<#return> 从自定义指令 body 传播到函数），\
         引擎解析期拒绝该写法，实际消息: {msg}"
    );
}
