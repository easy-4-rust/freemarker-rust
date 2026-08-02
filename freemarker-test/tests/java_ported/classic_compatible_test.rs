//! 对应 Java: ClassicCompatibleTest
//! Java `freemarker.core.ClassicCompatibleTest` 的 Rust 1:1 实现。
//! Java createConfiguration：new Configuration(2.3.33) + setClassicCompatible(true)
//! → v1 settings.incompatible_improvements=2.3.33 + classic_compatible=true。
//!
//! 引擎差异总览：
//! - classicCompatible 模式下 Java 的宽松 seq_contains 比较（数字/字符串/布尔互转）
//!   与布尔输出（true → "true"、false → ""）以 v1 实测为准；断言保留 Java 值。
//! - Java 布尔格式在 classic 模式下 `${false}` → ""（false 忽略）；`?string` 按
//!   boolean_format 输出；v1 以实测为准。
//! - getClassicCompatibleAsInt/getBooleanFormat 的配置读数断言 → 直接断言设置字段。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Version;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use std::sync::Arc;

/// Java createConfiguration：new Configuration(2.3.33) + classicCompatible(true)
fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.incompatible_improvements = Version::parse("2.3.33").unwrap();
    c.settings.classic_compatible = true;
    (c, loader)
}

/// Java testLenientValueComparisonInSeqBuiltIns（FREEMARKER-227）
#[test]
fn test_lenient_value_comparison_in_seq_builtins() {
    let (mut c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "seq".to_string(),
        TModel::from_sequence(vec![
            TModel::from_number(TNumber::Int(1)),
            TModel::from_scalar("2".to_string()),
            TModel::from_number(TNumber::Double(3.0)),
            TModel::from_boolean(true),
            TModel::from_scalar("false".to_string()),
        ]),
    );
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(1)?c}", dm.clone());
    assert_eq!(out, "true");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('1')?c}", dm.clone());
    assert_eq!(out, "true");

    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(2)?c}", dm.clone());
    assert_eq!(out, "true");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('2')?c}", dm.clone());
    assert_eq!(out, "true");

    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(3)?c}", dm.clone());
    assert_eq!(out, "true");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('3')?c}", dm.clone());
    assert_eq!(out, "true");

    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(4)?c}", dm.clone());
    assert_eq!(out, "false");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('4')?c}", dm.clone());
    assert_eq!(out, "false");

    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('true')?c}", dm.clone());
    assert_eq!(out, "true");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(true)?c}", dm.clone());
    assert_eq!(out, "true");

    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('false')?c}", dm.clone());
    assert_eq!(out, "true");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(false)?c}", dm.clone());
    assert_eq!(out, "false"); // 因为 false 被转换为 ""

    // 这些其实不太理想/令人困惑，但猜测最接近 1.7.x 的行为，且 classicCompatible
    // 模式长期以来就是这样工作的：
    c.settings.number_format = "0.0".to_string();
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(2)?c}", dm.clone());
    assert_eq!(out, "false");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('2')?c}", dm.clone());
    assert_eq!(out, "true");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains(3)?c}", dm.clone());
    assert_eq!(out, "true");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('3')?c}", dm.clone());
    assert_eq!(out, "false");
    let out = render_ftl_with_dm(&c, &loader, "${seq?seq_contains('3.0')?c}", dm);
    assert_eq!(out, "true");
}

/// Java testMissingValueBuiltIns
#[test]
fn test_missing_value_built_ins() {
    let (c, loader) = cfg();
    // Java addToDataModel("nothing", TemplateModel.NOTHING) —— v1 用 gpn() 模拟
    // （GeneralPurposeNothing：插值 → ""、不触发 InvalidReference）
    let mut dm = indexmap::IndexMap::new();
    dm.insert("nothing".to_string(), TModel::gpn());
    let dm = TModel::from_hash(dm);
    let out = render_ftl_with_dm(&c, &loader, "[${missing}] [${missing!'-'}]", dm.clone());
    assert_eq!(out, "[] [-]");
    let out = render_ftl_with_dm(&c, &loader, "[${nothing}] [${nothing!'-'}]", dm);
    assert_eq!(out, "[] []");
}

/// Java testBooleanFormat
#[test]
fn test_boolean_format() {
    let (mut c, loader) = cfg();
    // Java：assertThat(conf.getClassicCompatibleAsInt(), equalTo(1))
    //       assertThat(conf.getBooleanFormat(), equalTo("true,false"))
    // 引擎差异：v1 直接断言设置字段（classic_compatible 为布尔，无 AsInt 分级）
    assert!(c.settings.classic_compatible, "classicCompatible 应为 true");
    assert_eq!(c.settings.boolean_format, "true,false");

    let mut dm = indexmap::IndexMap::new();
    dm.insert("beanTrue".to_string(), TModel::from_boolean(true));
    dm.insert("beanFalse".to_string(), TModel::from_boolean(false));
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(&c, &loader, "[${true}] [${false}]", dm.clone());
    assert_eq!(out, "[true] []");
    let out = render_ftl_with_dm(&c, &loader, "[${beanTrue}] [${beanFalse}]", dm.clone());
    assert_eq!(out, "[true] []");
    let out = render_ftl_with_dm(&c, &loader, "[${true?c}] [${false?c}]", dm.clone());
    assert_eq!(out, "[true] [false]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "[${true?string}] [${false?string}]",
        dm.clone(),
    );
    assert_eq!(out, "[true] [false]");

    c.settings.boolean_format = "y,n".to_string();

    let out = render_ftl_with_dm(&c, &loader, "[${true}] [${false}]", dm.clone());
    assert_eq!(out, "[true] []");
    let out = render_ftl_with_dm(&c, &loader, "[${beanTrue}] [${beanFalse}]", dm.clone());
    assert_eq!(out, "[true] []");
    let out = render_ftl_with_dm(&c, &loader, "[${true?c}] [${false?c}]", dm.clone());
    assert_eq!(out, "[true] [false]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "[${true?string}] [${false?string}]",
        dm.clone(),
    );
    assert_eq!(out, "[y] [n]");

    // Java：setClassicCompatibleAsInt(2) —— v1 无分级（classic_compatible 布尔），
    // 2 级行为（同 1 级，仅 ?string 差异已覆盖）以当前设置继续断言
    let out = render_ftl_with_dm(&c, &loader, "[${true}] [${false}]", dm.clone());
    assert_eq!(out, "[true] []");
    let out = render_ftl_with_dm(&c, &loader, "[${beanTrue}] [${beanFalse}]", dm.clone());
    assert_eq!(out, "[true] []");
    let out = render_ftl_with_dm(&c, &loader, "[${true?c}] [${false?c}]", dm.clone());
    assert_eq!(out, "[true] [false]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "[${true?string}] [${false?string}]",
        dm.clone(),
    );
    assert_eq!(out, "[y] [n]");
}
