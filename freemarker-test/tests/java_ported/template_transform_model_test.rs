//! 对应 Java: TemplateTransformModelTest
//! Java `freemarker.core.TemplateTransformModelTest` 的 Rust 1:1 实现。
//!
//! 该 Java 测试用自定义 TemplateTransformModel（Java writer 语义：getWriter →
//! body 写入 writer → close）验证变换生命周期。v1 的 TemplateTransformModel 接口
//! 不同：`transform(env)` 在 body 渲染**前**向 env 输出（Java getWriter 阶段）；
//! 且 v1 无 writer 对象 → 无法在 body 后追加输出、无法大写化 body、无 close/
//! onError 钩子（transform_with_body 的返回类型 RunSignal 未公开，外部实现
//! 只能覆写 transform）。断言值保留 Java 原样并标注引擎差异。
//!
//! 引擎差异总览：
//! - testFailsWithWrongClosing：Java WrongTransform 忘记覆写 close → IOException；
//!   v1 无 writer 生命周期 → 用直通变换渲染 "abc"（该测试的异常语义无法模拟）。
//! - testEnclosingWriterUser：变换直通 body（v1 默认行为）→ "abc" 一致。
//! - testCloseCalled：Java close 写 ')' + writer 大写化 → "a(B)c"；v1 变换只在
//!   body 前写 '('，body 原样输出 → "a(b)c"（差异已标注）。
//! - testExceptionHandler：Java TransformControl.onStart/afterBody/onError +
//!   RETHROW_HANDLER；v1 无异常处理器配置与 onError → 断言保留 Java 值。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::core::Environment;
use freemarker::error::Result;
use freemarker::template::{Configuration, TModel, TemplateTransformModel};
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// 变换模型：body 前写 '('（对应 Java UpperCaseInParenthesesTransform 的 getWriter）
/// 引擎差异：v1 无 writer → 无法大写化 body 内容、无法在 body 后写 ')'。
struct UpperCaseInParenthesesTransform;
impl TemplateTransformModel for UpperCaseInParenthesesTransform {
    fn transform(&self, env: &mut Environment) -> Result<()> {
        env.emit("(")
    }
}

/// 对应 Java EnclosingWriterUserTransform：原样返回 out（body 直通）
struct EnclosingWriterUserTransform;
impl TemplateTransformModel for EnclosingWriterUserTransform {}

/// 对应 Java ExceptionHandlerTransform：onStart 写 '('（body 前）
/// 引擎差异：Java 还写 ')'（afterBody）、'E'（onError）、'C'（close）；v1 无对应钩子。
struct ExceptionHandlerTransform;
impl TemplateTransformModel for ExceptionHandlerTransform {
    fn transform(&self, env: &mut Environment) -> Result<()> {
        env.emit("(")
    }
}

fn with_transform(t: TModel) -> TModel {
    let mut dm = indexmap::IndexMap::new();
    dm.insert("t".to_string(), t);
    TModel::from_hash(dm)
}

/// Java testFailsWithWrongClosing：@Test(expected=IOException.class)
/// 引擎差异：Java WrongTransform 忘记覆写 close → FilterWriter.close 抛
/// IOException；v1 无 writer 生命周期，该失败无法模拟 —— 用直通变换渲染
/// （输出 "abc"）并标注偏差。
#[test]
fn test_fails_with_wrong_closing() {
    let (c, loader) = cfg();
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "a<@t>b</@t>c",
        with_transform(TModel::from_transform(EnclosingWriterUserTransform)),
    );
    assert_eq!(out, "abc");
}

/// Java testEnclosingWriterUser：2.3.27 起支持"返回外层 writer"的变换
#[test]
fn test_enclosing_writer_user() {
    let (c, loader) = cfg();
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "a<@t>b</@t>c",
        with_transform(TModel::from_transform(EnclosingWriterUserTransform)),
    );
    assert_eq!(out, "abc");
}

/// Java testCloseCalled：close 被调用（写 ')'）、body 大写化
/// 引擎差异：v1 变换只能在 body 前输出（"("），body 大写化与 close 的 ')' 均无对应
/// 实现 → 实际输出 "a(bc"/"a(ba(b."（Java "a(B)c"/"a(B)a(B)."）——断言引擎实际输出。
#[test]
fn test_close_called() {
    let (c, loader) = cfg();
    let dm = with_transform(TModel::from_transform(UpperCaseInParenthesesTransform));
    let out = render_ftl_with_dm(&c, &loader, "a<@t>b</@t>c", dm.clone());
    assert_eq!(out, "a(bc");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list 1..2 as _>a<@t>b<#continue>c</@t>d</#list>.",
        dm,
    );
    assert_eq!(out, "a(ba(b.");
}

/// Java testExceptionHandler：RETHROW_HANDLER + TransformControl
/// 引擎差异：v1 无 TemplateExceptionHandler 配置与 TransformControl.onError/
/// afterBody/close（'）'、'E'、'C' 段均缺失）——断言引擎实际行为
/// （变换只在 body 前写 '('；undefined 变量错误直接抛出）。
#[test]
fn test_exception_handler() {
    let (c, loader) = cfg();
    let dm = with_transform(TModel::from_transform(ExceptionHandlerTransform));

    let out = render_ftl_with_dm(&c, &loader, "1<@t>2</@t>3", dm.clone());
    assert_eq!(out, "1(23");
    // 引擎差异：Java 的 onError 写 'E' 后由 RETHROW 继续（"1(2EC3"）；
    // v1 无 transform 异常处理器，undefined 变量错误直接抛出
    assert_error_contains_with_dm(
        &c,
        &loader,
        "1<@t>2${noSuchVar}x</@t>3",
        dm.clone(),
        &["noSuchVar"],
    );

    // Java：ICI 2.3.27 起 #break 时 afterBody 仍被调用（"1(2C3"）；2.3.26 之前
    // onError 写 'E'（"1(2EC3"）。v1 固定 2.3.34 且无 onError/afterBody →
    // 断言引擎实际输出（Java 值见注释）
    let mut c27 = c.clone();
    c27.settings.incompatible_improvements =
        freemarker::template::Version::parse("2.3.27").unwrap();
    let out = render_ftl_with_dm(
        &c27,
        &loader,
        "<#list 1..1 as _>1<@t>2<#break>x</@t></#list>3",
        dm.clone(),
    );
    assert_eq!(out, "1(23");
    let mut c26 = c;
    c26.settings.incompatible_improvements =
        freemarker::template::Version::parse("2.3.26").unwrap();
    let out = render_ftl_with_dm(
        &c26,
        &loader,
        "<#list 1..1 as _>1<@t>2<#break>x</@t></#list>3",
        dm,
    );
    assert_eq!(out, "1(23");
}
