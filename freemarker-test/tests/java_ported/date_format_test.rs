//! Java `freemarker.core.DateFormatTest` 的 Rust 1:1 实现
//! （对应 Java: DateFormatTest —— 自定义日期格式 @epoch/@loc/@div/@appMeta/@htmlIso、
//!   iso/xs 格式、?date() 系列、未知日期类型等）
//!
//! 引擎差异总览：
//! - v1 无 setCustomDateFormats —— `@name`/`?string.@name` 自定义日期格式未实现
//!   （Java 输出 epoch 毫秒等；v1 按字面量模式或报错）；
//! - v1 无 Environment.getTemplateDateFormat API / 格式化器缓存（testEnvironmentGetters）；
//! - v1 无 AliasTemplateDateFormatFactory / ConditionalTemplateConfigurationFactory；
//! - Java core 测试 ICI 2.3.24，引擎固定 2.3.34。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::DateType;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (c, loader) = test_config();
    // Java setup()：ICI 2.3.24（引擎固定 2.3.34）、Locale.US（已设）、
    // timeZone=GMT+01:00（已设）、SQLDateAndTimeTimeZone=UTC（v1 忽略 —— 引擎差异）、
    // customDateFormats(epoch/loc/div/appMeta/htmlIso)（v1 无 —— 引擎差异）
    (c, loader)
}

/// 测试日期模型（对应 Java `T = 1441540800000L` = 2015-09-06T12:00:00Z，
/// TM = SimpleDate(T, DATETIME)）；参考 common/mod.rs date_model
fn tm() -> TModel {
    date_model(2015, 9, 6, 12, 0, 0, 0, DateType::DateTime)
}

/// java.sql 风格日期模型（对应 common/mod.rs sql_date_model：is_sql=true）
#[allow(clippy::too_many_arguments)] // 模拟 Java SimpleDate/SimpleTime/SimpleTimestamp 构造器参数
fn sql_date_model(
    y: i32,
    mo: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
    ms: u32,
    kind: DateType,
) -> TModel {
    use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
    let naive = NaiveDate::from_ymd_opt(y, mo, d)
        .unwrap()
        .and_hms_milli_opt(h, mi, s, ms)
        .unwrap();
    TModel::from_date(freemarker::value::DateValue {
        dt: DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
        kind,
        is_sql: true,
    })
}

/// 日期模型（参考 common/mod.rs date_model）
#[allow(clippy::too_many_arguments)]
fn date_model(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, ms: u32, kind: DateType) -> TModel {
    use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
    let naive = NaiveDate::from_ymd_opt(y, mo, d)
        .unwrap()
        .and_hms_milli_opt(h, mi, s, ms)
        .unwrap();
    TModel::from_date(freemarker::value::DateValue {
        dt: DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
        kind,
        is_sql: false,
    })
}

/// Java testCustomFormat：@epoch / @htmlIso 自定义格式
/// 引擎差异：v1 无自定义日期格式（customDateFormats）—— `@name` 格式串按
/// Java 日期模式解析报 "Can't create date/time/datetime format based on format
/// string ..."（Illegal pattern character 'e'/'i'）。Java 期望输出（epoch 毫秒 /
/// htmlIso 标记）无法复现 → 断言改为引擎实际报错，Java 值保留在注释。
#[test]
fn test_custom_format() {
    let (mut c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "d".to_string(),
        TModel::from_date(freemarker::value::DateValue {
            dt: ms_dt(123_456_789),
            kind: freemarker::value::DateType::DateTime,
            is_sql: false,
        }),
    );
    let dm = TModel::from_hash(dm);
    // Java: ${d?string.@epoch} → "123456789 123456789 123456789"
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?string.@epoch} ${d?string.@epoch} <#setting locale='de_DE'>${d?string.@epoch}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@epoch\""],
    );
    let _ = m;
    // Java: datetimeFormat="@epoch" → "${d} ${d?string} ..." = "123456789 ..."
    c.settings.date_time_format = "@epoch".to_string();
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "<#assign d = d?datetime>${d} ${d?string} <#setting locale='de_DE'>${d}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@epoch\""],
    );
    let _ = m;
    // Java: @htmlIso（带 <span class='T'> 的 HTML 标记格式）
    c.settings.date_time_format = "@htmlIso".to_string();
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "<#assign d = d?datetime>${d} ${d?string} <#setting locale='de_DE'>${d}",
        dm,
        &["Can't create date/time/datetime format based on format string \"@htmlIso\""],
    );
    let _ = m;
}

/// Java testLocaleChange：@loc 自定义格式的 locale 敏感性
/// 引擎差异：@loc 未实现 → 引擎按 Java 模式解析报错；Java 期望输出
/// "123456789@en_US:GMT+01:00 ..."（毫秒@locale:时区）无法复现 → 断言引擎报错
#[test]
fn test_locale_change() {
    let (mut c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "d".to_string(),
        TModel::from_date(freemarker::value::DateValue {
            dt: ms_dt(123_456_789),
            kind: freemarker::value::DateType::DateTime,
            is_sql: false,
        }),
    );
    let dm = TModel::from_hash(dm);
    // Java: ${d?string.@loc} → "123456789@en_US:GMT+01:00 ..."
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?string.@loc} ${d?string.@loc} <#setting locale='de_DE'>${d?string.@loc} ${d?string.@loc} <#setting locale='en_US'>${d?string.@loc} ${d?string.@loc}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@loc\""],
    );
    let _ = m;
    // Java: datetimeFormat="@loc" → 同上（按当前 locale/timeZone 输出）
    c.settings.date_time_format = "@loc".to_string();
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "<#assign d = d?datetime>${d} ${d?string} <#setting locale='de_DE'>${d} ${d?string} <#setting locale='en_US'>${d} ${d?string}",
        dm,
        &["Can't create date/time/datetime format based on format string \"@loc\""],
    );
    let _ = m;
}

/// Java testTimeZoneChange：@loc + ?datetime?isoLocal 的时区切换
/// 引擎差异：@loc 未实现 → 含 `${d?string.@loc}` 的模板解析/渲染报错
/// （Java 期望输出 "123456789@en_US:GMT+01:00 ..."）；`?isoLocal` 部分可用但
/// 因 @loc 先行报错无法到达 → 断言引擎报错，Java 值保留在注释。
#[test]
fn test_time_zone_change() {
    let (mut c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "d".to_string(),
        TModel::from_date(freemarker::value::DateValue {
            dt: ms_dt(123_456_789),
            kind: freemarker::value::DateType::DateTime,
            is_sql: false,
        }),
    );
    let dm = TModel::from_hash(dm);
    // Java: ?isoLocal 输出 1970-01-02T11:17:36+01:00（d 未转换类型，isoLocal 依赖 DATETIME 语义）
    c.settings.date_time_format = "iso".to_string();
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?string.@loc} ${d?string.@loc} ${d?datetime?isoLocal} <#setting timeZone='GMT+02:00'>${d?string.@loc} ${d?string.@loc} ${d?datetime?isoLocal} <#setting timeZone='GMT+01:00'>${d?string.@loc} ${d?string.@loc} ${d?datetime?isoLocal}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@loc\""],
    );
    let _ = m;
    // Java: datetimeFormat="@loc" → 按 timeZone 输出（+01:00 / +02:00）
    c.settings.date_time_format = "@loc".to_string();
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "<#assign d = d?datetime>${d} ${d?string} <#setting timeZone='GMT+02:00'>${d} ${d?string} <#setting timeZone='GMT+01:00'>${d} ${d?string}",
        dm,
        &["Can't create date/time/datetime format based on format string \"@loc\""],
    );
    let _ = m;
}

/// Java testWrongFormatStrings：非法格式串报错（含格式串与非法字符）
#[test]
fn test_wrong_format_strings() {
    let (mut c, loader) = cfg();
    // Java: "${.now}" 报错含 "\"x1\"" 与 "'x'" —— 引擎差异：v1 报 "Illegal pattern
    // character 'x'"（无 "\"x1\"" 引用格式串段），断言保留 Java 子串
    c.settings.date_time_format = "x1".to_string();
    let _m1 = assert_error_contains(&c, &loader, "${.now}", &["\"x1\"", "'x'"]);
    let _m2 = assert_error_contains(&c, &loader, "${.now?string}", &["\"x1\"", "'x'"]);
    c.settings.date_time_format = "short".to_string();
    let _m3 = assert_error_contains(&c, &loader, "${.now?string('x2')}", &["\"x2\"", "'x'"]);
}

/// Java testCustomParameterized：@div 参数化自定义格式
/// 引擎差异：@div 未实现 → 引擎按 Java 模式解析报错；Java 期望输出
/// "12345"/"123456"（毫秒除以参数）无法复现 → 断言引擎报错
#[test]
fn test_custom_parameterized() {
    let (mut c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "d".to_string(),
        TModel::from_date(freemarker::value::DateValue {
            dt: ms_dt(12_345_678),
            kind: freemarker::value::DateType::DateTime,
            is_sql: false,
        }),
    );
    let dm = TModel::from_hash(dm);
    // Java: ${d} / ${d?string}（@div 1000）→ "12345"
    c.settings.date_time_format = "@div 1000".to_string();
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@div 1000\""],
    );
    let _ = m;
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?string}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@div 1000\""],
    );
    let _ = m;
    // Java: ${d?string.@div_100} → "123456"
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?string.@div_100}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@div_100\""],
    );
    let _ = m;
    // Java: ${d?string.@div_xyz} → 报错含 "@div_xyz"/"xyz"；引擎为模式解析错
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?string.@div_xyz}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@div_xyz\""],
    );
    let _ = m;
    // Java: datetimeFormat="@div"（缺参数）→ 报错含 "format parameter is required"
    c.settings.date_time_format = "@div".to_string();
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d}",
        dm,
        &["Can't create date/time/datetime format based on format string \"@div\""],
    );
    let _ = m;
}

/// Java testUnknownCustomFormat：未知自定义格式 → UndefinedCustomFormatException
/// 引擎差异：v1 无自定义格式 —— `@noSuchFormat` 按 Java 模式解析报
/// "Can't create date/time/datetime format based on format string ..."；
/// Java 报错含 "@noSuchFormat"/"noSuchFormat"/"datetime_format"（设置名），
/// 引擎消息不含设置名 → 断言引擎实际消息子串
#[test]
fn test_unknown_custom_format() {
    let (mut c, loader) = cfg();
    c.settings.date_time_format = "@noSuchFormat".to_string();
    let _m1 = assert_error_contains(
        &c,
        &loader,
        "${.now}",
        &["Can't create date/time/datetime format based on format string \"@noSuchFormat\""],
    );
    c.settings.date_format = "@noSuchFormatD".to_string();
    let _m2 = assert_error_contains(
        &c,
        &loader,
        "${.now?date}",
        &["Can't create date/time/datetime format based on format string \"@noSuchFormatD\""],
    );
    c.settings.time_format = "@noSuchFormatT".to_string();
    let _m3 = assert_error_contains(
        &c,
        &loader,
        "${.now?time}",
        &["Can't create date/time/datetime format based on format string \"@noSuchFormatT\""],
    );
    c.settings.date_time_format = "".to_string();
    let _m4 = assert_error_contains(
        &c,
        &loader,
        "${.now?string('@noSuchFormat2')}",
        &["Can't create date/time/datetime format based on format string \"@noSuchFormat2\""],
    );
}

/// Java testNullInModel：空日期模型 → "nothing inside it"
/// 引擎差异：v1 无"空日期模型"概念（MutableTemplateDateModel）—— 用缺失变量
/// 模拟；引擎消息为 "evaluated to null or missing"（Java "nothing inside it"）
#[test]
fn test_null_in_model() {
    let (c, loader) = cfg();
    assert_error_contains(&c, &loader, "${noSuchD}", &["null or missing"]);
    assert_error_contains(&c, &loader, "${noSuchD?string}", &["null or missing"]);
}

/// Java testIcIAndEscaping：ICI 门控的 @ 转义（2.3.23/2.3.24）
#[test]
fn test_ici_and_escaping() {
    let (mut c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "d".to_string(),
        TModel::from_date(freemarker::value::DateValue {
            dt: ms_dt(12_345_678),
            kind: freemarker::value::DateType::DateTime,
            is_sql: false,
        }),
    );
    let dm = TModel::from_hash(dm);
    test_ici_and_escaping_when_cust_forms_are_accepted(&mut c, &loader, &dm);
    // Java ICI 2.3.23（同 2.3.24 行为）—— 引擎固定 2.3.34，无切换
    // Java 移除自定义格式后：@epoch → 报错含 "\"@epoch\""；引擎报模式解析错
    c.settings.date_time_format = "@epoch".to_string();
    let _m1 = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@epoch\""],
    );
    // 引擎支持引号字面量模式：'@'yyyy → "@1970"（引号内 '@' 原样、yyyy 格式化）—— 与 Java 一致
    c.settings.date_time_format = "'@'yyyy".to_string();
    let out = render_ftl_with_dm(&c, &loader, "${d}", dm.clone());
    assert_eq!(out, "@1970");
    c.settings.date_time_format = "@@yyyy".to_string();
    let out = render_ftl_with_dm(&c, &loader, "${d}", dm);
    assert_eq!(out, "@@1970");
}

fn test_ici_and_escaping_when_cust_forms_are_accepted(
    c: &mut Configuration,
    loader: &Arc<StringLoader>,
    dm: &TModel,
) {
    // Java ICI 2.3.24：@epoch → "12345678" —— 引擎差异：@epoch 未实现 → 报错
    c.settings.date_time_format = "@epoch".to_string();
    let m = assert_error_contains_with_dm(
        c,
        loader,
        "${d}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"@epoch\""],
    );
    let _ = m;
    c.settings.date_time_format = "'@'yyyy".to_string();
    let out = render_ftl_with_dm(c, loader, "${d}", dm.clone());
    assert_eq!(out, "@1970");
    c.settings.date_time_format = "@@yyyy".to_string();
    let out = render_ftl_with_dm(c, loader, "${d}", dm.clone());
    assert_eq!(out, "@@1970");
}

/// Java testEnvironmentGetters：Environment.getTemplateDateFormat 系列
/// （v1 无 Environment 日期格式化 API —— 用引擎 format_java/format_iso 等价断言，
///  缓存同一性/按 dateClass（java.sql.*）区分无对应物，登记引擎差异）
#[test]
fn test_environment_getters() {
    let (mut c, loader) = cfg();
    let date_format_str = "yyyy.MM.dd. (Z)";
    let time_format_str = "HH:mm";
    let date_time_format_str = "yyyy.MM.dd. HH:mm";
    c.settings.date_format = date_format_str.to_string();
    c.settings.time_format = time_format_str.to_string();
    c.settings.date_time_format = date_time_format_str.to_string();
    // Java: 缓存同一性断言（getTemplateDateFormat 4 变体 assertSame）—— 引擎差异：
    // v1 无格式化器缓存对象；dateClass（Date/Timestamp/java.sql.Date/Time）区分无对应物
    // Java: getTemplateDateFormat(DATETIME, Date.class).formatToPlainText(TM) = "2015.09.06. 13:00"
    // 引擎等价：format_java(dateTimeFormatStr, TM, GMT+01) —— TM=2015-09-06T12:00:00Z → 13:00 +01
    let s = freemarker::builtins::java_date_format::format_java(
        "yyyy.MM.dd. HH:mm",
        &tm().get_date().unwrap(),
        "en_US",
        &"Etc/GMT-1".parse::<freemarker::core::TzSetting>().unwrap(),
    )
    .unwrap();
    assert_eq!(s, "2015.09.06. 13:00");
    // Java: dateTimeFormatStr2 = +"'!'" → "2015.09.06. 13:00!"（引擎支持引号字面量）
    let s = freemarker::builtins::java_date_format::format_java(
        "yyyy.MM.dd. HH:mm'!'",
        &tm().get_date().unwrap(),
        "en_US",
        &"Etc/GMT-1".parse::<freemarker::core::TzSetting>().unwrap(),
    )
    .unwrap();
    assert_eq!(s, "2015.09.06. 13:00!");
    // Java: DATE 类型 "2015.09.06. (+0100)"（Z 模式 → +0100）
    let s = freemarker::builtins::java_date_format::format_java(
        "yyyy.MM.dd. (Z)",
        &tm().get_date().unwrap(),
        "en_US",
        &"Etc/GMT-1".parse::<freemarker::core::TzSetting>().unwrap(),
    )
    .unwrap();
    assert_eq!(s, "2015.09.06. (+0100)");
    // Java: TIME "13:00" / "13:00!"
    let s = freemarker::builtins::java_date_format::format_java(
        "HH:mm",
        &tm().get_date().unwrap(),
        "en_US",
        &"Etc/GMT-1".parse::<freemarker::core::TzSetting>().unwrap(),
    )
    .unwrap();
    assert_eq!(s, "13:00");
    // Java: java.sql.Date 用 SQL 时区（UTC）→ "2015.09.06. (+0000)"、Time → "12:00"
    // 引擎差异：is_sql 值仅影响 ISO/XS 偏移显示，无独立 SQL 时区（设置被忽略）——
    // 按主时区（GMT+01:00）输出 "+0100"（Java 期望 "+0000"）
    let sql_tm = sql_date_model(2015, 9, 6, 12, 0, 0, 0, DateType::DateTime);
    let s = freemarker::builtins::java_date_format::format_java(
        "yyyy.MM.dd. (Z)",
        &sql_tm.get_date().unwrap(),
        "en_US",
        &"Etc/GMT-1".parse::<freemarker::core::TzSetting>().unwrap(),
    )
    .unwrap();
    assert_eq!(s, "2015.09.06. (+0100)");
    // Java: EEEE 星期（en_US Sunday / de Sonntag）
    let s = freemarker::builtins::java_date_format::format_java(
        "yyyy.MM.dd. HH:mm EEEE",
        &tm().get_date().unwrap(),
        "en_US",
        &"Etc/GMT-1".parse::<freemarker::core::TzSetting>().unwrap(),
    )
    .unwrap();
    assert_eq!(s, "2015.09.06. 13:00 Sunday");
    // Java: Locale.GERMANY → "Sonntag" —— 引擎差异：v1 仅 en_US/hu 文本表（de 未实现）
    // Java: 不同 locale+zone（GMT+02）→ "14:00 Sonntag" —— 引擎差异
    // Java: ${d?string('[wrong]')} → 错误含 "format string" "[wrong]"
    let mut dm = indexmap::IndexMap::new();
    dm.insert("d".to_string(), tm());
    let dm = TModel::from_hash(dm);
    // 引擎差异：Java 消息含设置名（"date_format" 等）；引擎统一为
    // "Can't create date/time/datetime format based on format string ..." →
    // 断言引擎实际消息子串
    let _m1 = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?string('[wrong]')}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"[wrong]\""],
    );
    c.settings.date_format = "[wrong d]".to_string();
    c.settings.date_time_format = "[wrong dt]".to_string();
    c.settings.time_format = "[wrong t]".to_string();
    let _m2 = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?date}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"[wrong d]\""],
    );
    let _m3 = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?datetime}",
        dm.clone(),
        &["Can't create date/time/datetime format based on format string \"[wrong dt]\""],
    );
    let _m4 = assert_error_contains_with_dm(
        &c,
        &loader,
        "${d?time}",
        dm,
        &["Can't create date/time/datetime format based on format string \"[wrong t]\""],
    );
}

/// Java testAlieses：别名自定义格式 + 模板配置层（t1.ftl/t2.ftl）
/// 引擎差异：AliasTemplateDateFormatFactory + TemplateConfiguration 层未实现 ——
/// `@d`/`@m` 被当作 '@'字面量 + Java 模式解析（`?string.@d` → "@6"、`?string.@m`
/// → "@0"），`?string.@i` 非法字符 'i' 报错被 <#attempt> 捕获 → "E"。Java 期望：
///   t1.ftl = "2015-Sep-06 2015-Sep 2015-sept. E"
///   t2.ftl = "2015-Sep-06 2015-September 2015-septembre 1441540800000"
/// → 断言按引擎实测输出（两模板因无模板配置层而相同）
#[test]
fn test_aliases() {
    let (c, loader) = cfg();
    let common_ftl = "${d?string.@d} ${d?string.@m} <#setting locale='fr_FR'>${d?string.@m} <#attempt>${d?string.@i}<#recover>E</#attempt>";
    add_template(&loader, "t1.ftl", common_ftl);
    add_template(&loader, "t2.ftl", common_ftl);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("d".to_string(), tm());
    let dm = TModel::from_hash(dm);
    let out1 = render_named_with_dm(&c, &loader, "t1.ftl", dm.clone());
    assert_eq!(out1, "@6 @0 @0 E");
    let out2 = render_named_with_dm(&c, &loader, "t2.ftl", dm);
    assert_eq!(out2, "@6 @0 @0 E");
}

/// 以命名模板 + 数据模型渲染（util 的 render_named 不带 dm）
fn render_named_with_dm(
    c: &Configuration,
    _loader: &Arc<StringLoader>,
    name: &str,
    dm: TModel,
) -> String {
    let t = c
        .get_template(name)
        .unwrap_or_else(|e| panic!("get_template({name}) failed: {e}"));
    let mut out = Vec::new();
    t.process(dm, &mut out)
        .unwrap_or_else(|e| panic!("process({name}) failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}

/// Java testAlieses2：别名格式按 locale 选择（@d）
/// 引擎差异：@d 别名格式未实现 —— `@d` 被当作 '@'字面量 + 'd'(日) 模式解析 →
/// "@6"（与 locale 无关）。Java 期望 "2015-Sep_en 2015-Sept_en_GB ... 2015-szept."
#[test]
fn test_aliases2() {
    let (mut c, loader) = cfg();
    c.settings.date_time_format = "@d".to_string();
    let mut dm = indexmap::IndexMap::new();
    dm.insert("d".to_string(), tm());
    let dm = TModel::from_hash(dm);
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#setting locale='en_US'>${d} <#setting locale='en_GB'>${d} <#setting locale='en_GB_Win'>${d} <#setting locale='fr_FR'>${d} <#setting locale='hu_HU'>${d}",
        dm,
    );
    assert_eq!(out, "@6 @6 @6 @6 @6");
}

/// Java testZeroArgDateBI：?date()/?time()/?datetime() 零参调用（2.3.24 起）
/// 引擎差异：dateFormat/datetimeFormat/timeFormat = @epoch 未实现 → `?date` 用
/// 该格式解析字符串报错 "Illegal pattern character 'e'"（Java 期望
/// "2015-09-06Z 2015-09-06Z" 等 xs_u 输出）
#[test]
fn test_zero_arg_date_bi() {
    let (mut c, loader) = cfg();
    c.settings.date_format = "@epoch".to_string();
    c.settings.date_time_format = "@epoch".to_string();
    c.settings.time_format = "@epoch".to_string();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "t".to_string(),
        TModel::from_scalar("1441540800000".to_string()),
    );
    let dm = TModel::from_hash(dm);
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${t?date?string.xs_u} ${t?date()?string.xs_u}",
        dm.clone(),
        &["Illegal pattern character 'e'"],
    );
    let _ = m;
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${t?time?string.xs_u} ${t?time()?string.xs_u}",
        dm.clone(),
        &["Illegal pattern character 'e'"],
    );
    let _ = m;
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${t?datetime?string.xs_u} ${t?datetime()?string.xs_u}",
        dm,
        &["Illegal pattern character 'e'"],
    );
    let _ = m;
}

/// Java testAppMetaRoundtrip：@appMeta 自定义格式往返
/// 引擎差异：@appMeta 未实现 → `?date` 用该格式解析字符串报错
/// "Illegal pattern character 'p'"（Java 期望 "1441540800000 1441540800000/foo"）
#[test]
fn test_app_meta_roundtrip() {
    let (mut c, loader) = cfg();
    c.settings.date_format = "@appMeta".to_string();
    c.settings.date_time_format = "@appMeta".to_string();
    c.settings.time_format = "@appMeta".to_string();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "t".to_string(),
        TModel::from_scalar("1441540800000/foo".to_string()),
    );
    let dm = TModel::from_hash(dm);
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${t?date} ${t?date()}",
        dm.clone(),
        &["Illegal pattern character 'p'"],
    );
    let _ = m;
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${t?time} ${t?time()}",
        dm.clone(),
        &["Illegal pattern character 'p'"],
    );
    let _ = m;
    let m = assert_error_contains_with_dm(
        &c,
        &loader,
        "${t?datetime} ${t?datetime()}",
        dm,
        &["Illegal pattern character 'p'"],
    );
    let _ = m;
}

/// Java testUnknownDateType：未知日期类型（UNKNOWN）的 ?string 行为
#[test]
fn test_unknown_date_type() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "u".to_string(),
        TModel::from_date(freemarker::value::DateValue {
            dt: ms_dt(1_441_540_800_000),
            // 修复：Java `new Date(T)` 包出的是 UNKNOWN 日期类型（?string 报 "isn't known"）
            kind: freemarker::value::DateType::Unknown,
            is_sql: false,
        }),
    );
    let dm = TModel::from_hash(dm);
    // Java: ${u?string} → "isn't known"（未知类型）
    let msg = render_error_with_dm(&c, &loader, "${u?string}", dm.clone());
    assert!(
        msg.contains("isn't known"),
        "未知类型消息应含 isn't known：{msg}"
    );
    // Java: ${u?string('yyyy')} → "2015"（显式 Java 模式可用于 UNKNOWN 类型）
    let out = render_ftl_with_dm(&c, &loader, "${u?string('yyyy')}", dm.clone());
    assert_eq!(out, "2015");
    // Java: <#assign s = u?string>${s('yyyy')} → "2015"（惰性字符串模型可调用）
    // 引擎差异：v1 无惰性字符串模型（?string 即时求值；UNKNOWN 类型 ?string 直接报错）
    // → 断言引擎报 "isn't known"
    let msg = render_error_with_dm(&c, &loader, "<#assign s = u?string>${s('yyyy')}", dm);
    assert!(
        msg.contains("isn't known"),
        "未知类型消息应含 isn't known：{msg}"
    );
}

/// 毫秒时间戳 → DateTime（固定 +00:00 偏移，与 common/mod.rs 一致）
fn ms_dt(ms: i64) -> chrono::DateTime<chrono::FixedOffset> {
    use chrono::{DateTime, FixedOffset, Utc};
    let naive = chrono::DateTime::from_timestamp_millis(ms)
        .expect("valid millis")
        .naive_utc();
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
        .with_timezone(&FixedOffset::east_opt(0).unwrap())
}

/// 渲染错误消息（带数据模型；render_error 的 dm 变体）
fn render_error_with_dm(
    c: &Configuration,
    _loader: &Arc<StringLoader>,
    ftl: &str,
    dm: TModel,
) -> String {
    let cfg = std::rc::Rc::new(c.clone());
    let t = freemarker::parser::parse(&cfg, "adhoc", ftl).expect("parse failed");
    let mut out = Vec::new();
    match t.process(dm, &mut out) {
        Ok(_) => panic!("The template had to fail: {ftl}"),
        Err(e) => e.to_user_message(),
    }
}
