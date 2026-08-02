//! Java 测试逻辑的 Rust 1:1 实现 —— 对应 freemarker-core/src/test 与
//! freemarker-jython25/src/test 的测试类（测试方法同名、同断言、错误消息逐字对齐）。
//!
//! 共享辅助：对应 `freemarker-test-utils` 的 `TemplateTest` 基类
//! （applyEnvironmentIndependentDefaults / assertOutput / assertErrorContains /
//! addTemplate / createCommonTestValuesDataModel）。

use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// 测试资源镜像根（java-tests/ 下保留的测试所需文件）
pub const JAVA_TEST_RES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/java-tests");

/// 读取 java-tests 镜像中的资源（rel 形如 "core/src/test/resources/freemarker/core/ast-1.ftl"）
pub fn read_java_resource(rel: &str) -> String {
    std::fs::read_to_string(format!("{JAVA_TEST_RES}/{rel}"))
        .unwrap_or_else(|e| panic!("cannot read java-tests resource {rel}: {e}"))
}

/// 测试配置 —— 对齐 `TemplateTest.getConfiguration`（createConfiguration +
/// applyEnvironmentIndependentDefaults）：locale=US、defaultEncoding=UTF-8、
/// timeZone=GMT+1（Java `TimeZone.getTimeZone("GMT+1")`，.time_zone 读数 "GMT+01:00"）。
/// Java core 测试的 createConfiguration 默认 `new Configuration(VERSION_2_3_0)`
/// —— ICI 2.3.0；本引擎固定 ICI 2.3.34，测试类在 ICI 门控断言处按 jar 实测对齐。
pub fn test_config() -> (Configuration, Arc<StringLoader>) {
    let mut c = Configuration::new();
    c.settings.locale = "en_US".to_string();
    c.settings.time_zone = "Etc/GMT-1"
        .parse()
        .unwrap_or(freemarker::core::TzSetting::Fixed(
            chrono::FixedOffset::east_opt(0).unwrap(),
        ));
    c.settings.time_zone_id = "GMT+01:00".to_string();
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    (c, loader)
}

/// 注册命名模板（对应 `TemplateTest.addTemplate`）
pub fn add_template(loader: &Arc<StringLoader>, name: &str, content: &str) {
    loader.put(name, content);
}

/// 渲染内联模板 —— 对应 `TemplateTest.createTemplate(name=null, ftl)` + process
/// （Java 直接 `new Template(null, ftl, cfg)`，不经模板缓存——本实现同样直连解析）
pub fn render_ftl(c: &Configuration, _loader: &Arc<StringLoader>, ftl: &str) -> String {
    let cfg = std::rc::Rc::new(c.clone());
    let t = freemarker::parser::parse(&cfg, "adhoc", ftl)
        .unwrap_or_else(|e| panic!("parse failed: {e}"));
    let mut out = Vec::new();
    t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    )
    .unwrap_or_else(|e| panic!("process failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}

/// 渲染命名模板（对应 `assertOutputForNamed` 的 getTemplate(name) + process）
pub fn render_named(c: &Configuration, _loader: &Arc<StringLoader>, name: &str) -> String {
    let t = c
        .get_template(name)
        .unwrap_or_else(|e| panic!("get_template({name}) failed: {e}"));
    let mut out = Vec::new();
    t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    )
    .unwrap_or_else(|e| panic!("process({name}) failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}

/// 断言渲染输出（对应 `TemplateTest.assertOutput`：输出逐字相等，不归一化换行）
pub fn assert_output(c: &Configuration, _loader: &Arc<StringLoader>, ftl: &str, expected: &str) {
    let out = render_ftl(c, _loader, ftl);
    assert_eq!(out, expected, "ftl: {ftl}");
}

/// 断言渲染失败，且消息包含全部子串（对应 `TemplateTest.assertErrorContains`；
/// 子串以 `\!` 开头 = 断言**不**包含）。返回消息供进一步逐字断言。
pub fn assert_error_contains(
    c: &Configuration,
    _loader: &Arc<StringLoader>,
    ftl: &str,
    substrings: &[&str],
) -> String {
    let cfg = std::rc::Rc::new(c.clone());
    let t = match freemarker::parser::parse(&cfg, "adhoc", ftl) {
        Ok(t) => t,
        Err(e) => {
            // 解析期错误（Java ParseException → getEditorMessage）
            let msg = e.to_user_message();
            assert_contains_all(&msg, substrings, ftl);
            return msg;
        }
    };
    let mut out = Vec::new();
    match t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    ) {
        Ok(_) => panic!("The template had to fail: {ftl}"),
        Err(e) => {
            // Java TemplateException.getMessageWithoutStackTop（不含 FTL/Java stack 段）
            let msg = e.to_user_message();
            assert_contains_all(&msg, substrings, ftl);
            msg
        }
    }
}

/// 断言渲染失败，且消息与 expected 逐字相等（错误消息精确对齐用）
pub fn assert_error_message_eq(
    c: &Configuration,
    _loader: &Arc<StringLoader>,
    ftl: &str,
    expected: &str,
) {
    let msg = assert_error_contains(c, _loader, ftl, &[]);
    assert_eq!(msg, expected, "ftl: {ftl}");
}

fn assert_contains_all(msg: &str, substrings: &[&str], ftl: &str) {
    for needle in substrings {
        if let Some(rest) = needle.strip_prefix("\\!") {
            if msg.contains(rest) {
                panic!("The message shouldn't contain substring {rest:?}:\n{msg}\nftl: {ftl}");
            }
        } else if !msg.contains(needle) {
            panic!("The message didn't contain substring {needle:?}:\n{msg}\nftl: {ftl}");
        }
    }
}

/// 通用测试数据模型（对应 `TemplateTest.createCommonTestValuesDataModel`：
/// map/list/s/n/b；bean 用 Rust 等价模型替代）
pub fn common_data_model() -> freemarker::template::TModel {
    use freemarker::template::TModel;
    let mut map = indexmap::IndexMap::new();
    map.insert("key".to_string(), TModel::from_scalar("value".to_string()));
    let mut root = indexmap::IndexMap::new();
    root.insert("map".to_string(), TModel::from_hash(map));
    root.insert(
        "list".to_string(),
        TModel::from_sequence(vec![TModel::from_scalar("item".to_string())]),
    );
    root.insert("s".to_string(), TModel::from_scalar("text".to_string()));
    root.insert(
        "n".to_string(),
        TModel::from_number(freemarker::value::TNumber::Int(1)),
    );
    root.insert("b".to_string(), TModel::from_boolean(true));
    TModel::from_hash(root)
}

/// 以通用数据模型渲染内联模板（部分测试 setDataModel(createCommonTestValuesDataModel)）
pub fn render_ftl_with_dm(
    c: &Configuration,
    _loader: &Arc<StringLoader>,
    ftl: &str,
    dm: freemarker::template::TModel,
) -> String {
    let cfg = std::rc::Rc::new(c.clone());
    let t = freemarker::parser::parse(&cfg, "adhoc", ftl)
        .unwrap_or_else(|e| panic!("parse failed: {e}"));
    let mut out = Vec::new();
    t.process(dm, &mut out)
        .unwrap_or_else(|e| panic!("process failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}

/// 断言渲染失败并返回 TemplateError（不检查消息；供精确断言）
pub fn render_error(
    c: &Configuration,
    _loader: &Arc<StringLoader>,
    ftl: &str,
) -> freemarker::error::TemplateError {
    let cfg = std::rc::Rc::new(c.clone());
    let t = freemarker::parser::parse(&cfg, "adhoc", ftl)
        .unwrap_or_else(|e| panic!("parse failed: {e}"));
    let mut out = Vec::new();
    match t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    ) {
        Ok(_) => panic!("The template had to fail: {ftl}"),
        Err(e) => e,
    }
}
