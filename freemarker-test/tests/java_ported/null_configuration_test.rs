//! Java `freemarker.template.NullConfigurationTest` 的 Rust 1:1 实现
//! （NullConfigurationTest.java：无配置构造 Template 的 NPE 回归测试）
//!
//! Java：`new Template("legacy", new StringReader("foo"))`（两参构造，无
//! Configuration）不得抛异常。引擎映射：v1 解析恒带 Configuration
//! （parser::parse(cfg, name, ftl)）——等价验证"名称 + 文本"构造路径不抛错。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testTemplateNPEBug：无配置构造模板的 NPE 回归
#[test]
fn test_template_npe_bug() {
    let (c, loader) = test_config();
    // Java：new Template("legacy", reader) 不抛异常（历史 NPE 回归）
    // 引擎差异：v1 无两参 Template 构造（恒带配置）——parse 路径等价验证
    let cfg = std::rc::Rc::new(c.clone());
    let t = freemarker::parser::parse(&cfg, "legacy", "foo").expect("解析不应失败");
    let mut out = Vec::new();
    t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8_lossy(&out), "foo");
    let _ = loader;
}
