//! 黄金套件测试辅助 —— 对应 Java `TemplateTestCase.java`（templatesuite 数据模型构造 +
//! assert/assertEquals/assertFails/noOutput 指令 + 设置应用；docs/11 §3）

use freemarker::cache::StringLoader;
use freemarker::core::{compare_models, CmpOp};
use freemarker::error::{Result, TemplateError};
use freemarker::template::{Configuration, TModel, TemplateDirectiveBody, TemplateDirectiveModel};
use freemarker::value::{DateType, DateValue, TNumber};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

pub const SUITE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/suite");

/// 读取套件文件（模板/expected）
pub fn read_suite(rel: &str) -> String {
    std::fs::read_to_string(format!("{SUITE_DIR}/{rel}"))
        .unwrap_or_else(|e| panic!("cannot read {SUITE_DIR}/{rel}: {e}"))
}

/// 剥掉 expected 文件开头的 `/* ... */` 许可证注释块（Java 侧同样先剥除后比较；
/// 对应 FileTestCase.assertExpectedFileEqualsString 的 CopyrightCommentRemover）
pub fn strip_license_comment(s: &str) -> String {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix("/*") {
        if let Some(i) = rest.find("*/") {
            let out = &rest[i + 2..];
            // 兼容 CRLF 行尾（stringliteral.txt 等用例为 CRLF）
            return out
                .strip_prefix("\r\n")
                .or_else(|| out.strip_prefix('\n'))
                .unwrap_or(out)
                .to_string();
        }
    }
    s.to_string()
}

/// 配置 + StringLoader（Java TemplateTestCase.setUp：locale=US、时区 GMT+1、UTF-8）
pub fn base_config() -> (Configuration, Arc<StringLoader>) {
    let mut c = Configuration::new();
    c.settings.locale = "en_US".to_string();
    c.settings.time_zone = "Etc/GMT-1"
        .parse()
        .unwrap_or(freemarker::core::TzSetting::Fixed(
            chrono::FixedOffset::east_opt(0).unwrap(),
        ));
    // Java TemplateTestCase.java:146 setTimeZone(TimeZone.getTimeZone("GMT+1")) →
    // getID() = "GMT+01:00"（`.time_zone` 内置变量读数；Etc/GMT-1 仅时刻计算等价）
    c.settings.time_zone_id = "GMT+01:00".to_string();
    c.settings.whitespace_stripping = true;
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    (c, loader)
}

/// 注册用例模板与依赖模板（Java FileTemplateLoader(templates 目录) 的等价物：
/// 预注册全部依赖模板，避免 include/import 相对路径解析失败；
/// 模板经 removeFTLCopyrightComment 处理（Java CopyrightCommentRemoverTemplateLoader）
pub fn load_all_templates(loader: &Arc<StringLoader>) {
    let dir = format!("{SUITE_DIR}/templates");
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    collect_ftl(std::path::Path::new(&dir), &mut files);
    for f in files {
        let rel = f
            .strip_prefix(&dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        loader.put(&rel, &remove_ftl_copyright_comment(&text));
    }
}

/// 移除模板开头的版权注释（Java TestUtil.removeFTLCopyrightComment：找到包含
/// "copyright" 的 `<#-- ... -->`/`[#-- ... --]` 注释，连注释后的换行与下一字符一并移除）
pub fn remove_ftl_copyright_comment(ftl: &str) -> String {
    let lower = ftl.to_ascii_lowercase();
    let copyright_idx = match lower.find("copyright") {
        Some(i) => i,
        None => return ftl.to_string(),
    };
    let before = &ftl[..copyright_idx];
    let ab_start = before.rfind("<#--");
    let sb_start = before.rfind("[#--");
    let (comment_first_idx, end_marker) = match (ab_start, sb_start) {
        (Some(a), Some(b)) if b > a => (b, "--]"),
        (Some(a), _) => (a, "-->"),
        (None, Some(b)) => (b, "--]"),
        _ => return ftl.to_string(),
    };
    let after = &ftl[comment_first_idx..];
    let end_pos = match after.find(end_marker) {
        Some(i) => i,
        None => return ftl.to_string(),
    };
    let comment_last_idx = comment_first_idx + end_pos + 2;
    let mut after_comment = comment_last_idx + 1;
    if after_comment < ftl.len() {
        let c = ftl.as_bytes()[after_comment] as char;
        if c == '\n' || c == '\r' {
            if c == '\r'
                && after_comment + 1 < ftl.len()
                && ftl.as_bytes()[after_comment + 1] == b'\n'
            {
                after_comment += 2;
            } else {
                after_comment += 1;
            }
        }
    }
    // Java：commentLastIdx 为 '>' 的含位下标，+1 即越过 '>'，再越过换行后即为剩余内容
    let mut out = String::with_capacity(ftl.len());
    out.push_str(&ftl[..comment_first_idx]);
    if after_comment <= ftl.len() {
        out.push_str(&ftl[after_comment..]);
    }
    out
}

fn collect_ftl(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_ftl(&p, out);
            } else if p.extension().is_some_and(|x| x == "ftl") {
                out.push(p);
            }
        }
    }
}

/// 应用用例 settings（Java TemplateTestCase.setSetting；Java 特有设置跳过并记录原因）
pub fn apply_settings(
    c: &mut Configuration,
    settings: &std::collections::BTreeMap<String, String>,
) -> Vec<String> {
    let mut skipped = Vec::new();
    for (k, v) in settings {
        match k.as_str() {
            "locale" => c.settings.locale = v.clone(),
            "incompatible_improvements" => {
                // 取清单中的具体版本（"min, 2.3.20" / "2.3.21, max" / "min, 2.3.21, max" →
                // 2.3.20 / 2.3.21 / 2.3.21）：仅影响 IcI 相关语义
                // （如 ?iso 对 java.sql.Time 偏移显示的 2.3.21 分界，AbstractISOBI :202-212）
                let v = v
                    .split(',')
                    .map(|t| t.trim())
                    .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()));
                if let Some(v) = v {
                    if let Ok(ver) = freemarker::template::Version::parse(v) {
                        c.settings.incompatible_improvements = ver;
                    }
                }
            }
            "url_escaping_charset" => c.settings.url_escaping_charset = v.clone(),
            "output_encoding" => c.settings.output_encoding = v.clone(),
            "classic_compatible" => {
                c.settings.classic_compatible = v == "Y" || v == "2";
            }
            "strict_syntax" => c.settings.strict_syntax = v == "Y" || v == "1",
            "time_zone" => {
                if let Ok(tz) = v.parse() {
                    c.settings.time_zone = tz;
                }
            }
            "auto_import" => {
                skipped.push("auto_import 设置（Configuration.addAutoImport）未实现".to_string())
            }
            "input_encoding" => {} // v1 字符串加载器无编码概念（P4）
            "clear_encoding_map" => {}
            "object_wrapper" => {
                // SimpleObjectWrapper 与我们的数据模型等价；其余 Java wrapper 跳过
                if !v.contains("SimpleObjectWrapper") {
                    skipped.push(format!("object_wrapper={v}（Java 特有 wrapper）"));
                }
            }
            "api_builtin_enabled" => {}
            "new_builtin_class_resolver" => skipped.push("?new 类解析（Java 特有）".to_string()),
            other => skipped.push(format!("未识别设置 {other}")),
        }
    }
    skipped
}

/// 渲染用例，返回输出（Java runTest：process(dataModel, out)）
pub fn render_case(c: &Configuration, name: &str, root: TModel) -> Result<String> {
    let t = c.get_template_localized(name, Some(&c.settings.locale))?;
    let mut out = Vec::new();
    t.process(root, &mut out)?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// 数字模型辅助
pub fn num(v: i64) -> TModel {
    TModel::from_number(TNumber::from_i64(v))
}

pub fn dec(s: &str) -> TModel {
    TModel::from_number(TNumber::Decimal(s.parse().unwrap()))
}

pub fn dbl(v: f64) -> TModel {
    TModel::from_number(TNumber::Double(v))
}

pub fn flt(v: f32) -> TModel {
    TModel::from_number(TNumber::Float(v))
}

/// 日期模型（UTC 时刻 → FixedOffset）
/// 测试辅助构造器（8 参数）；豁免 too_many_arguments
#[allow(clippy::too_many_arguments)]
pub fn date_model(
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
    TModel::from_date(DateValue {
        dt: DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
        kind,
        is_sql: false,
    })
}

/// java.sql.Date/Time 风格日期模型（对应 TemplateTestCase 的 java.sql 值：
/// SQL 值在 ISO/XS 格式中默认不显示时区偏移，见 value.rs DateValue.is_sql）
/// 测试辅助（8 参数）；豁免 too_many_arguments
#[allow(clippy::too_many_arguments)]
pub fn sql_date_model(
    y: i32,
    mo: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
    ms: u32,
    kind: DateType,
) -> TModel {
    let mut m = date_model(y, mo, d, h, mi, s, ms, kind);
    if let Ok(dv) = m.get_date() {
        m = TModel::from_date(DateValue {
            dt: dv.dt,
            kind: dv.kind,
            is_sql: true,
        });
    }
    m
}

/// 数据模型构造 —— 对应 Java TemplateTestCase.setUp（TemplateTestCase.java:132-440）：
/// 公共变量 + 按用例名的专用模型
pub fn build_data_model(simple_test_name: &str) -> TModel {
    let mut m: IndexMap<String, TModel> = IndexMap::new();
    // 公共变量（TemplateTestCase.java:184-193）
    m.insert(
        "assert".to_string(),
        TModel::from_directive(AssertDirective),
    );
    m.insert(
        "assertEquals".to_string(),
        TModel::from_directive(AssertEqualsDirective),
    );
    m.insert(
        "assertFails".to_string(),
        TModel::from_directive(AssertFailsDirective),
    );
    m.insert(
        "noOutput".to_string(),
        TModel::from_directive(NoOutputDirective),
    );
    m.insert(
        "testName".to_string(),
        TModel::from_scalar(simple_test_name.to_string()),
    );
    m.insert("iciIntValue".to_string(), num(2_003_034)); // Configuration(2.3.34).intValue()
    m.insert(
        "message".to_string(),
        TModel::from_scalar("Hello, world!".to_string()),
    );
    m.insert(
        "javaObjectInfo".to_string(),
        TModel::from_method(JavaObjectInfoMethod),
    );

    match simple_test_name {
        "boolean" => {
            // TemplateTestCase.java:264-276
            m.insert("boolean1".to_string(), TModel::from_boolean(false));
            m.insert("boolean2".to_string(), TModel::from_boolean(true));
            m.insert("boolean3".to_string(), TModel::from_boolean(true));
            m.insert("boolean4".to_string(), TModel::from_boolean(true));
            m.insert("boolean5".to_string(), TModel::from_boolean(false));
            m.insert(
                "list1".to_string(),
                TModel::from_sequence(vec![
                    TModel::from_scalar("false".to_string()),
                    TModel::from_scalar("0".to_string()),
                    TModel::from_boolean(false),
                    TModel::from_boolean(true),
                    TModel::from_boolean(true),
                    TModel::from_boolean(true),
                    TModel::from_boolean(false),
                ]),
            );
            m.insert("list2".to_string(), TModel::from_sequence(vec![]));
            m.insert(
                "hash1".to_string(),
                TModel::from_hash({
                    let mut h = IndexMap::new();
                    h.insert(
                        "temp".to_string(),
                        TModel::from_scalar("Hello, world.".to_string()),
                    );
                    h.insert("boolean".to_string(), TModel::from_boolean(false));
                    h
                }),
            );
            m.insert("hash2".to_string(), TModel::from_hash(IndexMap::new()));
        }
        "variables" | "iterators" | "if" | "comment" => {}
        "list" | "list2" | "list3" | "list-bis" | "listhash" => {
            m.insert("listables".to_string(), listables_model());
        }
        "number-format" => {
            // TemplateTestCase.java:305-311
            m.insert("int".to_string(), num(1));
            m.insert("double".to_string(), dbl(1.0));
            m.insert("double2".to_string(), dbl(1.0 + 1e-15));
            m.insert("double3".to_string(), dbl(1e-16));
            m.insert("double4".to_string(), dbl(-1e-16));
            m.insert("bigDecimal".to_string(), dec("1"));
            m.insert(
                "bigDecimal2".to_string(),
                TModel::from_number(TNumber::Decimal("1E-16".parse().unwrap())),
            );
        }
        "number-math-builtins" => {
            // TemplateTestCase.java:390-420
            m.insert("fNan".to_string(), flt(f32::NAN));
            m.insert("dNan".to_string(), dbl(f64::NAN));
            m.insert("fNinf".to_string(), flt(f32::NEG_INFINITY));
            m.insert("dPinf".to_string(), dbl(f64::INFINITY));
            m.insert("fn".to_string(), flt(-0.05));
            m.insert("dn".to_string(), dbl(-0.05));
            m.insert("ineg".to_string(), num(-5));
            m.insert("ln".to_string(), TModel::from_number(TNumber::Long(-5)));
            m.insert("sn".to_string(), num(-5));
            m.insert("bn".to_string(), num(-5));
            m.insert(
                "bin".to_string(),
                TModel::from_number(TNumber::BigInt(5.into())),
            );
            m.insert("bdn".to_string(), dec("-0.05"));
            m.insert("fp".to_string(), flt(0.05));
            m.insert("dp".to_string(), dbl(0.05));
            m.insert("ip".to_string(), num(5));
            m.insert("lp".to_string(), TModel::from_number(TNumber::Long(5)));
            m.insert("sp".to_string(), num(5));
            m.insert("bp".to_string(), num(5));
            m.insert(
                "bip".to_string(),
                TModel::from_number(TNumber::BigInt(5.into())),
            );
            m.insert("bdp".to_string(), dec("0.05"));
        }
        "boolean-formatting" => {
            m.insert("beansBoolean".to_string(), TModel::from_boolean(true));
            m.insert("booleanAndString".to_string(), bool_and_string());
            // BooleanVsStringMethods.java：expectsString/expectsBoolean/overloaded
            let mut bvsm = IndexMap::new();
            bvsm.insert(
                "expectsString".to_string(),
                TModel::from_method(BvsExpectsString),
            );
            bvsm.insert(
                "expectsBoolean".to_string(),
                TModel::from_method(BvsExpectsBoolean),
            );
            bvsm.insert("overloaded".to_string(), TModel::from_method(BvsOverloaded));
            m.insert(
                "booleanVsStringMethods".to_string(),
                TModel::from_hash(bvsm),
            );
        }
        "date-type-builtins" => {
            // TemplateTestCase.java:336-344：2003-04-05 06:07:08 UTC
            m.insert(
                "unknown".to_string(),
                date_model(2003, 4, 5, 6, 7, 8, 0, DateType::DateTime),
            );
            m.insert(
                "timeOnly".to_string(),
                date_model(2003, 4, 5, 6, 7, 8, 0, DateType::Time),
            );
            m.insert(
                "dateOnly".to_string(),
                date_model(2003, 4, 5, 6, 7, 8, 0, DateType::Date),
            );
            m.insert(
                "dateTime".to_string(),
                date_model(2003, 4, 5, 6, 7, 8, 0, DateType::DateTime),
            );
        }
        "sequence-builtins" => {
            // TemplateTestCase.java:368-389：abcSet 用 TreeSet（有序）；set 用 HashSet
            m.insert(
                "abcSet".to_string(),
                TModel::from_sequence(vec![
                    TModel::from_scalar("a".to_string()),
                    TModel::from_scalar("b".to_string()),
                    TModel::from_scalar("c".to_string()),
                ]),
            );
            m.insert(
                "abcSetNonSeq".to_string(),
                TModel::from_collection(vec![
                    TModel::from_scalar("a".to_string()),
                    TModel::from_scalar("b".to_string()),
                    TModel::from_scalar("c".to_string()),
                ]),
            );
            m.insert(
                "listWithNull".to_string(),
                TModel::from_sequence(vec![
                    TModel::from_scalar("a".to_string()),
                    TModel::nothing(),
                    TModel::from_scalar("c".to_string()),
                ]),
            );
            m.insert(
                "listWithNullsOnly".to_string(),
                TModel::from_sequence(vec![TModel::nothing()]),
            );
            m.insert(
                "abcCollection".to_string(),
                TModel::from_collection(vec![
                    TModel::from_scalar("a".to_string()),
                    TModel::from_scalar("b".to_string()),
                    TModel::from_scalar("c".to_string()),
                ]),
            );
            m.insert(
                "set".to_string(),
                TModel::from_sequence(vec![
                    TModel::from_scalar("a".to_string()),
                    TModel::from_scalar("b".to_string()),
                    TModel::from_scalar("c".to_string()),
                ]),
            );
        }
        "dateformat-iso-bi"
        | "dateformat-iso-bi-ici-2.3.21"
        | "dateformat-iso-like"
        | "dateformat-java" => {
            // TemplateTestCase.java:278-293：2002-11-15 14:54:13 GMT
            m.insert(
                "date".to_string(),
                date_model(2002, 11, 15, 14, 54, 13, 0, DateType::DateTime),
            );
            m.insert(
                "unknownDate".to_string(),
                // TemplateTestCase.java:279：SimpleDate(date, TemplateDateModel.UNKNOWN)
                TModel::from_date(DateValue {
                    dt: date_model(2002, 11, 15, 14, 54, 13, 0, DateType::DateTime)
                        .get_date()
                        .unwrap()
                        .dt,
                    kind: DateType::Unknown,
                    is_sql: false,
                }),
            );
            m.insert(
                "sqlDate".to_string(),
                sql_date_model(2010, 5, 15, 0, 0, 0, 0, DateType::Date),
            );
            m.insert(
                "sqlTime".to_string(),
                sql_date_model(1970, 1, 1, 20, 38, 5, 23, DateType::Time),
            );
            m.insert(
                "javaGMT02".to_string(),
                TModel::from_scalar("GMT+02".to_string()),
            );
            m.insert(
                "javaUTC".to_string(),
                TModel::from_scalar("UTC".to_string()),
            );
            m.insert(
                "adaptedToStringScalar".to_string(),
                TModel::from_scalar("GMT+02".to_string()),
            );
        }
        "var-layers" => {
            m.insert("x".to_string(), num(4));
            m.insert("z".to_string(), num(4));
        }
        "multimodels" => {
            m.insert(
                "test".to_string(),
                TModel::from_scalar("selftest".to_string()),
            );
            m.insert("self".to_string(), TModel::from_scalar("self".to_string()));
            m.insert("zero".to_string(), num(0));
        }
        "type-builtins" => {
            m.insert("testmethod".to_string(), TModel::from_method(TestMethod));
            m.insert("testnode".to_string(), TModel::nothing()); // v1 无节点模型
            m.insert(
                "testcollection".to_string(),
                TModel::from_collection(vec![]),
            );
            m.insert(
                "testcollectionEx".to_string(),
                TModel::from_collection(vec![]),
            );
        }
        "simplehash-char-key" => {
            // TemplateTestCase.java:313-330：String 键与 Character 键的哈希
            let mut mc = IndexMap::new();
            mc.insert("c".to_string(), TModel::from_scalar("string".to_string()));
            m.insert("mStringC".to_string(), TModel::from_hash(mc));
            let mut mcn = IndexMap::new();
            mcn.insert("c".to_string(), TModel::nothing());
            m.insert("mStringCNull".to_string(), TModel::from_hash(mcn));
            let mut mcc = IndexMap::new();
            mcc.insert("c".to_string(), TModel::from_scalar("char".to_string()));
            m.insert("mCharC".to_string(), TModel::from_hash(mcc));
            let mut mcn2 = IndexMap::new();
            mcn2.insert("c".to_string(), TModel::nothing());
            m.insert("mCharCNull".to_string(), TModel::from_hash(mcn2));
            let mut mm = IndexMap::new();
            mm.insert("c".to_string(), TModel::from_scalar("char".to_string()));
            mm.insert("s".to_string(), TModel::from_scalar("string".to_string()));
            mm.insert("s2".to_string(), TModel::from_scalar("string2".to_string()));
            mm.insert("s2n".to_string(), TModel::nothing());
            m.insert("mMixed".to_string(), TModel::from_hash(mm));
        }
        "classic-compatible" | "classic-compatible-mode2" => {
            // TemplateTestCase.java:444-446：beanTrue/beanFalse = beansWrapper.wrap(Boolean)；
            // Java 经典模式 2 下 BeanModel 布尔按 2.1 字符串行为 → "true"/"false"
            // （EvalUtil.coerceModelToTextualCommon :510-516）；用双角色模型（scalar+boolean）近似
            m.insert("beanTrue".to_string(), TModel::from_boolean(true));
            m.insert(
                "beanFalse".to_string(),
                TModel {
                    scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                        "false".to_string(),
                    ))),
                    boolean: Some(std::rc::Rc::new(freemarker::template::SimpleBoolean(false))),
                    type_name: "boolean",
                    kind: freemarker::template::ModelKind::Boolean,
                    ..TModel::nothing()
                },
            );
        }
        "number-to-date" => {
            m.insert(
                "bigInteger".to_string(),
                TModel::from_number(TNumber::BigInt("1305575275540".parse().unwrap())),
            );
            m.insert("bigDecimal".to_string(), dec("1305575275539.5"));
        }
        _ => {}
    }
    TModel::from_hash(m)
}

/// listables 模型（对应 Listables.java：list/linkedList/set/iterator/empty*）
fn listables_model() -> TModel {
    let mut h = IndexMap::new();
    let seq = TModel::from_sequence(vec![num(11), num(22), num(33)]);
    h.insert("list".to_string(), seq.clone());
    h.insert("linkedList".to_string(), seq.clone());
    h.insert("set".to_string(), seq.clone());
    h.insert(
        "iterator".to_string(),
        TModel::from_collection(vec![num(11), num(22), num(33)]),
    );
    h.insert(
        "getIterator".to_string(),
        TModel::from_method(IteratorMethod),
    );
    h.insert(
        "getEmptyIterator".to_string(),
        TModel::from_method(EmptyIteratorMethod),
    );
    h.insert("emptyList".to_string(), TModel::from_sequence(vec![]));
    h.insert("emptyLinkedList".to_string(), TModel::from_sequence(vec![]));
    h.insert("emptySet".to_string(), TModel::from_sequence(vec![]));
    h.insert("hashEx2s".to_string(), TModel::from_sequence(vec![]));
    h.insert("emptyHashes".to_string(), TModel::from_sequence(vec![]));
    h.insert("hashNonEx2".to_string(), TModel::from_hash(IndexMap::new()));
    TModel::from_hash(h)
}

struct IteratorMethod;
impl freemarker::template::TemplateMethodModelEx for IteratorMethod {
    fn exec(&self, _args: Vec<TModel>) -> Result<TModel> {
        Ok(TModel::from_collection(vec![num(11), num(22), num(33)]))
    }
}

struct EmptyIteratorMethod;
impl freemarker::template::TemplateMethodModelEx for EmptyIteratorMethod {
    fn exec(&self, _args: Vec<TModel>) -> Result<TModel> {
        Ok(TModel::from_collection(vec![]))
    }
}

/// 布尔+字符串双角色模型（Java BooleanAndStringTemplateModel：
/// getAsString = "theStringValue"、getAsBoolean = true）
fn bool_and_string() -> TModel {
    TModel {
        scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
            "theStringValue".to_string(),
        ))),
        boolean: Some(std::rc::Rc::new(freemarker::template::SimpleBoolean(true))),
        type_name: "boolean",
        kind: freemarker::template::ModelKind::Boolean,
        ..TModel::nothing()
    }
}

/// JavaObjectInfo（v1：info(.locale_object) 等方法不支持 → 返回占位字符串）
struct JavaObjectInfoMethod;
impl freemarker::template::TemplateMethodModelEx for JavaObjectInfoMethod {
    fn exec(&self, _args: Vec<TModel>) -> Result<TModel> {
        Ok(TModel::from_scalar(String::new()))
    }
}

/// BooleanVsStringMethods.expectsString（参数须为字符串；布尔参数报错——Java
/// BeansWrapper 解包失败消息 "Can't convert the ..."）
struct BvsExpectsString;
impl freemarker::template::TemplateMethodModelEx for BvsExpectsString {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let m = args
            .first()
            .ok_or_else(|| TemplateError::misc("expectsString requires an argument"))?;
        if let Some(s) = &m.scalar {
            return Ok(TModel::from_scalar(s.as_string()?));
        }
        Err(TemplateError::misc(format!(
            "Can't convert the value to a string: it's a {}",
            m.type_name
        )))
    }
}

/// BooleanVsStringMethods.expectsBoolean
struct BvsExpectsBoolean;
impl freemarker::template::TemplateMethodModelEx for BvsExpectsBoolean {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let b = args
            .first()
            .ok_or_else(|| TemplateError::misc("expectsBoolean requires an argument"))?
            .eval_boolean()?;
        Ok(TModel::from_boolean(b))
    }
}

/// BooleanVsStringMethods.overloaded：字符串参数 → "String x"，布尔 → "boolean x"
struct BvsOverloaded;
impl freemarker::template::TemplateMethodModelEx for BvsOverloaded {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let m = args
            .first()
            .ok_or_else(|| TemplateError::misc("overloaded requires an argument"))?;
        if let Ok(s) = m.get_scalar() {
            return Ok(TModel::from_scalar(format!("String {s}")));
        }
        if let Ok(b) = m.eval_boolean() {
            return Ok(TModel::from_scalar(format!("boolean {b}")));
        }
        Err(TemplateError::misc("overloaded: unsupported argument type"))
    }
}

struct TestMethod;
impl freemarker::template::TemplateMethodModelEx for TestMethod {
    fn exec(&self, _args: Vec<TModel>) -> Result<TModel> {
        Ok(TModel::from_scalar("x".to_string()))
    }
}

/// `<@assert test=.../>` —— 对应 AssertDirective（参数 test 为布尔；假则报错）
pub struct AssertDirective;
impl TemplateDirectiveModel for AssertDirective {
    fn execute(
        &self,
        _env: &mut freemarker::core::Environment,
        params: &HashMap<String, TModel>,
        _loop_vars: &mut [TModel],
        _body: Option<&dyn TemplateDirectiveBody>,
    ) -> Result<()> {
        let test = params
            .get("test")
            .ok_or_else(|| TemplateError::misc("Missing required parameter \"test\""))?;
        let b = test.eval_boolean()?;
        if !b {
            return Err(TemplateError::misc("Assertion failed"));
        }
        Ok(())
    }
}

/// `<@assertEquals actual=... expected=.../>` —— 对应 AssertEqualsDirective：
/// 宽松相等（Java env.applyEqualsOperatorLenient：数字按值、字符串按内容）
pub struct AssertEqualsDirective;
impl TemplateDirectiveModel for AssertEqualsDirective {
    fn execute(
        &self,
        env: &mut freemarker::core::Environment,
        params: &HashMap<String, TModel>,
        _loop_vars: &mut [TModel],
        _body: Option<&dyn TemplateDirectiveBody>,
    ) -> Result<()> {
        let actual = params
            .get("actual")
            .ok_or_else(|| TemplateError::misc("Missing required parameter \"actual\""))?;
        let expected = params
            .get("expected")
            .ok_or_else(|| TemplateError::misc("Missing required parameter \"expected\""))?;
        let eq = compare_models(env, actual, expected, CmpOp::Eq)?;
        if !eq {
            return Err(TemplateError::misc(format!(
                "Assertion failed:\nExpected: {}\nActual: {}",
                describe(expected),
                describe(actual)
            )));
        }
        Ok(())
    }
}

/// 模型描述（错误消息用）
fn describe(m: &TModel) -> String {
    if m.is_scalar() {
        return m.get_scalar().unwrap_or_default();
    }
    if let Ok(n) = m.get_number() {
        return n.to_plain_string();
    }
    if let Ok(b) = m.get_boolean() {
        return b.to_string();
    }
    m.type_name.to_string()
}

/// `<@assertFails [message=...]>body</@>` —— 对应 AssertFailsDirective：
/// body 必须报错；message 参数检查错误消息包含指定文本
pub struct AssertFailsDirective;
impl TemplateDirectiveModel for AssertFailsDirective {
    fn execute(
        &self,
        env: &mut freemarker::core::Environment,
        params: &HashMap<String, TModel>,
        _loop_vars: &mut [TModel],
        body: Option<&dyn TemplateDirectiveBody>,
    ) -> Result<()> {
        let message = params.get("message").and_then(|m| m.get_scalar().ok());
        let message_regexp = params
            .get("messageRegexp")
            .and_then(|m| m.get_scalar().ok());
        let exception = params.get("exception").and_then(|m| m.get_scalar().ok());
        let body =
            body.ok_or_else(|| TemplateError::misc("assertFails requires nested content"))?;
        let err = match body.render(env) {
            Ok(()) => Err(TemplateError::misc(
                "Assertion failed: the nested content was expected to fail, but it didn't",
            )),
            Err(e) => {
                let msg = e.to_string();
                if let Some(expected) = message {
                    // Java AssertFailsDirective：toLowerCase().indexOf —— 大小写不敏感
                    let ml = msg.to_lowercase();
                    if !ml.contains(&expected.to_lowercase()) {
                        return Err(TemplateError::misc(format!(
                            "Failure is not like expected: expected message containing {expected:?}, got: {msg}"
                        )));
                    }
                }
                if let Some(re) = message_regexp {
                    // Java AssertFailsDirective.java:59-61：Pattern.CASE_INSENSITIVE
                    if !regex::Regex::new(&format!("(?i){re}"))
                        .map(|r| r.is_match(&msg))
                        .unwrap_or(false)
                    {
                        return Err(TemplateError::misc(format!(
                            "Failure is not like expected: message didn't match regexp {re:?}: {msg}"
                        )));
                    }
                }
                if let Some(exp) = exception {
                    // Rust 无 Java 异常类层级：UnexpectedTypeException 等以错误消息特征匹配
                    // （Java 检查 e.getClass().getName()；本引擎 TypeMismatch 消息含 "is required"）
                    let matched = if exp == "UnexpectedTypeException" {
                        msg.contains("is required") || msg.contains("is not applicable")
                    } else {
                        msg.contains(&exp)
                    };
                    if !matched {
                        return Err(TemplateError::misc(format!(
                            "Failure is not like expected: exception type {exp:?} not found in: {msg}"
                        )));
                    }
                }
                Ok(())
            }
        };
        err
    }
}

/// `<@noOutput>...</@>` —— 对应 NoOutputDirective：渲染 body 但丢弃输出
pub struct NoOutputDirective;
impl TemplateDirectiveModel for NoOutputDirective {
    fn execute(
        &self,
        env: &mut freemarker::core::Environment,
        _params: &HashMap<String, TModel>,
        _loop_vars: &mut [TModel],
        body: Option<&dyn TemplateDirectiveBody>,
    ) -> Result<()> {
        if let Some(b) = body {
            let captured = env.capture(|env| b.render(env))?;
            let _ = captured.1;
        }
        Ok(())
    }
}
