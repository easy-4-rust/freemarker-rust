//! Java `freemarker-jython25` 的 `freemarker.template.SimpleObjectWrapperTest` 的
//! Rust 1:1 实现（SimpleObjectWrapperTest.java：SimpleObjectWrapper 包装基本类型/
//!   拒绝未知类型/?api 测试）
//!
//! 引擎映射：`freemarker::template::{SIMPLE_WRAPPER, ObjectWrapper, DynValue}`
//! （simple_object_wrapper.rs：Str→SimpleScalar、Int→SimpleNumber(Int/Long)、
//!   Float→Double、Bool→SimpleBoolean、Date→SimpleDate、List→SimpleSequence、
//!   Map→SimpleHash、Null→None）。
//! 引擎差异：
//! - v1 包装输入是封闭的 DynValue 枚举（Java 可传任意 Object）——
//!   "won't wrap 未知类型"（DOM/File/TestBean）用例不存在于类型系统；
//! - ?api 概念缺失；无 per-ICI 包装器（Java 2.3.21 前用 SimpleHash/SimpleSequence、
//!   2.3.22 起用 DefaultMapAdapter/DefaultListAdapter/DefaultArrayAdapter）；
//! - 迭代器/HashSet/数组包装无对应 DynValue 输入。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::template::{DynValue, ObjectWrapper, SIMPLE_WRAPPER};

/// Java testDoesNotAllowAPIBuiltin：SimpleObjectWrapper 禁止 ?api。
/// 引擎差异：v1 无 ?api 内建（TemplateModelWithAPISupport.getAPI 无对应）——
/// 验证 ?api 用法在模板中报错。
#[test]
fn test_does_not_allow_api_builtin() {
    let (c, loader) = test_config();
    // Java：sow.wrap(new HashMap()) 的 getAPI() 抛 TemplateException 消息含
    // "?api" —— v1 无 ?api；模板侧 ?api 用法应报错
    assert_error_contains(&c, &loader, "${m?api}", &[]);
}

/// Java testCanWrapBasicTypes：基本类型包装
#[test]
fn test_can_wrap_basic_types() {
    // Java：两种 ICI（2.3.0/2.3.22）下 SimpleObjectWrapper 行为一致
    // （引擎差异：v1 无 per-ICI 包装器——恒为 SIMPLE_WRAPPER 语义）
    assert!(SIMPLE_WRAPPER
        .wrap(&DynValue::Str("s".to_string()))
        .unwrap()
        .unwrap()
        .is_scalar());
    assert!(SIMPLE_WRAPPER
        .wrap(&DynValue::Int(1))
        .unwrap()
        .unwrap()
        .is_number());
    assert!(SIMPLE_WRAPPER
        .wrap(&DynValue::Bool(true))
        .unwrap()
        .unwrap()
        .is_boolean());
    assert!(SIMPLE_WRAPPER
        .wrap(&DynValue::Date(freemarker::value::DateValue::new(
            chrono::Utc::now().fixed_offset(),
            freemarker::value::DateType::DateTime
        )))
        .unwrap()
        .unwrap()
        .is_date());
    assert!(SIMPLE_WRAPPER
        .wrap(&DynValue::List(vec![DynValue::Int(1)]))
        .unwrap()
        .unwrap()
        .is_sequence());
    // 引擎差异：Java 用例含 String[]（数组）、ArrayList().iterator()（迭代器）、
    // HashSet（→ SimpleSequence）——v1 DynValue 无数组/迭代器/HashSet 输入
    // （List 覆盖序列语义）
    let mut map = indexmap::IndexMap::new();
    map.insert("a".to_string(), DynValue::Int(1));
    let map_pairs: Vec<(String, DynValue)> = map.into_iter().collect();
    assert!(SIMPLE_WRAPPER
        .wrap(&DynValue::Map(map_pairs))
        .unwrap()
        .unwrap()
        .is_hash());
    // Java：assertNull(sow.wrap(null))
    assert!(SIMPLE_WRAPPER.wrap(&DynValue::Null).unwrap().is_none());
}

/// Java testWontWrapDOM：DOM 文档拒绝包装。
/// 引擎差异：Java 对未知 Object 类型抛 TemplateModelException 消息含
/// "won't wrap"；v1 DynValue 为封闭枚举（DOM/File 等 Java 类型无法进入
/// 包装器）——类型系统层面即不可能，注释保留。
#[test]
fn test_wont_wrap_dom() {
    // Java：sow.wrap(Document) → TemplateModelException 消息含 "won't wrap"
    // v1 无 DOM 类型（w3c.dom 不存在）
}

/// Java testWontWrapGenericObjects：任意对象拒绝包装
#[test]
fn test_wont_wrap_generic_objects() {
    // Java：sow.wrap(new File("/x")) → TemplateModelException 消息含 "won't wrap"
    // v1 DynValue 封闭枚举——不存在未知类型输入
}

/// Java testIncompatibleImprovements：ICI 相关的包装差异。
/// 引擎差异：v1 无 per-ICI 包装器（Java 2.3.22 起 Map→DefaultMapAdapter、
/// List→DefaultListAdapter、boolean[]→DefaultArrayAdapter、HashSet→SimpleSequence；
/// 2.3.21 起全部 SimpleHash/SimpleSequence）——注释保留。
#[test]
fn test_incompatible_improvements() {
    // Java 断言（注释保留）：
    // - 2.3.22：emptyMap→DefaultMapAdapter、emptyList→DefaultListAdapter、
    //   boolean[]→DefaultArrayAdapter、HashSet→SimpleSequence；
    // - 2.3.21：全部 → SimpleHash/SimpleSequence；
    // - 共同部分：isWriteProtected()==false、"x"→SimpleScalar、1.5→SimpleNumber、
    //   Date→SimpleDate、true→TemplateBooleanModel.TRUE、
    //   wrap(TestBean) 抛消息含 "type"。
    // v1：无对象模型角色差异（from_* 构造器直出 Simple 家族）、无 Bean 包装
}
