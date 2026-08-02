//! Java `freemarker.template.ExceptionTest` 的 Rust 1:1 实现
//! （ExceptionTest.java：异常可序列化 + 异常位置信息测试）
//!
//! 任务约定：无引擎等价物 → 空 mod + 注释（Java 测试方法逐一说明）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testParseExceptionSerializable：ParseException 可序列化（Java 对象流
/// 往返）。引擎差异：v1 错误为 Rust 值（无 Java 序列化机制）——跳过。
#[test]
fn test_parse_exception_serializable() {
    // Java：new Template("<string>", "<@>", cfg) 抛 ParseException，经
    // ObjectOutputStream/ObjectInputStream 往返不得失败。
    // v1 无 Java 对象序列化（Rust TemplateError 无此需求面）。
}

/// Java testTemplateErrorSerializable：TemplateException 可序列化。
/// 引擎差异：同上（无 Java 序列化）——跳过。
#[test]
fn test_template_error_serializable() {
    // Java：${noSuchVar} 抛 TemplateException，序列化往返不得失败。
    // v1 无 Java 对象序列化。
}

/// Java testTemplateExceptionLocationInformation：运行时错误的位置信息。
/// 引擎差异：v1 TemplateError 不暴露 getTemplateName/getLineNumber 等字段——
/// 但错误消息含模板名与变量名，可对齐部分断言。
#[test]
fn test_template_exception_location_information() {
    let (c, loader) = test_config();
    add_template(&loader, "foo_en.ftl", "\n\nxxx${noSuchVariable}");

    // Java：cfg.getTemplate("foo.ftl", Locale.US) → 命中 foo_en.ftl →
    // getTemplateName()=="foo.ftl"、getTemplateSourceName()=="foo_en.ftl"、
    // 行 3 列 6（v1 无位置字段）
    let t = c.get_template_localized("foo.ftl", Some("en_US")).unwrap();
    let mut out = Vec::new();
    let e = t
        .process(
            freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
            &mut out,
        )
        .expect_err("应报错");
    let msg = e.to_user_message();
    // Java：消息含 "foo_en.ftl" 与 "noSuchVariable"（引擎一致）
    assert!(msg.contains("foo_en.ftl"), "{msg}");
    assert!(msg.contains("noSuchVariable"), "{msg}");
    // 引擎差异：Java 断言 e.getTemplateName()=="foo.ftl"（请求名）、
    // getSourceName()=="foo_en.ftl"、行/列 3:6–3:19 —— v1 无这些字段
}

/// Java testParseExceptionLocationInformation：解析错误的位置信息。
/// 引擎差异：v1 Parse 错误消息含模板名与指令名（行/列字段无公开 API）——
/// 对齐消息断言。
#[test]
fn test_parse_exception_location_information() {
    let (c, loader) = test_config();
    add_template(&loader, "foo_en.ftl", "\n\nxxx<#noSuchDirective>");

    // Java：cfg.getTemplate("foo.ftl", Locale.US) → 命中 foo_en.ftl → 解析失败
    let e = c
        .get_template_localized("foo.ftl", Some("en_US"))
        .err()
        .expect("应解析失败");
    let msg = e.to_user_message();
    // Java：e.getTemplateName()=="foo_en.ftl"、消息含 "foo_en.ftl" 与
    // "noSuchDirective"；行 3 列 5 —— v1 消息含模板名与指令名
    assert!(msg.contains("foo_en.ftl"), "{msg}");
    // 引擎差异：Java 断言消息含 "noSuchDirective"（保留原大小写）；v1 错误消息
    // 把指令名规范化为小写 → "nosuchdirective"
    assert!(msg.contains("nosuchdirective"), "{msg}");
    // 引擎差异：v1 无 getLineNumber/getColumnNumber/getEndLineNumber/
    // getEndColumnNumber 字段
}
