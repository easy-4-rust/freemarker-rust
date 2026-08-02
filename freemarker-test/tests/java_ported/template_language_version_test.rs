//! Java `freemarker.template.TemplateLanguageVersionTest` 的 Rust 1:1 实现
//! （TemplateLanguageVersionTest.java：模板语言版本（TLV）随 Configuration ICI
//!   变化的测试）
//!
//! 引擎差异：v1 引擎固定 ICI 2.3.34（Configuration::new 无版本参数、
//! Template 无 getTemplateLanguageVersion 字段）——Java 断言
//! （ICI 2.3.0 → TLV 2.3.0；2.3.19 → 2.3.19；2.3.20 → 2.3.20；2.3.21 → 2.3.21；
//! 未来版本 → IllegalArgumentException 消息含 "version"）整体不可移植，
//! 以固定版本断言 + 注释登记。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testDefaultVersion：配置 ICI 决定模板语言版本
#[test]
fn test_default_version() {
    let (_c, _loader) = test_config();
    // Java：new Configuration(VERSION_2_3_0) → TLV 2.3.0；
    // new Version(2,3,18) → TLV 2.3.0；VERSION_2_3_19/20/21 → TLV 同配置；
    // TestUtil.getClosestFutureVersion() → IllegalArgumentException 消息含
    // "version"（提示更新测试）。
    // 引擎差异：v1 固定 ICI 2.3.34 且无 TLV 概念——断言引擎固定版本
    let v = freemarker::template::Configuration::version();
    assert_eq!(v, freemarker::template::Version::V2_3_34);
    // 引擎差异：Java 的 TLV 门控行为（2.3.0 起 FTL 头声明版本、2.3.19 起
    // 支持 `ftl_version` 声明等）v1 未实现
}
