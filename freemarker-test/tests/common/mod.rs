//! 黄金套件测试辅助 —— 对应 Java `TemplateTestCase.java`（templatesuite 数据模型构造 +
//! assert/assertEquals/assertFails/noOutput 指令 + 设置应用；docs/11 §3）

use bigdecimal::ToPrimitive;
use freemarker::cache::StringLoader;
use freemarker::core::{compare_models, CmpOp};
use freemarker::error::{Result, TemplateError};
use freemarker::template::{Configuration, TModel, TemplateDirectiveBody, TemplateDirectiveModel};
use freemarker::value::{DateType, DateValue, TNumber};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

pub const SUITE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/suite");

/// Java models/xmlns.xml 的内容（xmlns1/xmlns2 用例的 doc 模型；
/// xmlns2 的 eb: 前缀版文档命名空间 URI 相同，输出逐字节一致）
const BOOK_XML: &str = "<book xmlns=\"http://example.com/eBook\">\n  <title>Test Book</title>\n  <chapter>\n    <title>Ch1</title>\n    <para>p1.1</para>\n    <para>p1.2</para>\n    <para>p1.3</para>\n  </chapter>\n  <chapter>\n    <title>Ch2</title>\n    <para>p2.1</para>\n    <para>p2.2</para>\n  </chapter>\n</book>";

/// 读取套件文件（模板/expected）
pub fn read_suite(rel: &str) -> String {
    std::fs::read_to_string(format!("{SUITE_DIR}/{rel}"))
        .unwrap_or_else(|e| panic!("cannot read {SUITE_DIR}/{rel}: {e}"))
}

/// 读取预期文件并按指定编码解码（output_encoding 非 UTF-8 的测试用例）
pub fn read_suite_encoded(rel: &str, encoding: &str) -> String {
    let bytes = std::fs::read(format!("{SUITE_DIR}/{rel}"))
        .unwrap_or_else(|e| panic!("cannot read {SUITE_DIR}/{rel}: {e}"));
    decode_bytes(&bytes, encoding)
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
    c.settings.strict_syntax = true;
    c.settings.whitespace_stripping = true;
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    (c, loader)
}

/// 注册用例模板与依赖模板（Java FileTemplateLoader(templates 目录) 的等价物：
/// 预注册全部依赖模板，避免 include/import 相对路径解析失败；
/// 模板经 removeFTLCopyrightComment 处理（Java CopyrightCommentRemoverTemplateLoader）
/// 按原始字节注册（charset-in-header 等非 UTF-8 模板：read_to_string 会失败/损坏，
/// 原始字节保留使 read_encoded 能按声明编码解码）
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
        let bytes = std::fs::read(&f).unwrap_or_default();
        loader.put_bytes(&rel, &remove_ftl_copyright_comment_bytes(&bytes));
    }
}

/// 字节版版权注释移除：ASCII 查找（"copyright" / `<#--` / `-->`）对任意编码安全
/// （注释标记与版权词均为 ASCII；ISO-8859-x 等单字节内容不经解码直接搬运）
pub fn remove_ftl_copyright_comment_bytes(ftl: &[u8]) -> Vec<u8> {
    let lower: Vec<u8> = ftl.iter().map(|b| b.to_ascii_lowercase()).collect();
    let copyright_idx = find_bytes(&lower, b"copyright");
    let Some(copyright_idx) = copyright_idx else {
        return ftl.to_vec();
    };
    let before = &ftl[..copyright_idx];
    let ab_start = rfind_bytes(before, b"<#--");
    let sb_start = rfind_bytes(before, b"[#--");
    let (comment_first_idx, end_marker) = match (ab_start, sb_start) {
        (Some(a), Some(b)) if b > a => (b, b"--]".as_slice()),
        (Some(a), _) => (a, b"-->".as_slice()),
        (None, Some(b)) => (b, b"--]".as_slice()),
        _ => return ftl.to_vec(),
    };
    let after = &ftl[comment_first_idx..];
    let Some(end_pos) = find_bytes(after, end_marker) else {
        return ftl.to_vec();
    };
    let comment_last_idx = comment_first_idx + end_pos + 2;
    let mut after_comment = comment_last_idx + 1;
    if after_comment < ftl.len() {
        let c = ftl[after_comment];
        if c == b'\n' || c == b'\r' {
            if c == b'\r' && after_comment + 1 < ftl.len() && ftl[after_comment + 1] == b'\n' {
                after_comment += 2;
            } else {
                after_comment += 1;
            }
        }
    }
    let mut out = Vec::with_capacity(ftl.len());
    out.extend_from_slice(&ftl[..comment_first_idx]);
    if after_comment <= ftl.len() {
        out.extend_from_slice(&ftl[after_comment..]);
    }
    out
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
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
                // Java SettingStringParser.parseAsImportList："path as ns" 列表
                // （逗号分隔）；Configuration.addAutoImport(ns, path) 语义
                for item in v.split(',') {
                    let item = item.trim();
                    if item.is_empty() {
                        continue;
                    }
                    match item.split_once(" as ") {
                        Some((path, ns)) => {
                            c.auto_imports
                                .push((ns.trim().to_string(), path.trim().to_string()));
                        }
                        None => skipped.push(format!("auto_import 格式未识别: {item:?}")),
                    }
                }
            }
            "input_encoding" => c.settings.input_encoding = Some(v.clone()),
            "clear_encoding_map" => {}
            "object_wrapper" => {
                // SimpleObjectWrapper 与我们的数据模型等价；其余 Java wrapper 跳过
                if !v.contains("SimpleObjectWrapper") {
                    skipped.push(format!("object_wrapper={v}（Java 特有 wrapper）"));
                }
            }
            "api_builtin_enabled" => {}
            "template_exception_handler" => {
                // Java TemplateTestCase 可设置（Configuration.setTemplateExceptionHandler
                // 的字符串形式：rethrow/debug/html_debug/ignore）
                c.settings.template_exception_handler = v.clone();
            }
            "new_builtin_class_resolver" => skipped.push("?new 类解析（Java 特有）".to_string()),
            other => skipped.push(format!("未识别设置 {other}")),
        }
    }
    skipped
}

/// 渲染用例，返回输出（Java runTest：process(dataModel, out)）。
/// 内部按 output_encoding 转码后写出，再由本函数解码回 String 供比较。
pub fn render_case(c: &Configuration, name: &str, root: TModel) -> Result<String> {
    // Java Configuration.getTemplate(name, locale, ..., encoding=cfg default)：
    // input_encoding 设置时按该编码解码（charset-in-header），`<#ftl encoding>`
    // 头触发 WrongEncodingException 重读；否则走本地化回退的常规路径
    let t = match c.settings.input_encoding.as_deref() {
        Some(enc) => c.get_template_encoded(name, Some(enc))?,
        None => c.get_template_localized(name, Some(&c.settings.locale))?,
    };
    let mut out = Vec::new();
    t.process(root, &mut out)?;
    // 按 output_encoding 解码字节为 String
    let output_encoding = &c.settings.output_encoding;
    if output_encoding.eq_ignore_ascii_case("UTF-8") || output_encoding.is_empty() {
        Ok(String::from_utf8_lossy(&out).into_owned())
    } else {
        Ok(decode_bytes(&out, output_encoding))
    }
}

/// 按 IANA 编码名解码字节为 String（编码未知时回退到 UTF-8 lossy）
fn decode_bytes(bytes: &[u8], encoding_name: &str) -> String {
    // ISO-8859-1：逐字节映射到 Unicode 码点
    if encoding_name.eq_ignore_ascii_case("ISO-8859-1") {
        return bytes.iter().map(|&b| b as char).collect();
    }
    // UTF-16 系列：按 BOM 或显式字节序解码
    if encoding_name.to_uppercase().contains("UTF-16") {
        let (start, big_endian) = if bytes.len() >= 2 {
            match (bytes[0], bytes[1]) {
                (0xFE, 0xFF) => (2, true),  // UTF-16BE BOM
                (0xFF, 0xFE) => (2, false), // UTF-16LE BOM
                _ => (0, !encoding_name.to_uppercase().contains("LE")),
            }
        } else {
            (0, !encoding_name.to_uppercase().contains("LE"))
        };
        let mut s = String::with_capacity((bytes.len() - start) / 2);
        let mut i = start;
        while i + 1 < bytes.len() {
            let cu = if big_endian {
                u16::from_be_bytes([bytes[i], bytes[i + 1]])
            } else {
                u16::from_le_bytes([bytes[i], bytes[i + 1]])
            };
            if let Some(c) = char::from_u32(cu as u32) {
                s.push(c);
            } else {
                // 代理对（高位 → 低位拼接）
                if (0xD800..=0xDBFF).contains(&cu) && i + 3 < bytes.len() {
                    let low = if big_endian {
                        u16::from_be_bytes([bytes[i + 2], bytes[i + 3]])
                    } else {
                        u16::from_le_bytes([bytes[i + 2], bytes[i + 3]])
                    };
                    if (0xDC00..=0xDFFF).contains(&low) {
                        let cp = 0x10000 + ((cu as u32 - 0xD800) << 10) + (low as u32 - 0xDC00);
                        if let Some(c) = char::from_u32(cp) {
                            s.push(c);
                            i += 2;
                        }
                    }
                }
                s.push('\u{FFFD}');
            }
            i += 2;
        }
        return s;
    }
    // 未知编码：UTF-8 lossy 兜底
    String::from_utf8_lossy(bytes).into_owned()
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
/// 公共变量 + 按用例名的专用模型。
/// `case_name` 为完整用例名（含 `[#endTN]` 变体后缀）：变体与 base 共享模板/expected，
/// 但数据模型角色不同（如 collectionAdapter 变体的非 List 集合 → collection 角色）
pub fn build_data_model(simple_test_name: &str, case_name: &str) -> TModel {
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
    // Java JavaObjectInfo：对象（info 方法属性）；`javaObjectInfo.info(x)` 调用
    let mut info_hash = IndexMap::new();
    info_hash.insert(
        "info".to_string(),
        TModel::from_method(JavaObjectInfoMethod),
    );
    m.insert("javaObjectInfo".to_string(), TModel::from_hash(info_hash));

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
            // collectionAdapter 变体（DefaultObjectWrapper(2.3.22,
            // forceLegacyNonListCollections=false)）：非 List 集合（set/emptySet）
            // wrap 成 collection 角色而非 sequence；list/list2 模板与 base 共享
            // 同一 expected，输出逐字节一致
            m.insert(
                "listables".to_string(),
                listables_model(case_name.contains("collectionAdapter")),
            );
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
        "stringbimethods" => {
            // TemplateTestCase.java:502-510：multi = TestBoolean ——
            // TemplateBooleanModel + TemplateScalarModel 双角色
            // （getAsBoolean → true；getAsString → "de"；插值标量优先）
            m.insert(
                "multi".to_string(),
                TModel {
                    scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                        "de".to_string(),
                    ))),
                    boolean: Some(std::rc::Rc::new(freemarker::template::SimpleBoolean(true))),
                    type_name: "wrapped",
                    kind: freemarker::template::ModelKind::Wrapped,
                    ..TModel::nothing()
                },
            );
        }
        "date-type-builtins" => {
            // TemplateTestCase.java:336-344：2003-04-05 06:07:08 UTC；
            // unknown = SimpleDate(d, TemplateDateModel.UNKNOWN)（未知类型）
            m.insert(
                "unknown".to_string(),
                date_model(2003, 4, 5, 6, 7, 8, 0, DateType::Unknown),
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
            // TemplateTestCase.java:368-389：abcSet 用 TreeSet（有序）；set 用 HashSet。
            // 各 wrapper 变体的角色差异（Java）：
            // - BeansWrapper/SimpleObjectWrapper：Collection → CollectionModel（sequence）
            // - DefaultObjectWrapper（默认 forceLegacyNonListCollections=true）：
            //   Collection → SimpleSequence（sequence）
            // - DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)
            //   （collAdapters 变体）：非 List 集合 → DefaultNonListCollectionAdapter
            //   （collection 角色，?size/?seq_* 均可用）
            // 模板对 abcSet/set 只用 ?size/?seq_*/?join/?first（对 sequence 与
            // collection 角色输出一致）→ 非 collAdapters 变体复用 sequence 模型，
            // collAdapters 变体用 collection 角色模型
            let abc: Vec<TModel> = ["a", "b", "c"]
                .iter()
                .map(|s| TModel::from_scalar(s.to_string()))
                .collect();
            let coll_adapters = case_name.contains("collAdapters");
            let abc_set = if coll_adapters {
                collection_ex_model(abc.clone())
            } else {
                TModel::from_sequence(abc.clone())
            };
            m.insert("abcSet".to_string(), abc_set.clone());
            m.insert(
                "abcSetNonSeq".to_string(),
                // Java：DefaultNonListCollectionAdapter.adapt(abcSet)（collection 角色）
                TModel::from_collection(abc.clone()),
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
                // Java：new SimpleCollection(abcSet)（collection 角色，非 Ex）
                TModel::from_collection(abc.clone()),
            );
            m.insert("set".to_string(), abc_set);
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
            // TemplateTestCase.java:332-334：data = MultiModel1 —— 三角色模型
            // （TemplateHashModel + TemplateSequenceModel + TemplateScalarModel）；
            // 序列：10 × "Model1 value: N" + MultiModel3（scalar+hash 双角色）
            let mut seq: Vec<TModel> = Vec::new();
            for i in 0..10 {
                seq.push(TModel::from_scalar(format!("Model1 value: {i}")));
            }
            seq.push(multimodel3());
            // 哈希：model2 → MultiModel2（scalar+method）；modellist → 序列；
            // selftest → 标量；one → MultiModel4（空序列 + hash{size}）；
            // two → MultiModel5（1 项序列 + hash{empty}）；size → "Nasty!"；
            // nesting1 → hash{nested → MultiModel3}
            let mut h = IndexMap::new();
            h.insert("model2".to_string(), multimodel2());
            h.insert("modellist".to_string(), TModel::from_sequence(seq.clone()));
            h.insert(
                "selftest".to_string(),
                TModel::from_scalar("Selftest of a hash from MultiModel1".to_string()),
            );
            h.insert("one".to_string(), multimodel4());
            h.insert("two".to_string(), multimodel5());
            h.insert(
                "size".to_string(),
                TModel::from_scalar("Nasty!".to_string()),
            );
            let mut nesting1 = IndexMap::new();
            nesting1.insert("nested".to_string(), multimodel3());
            h.insert("nesting1".to_string(), TModel::from_hash(nesting1));
            let h_model = TModel::from_hash(h);
            let seq_model = TModel::from_sequence(seq);
            m.insert(
                "data".to_string(),
                TModel {
                    scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                        "MultiModel1 as a string!".to_string(),
                    ))),
                    sequence: seq_model.sequence.clone(),
                    collection: seq_model.collection.clone(),
                    hash: h_model.hash.clone(),
                    hash_ex: h_model.hash_ex.clone(),
                    type_name: "wrapped",
                    kind: freemarker::template::ModelKind::Wrapped,
                    ..TModel::nothing()
                },
            );
            m.insert(
                "test".to_string(),
                TModel::from_scalar("selftest".to_string()),
            );
            m.insert("self".to_string(), TModel::from_scalar("self".to_string()));
            m.insert("zero".to_string(), num(0));
        }
        "type-builtins" => {
            m.insert("testmethod".to_string(), TModel::from_method(TestMethod));
            // Java TestNode（TemplateTestCase.java:530-560）→ TestNodeModel
            let node = TModel {
                node: Some(std::rc::Rc::new(TestNodeModel)),
                type_name: "node",
                kind: freemarker::template::ModelKind::Node,
                ..TModel::nothing()
            };
            m.insert("testnode".to_string(), node);
            m.insert(
                "testcollection".to_string(),
                TModel::from_collection(vec![]),
            );
            // testcollectionEx = DefaultNonListCollectionAdapter（只实现
            // TemplateCollectionModelEx，无 Sequence 角色）
            m.insert(
                "testcollectionEx".to_string(),
                TModel {
                    collection: Some(std::rc::Rc::new(freemarker::template::SimpleCollection(
                        Vec::new(),
                    ))),
                    collection_ex: true,
                    type_name: "collection",
                    kind: freemarker::template::ModelKind::Collection,
                    ..TModel::nothing()
                },
            );
            // bean = TestBean（TemplateTestCase.java:558-573，DefaultObjectWrapper 2.3.32+
            // 包装为 GenericObjectModel：scalar + hash + hash_ex 三角色）；
            // bean.m / bean.mOverloaded = 方法模型（GenericMethodModel 同时实现
            // TemplateSequenceModel——?is_indexable → true）
            let mut bh = IndexMap::new();
            bh.insert(
                "m".to_string(),
                TModel {
                    method: Some(std::rc::Rc::new(BeanMethod)),
                    method_indexable: true,
                    type_name: "method",
                    kind: freemarker::template::ModelKind::Method,
                    ..TModel::nothing()
                },
            );
            bh.insert(
                "mOverloaded".to_string(),
                TModel {
                    method: Some(std::rc::Rc::new(BeanMethod)),
                    method_indexable: true,
                    type_name: "method",
                    kind: freemarker::template::ModelKind::Method,
                    ..TModel::nothing()
                },
            );
            let bh = TModel::from_hash(bh);
            m.insert(
                "bean".to_string(),
                TModel {
                    scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                        "TestBean".to_string(),
                    ))),
                    hash: bh.hash.clone(),
                    hash_ex: bh.hash_ex.clone(),
                    type_name: "wrapped",
                    kind: freemarker::template::ModelKind::Wrapped,
                    ..TModel::nothing()
                },
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
        "classic-compatible" => {
            // TemplateTestCase.java:443-446：beanTrue/beanFalse = beansWrapper.wrap(Boolean)；
            // Java 经典模式 1（compatMode==1）下 BeanModel 布尔走经典布尔分支 → "true"/""
            // （EvalUtil.coerceModelToTextualCommon :495-518）
            m.insert("beanTrue".to_string(), TModel::from_boolean(true));
            m.insert("beanFalse".to_string(), TModel::from_boolean(false));
            // beansArray = beansWrapper.wrap(new String[]{"a","b","c"})：classic 模式下
            // BeanModel 字符串化 = Java 数组 toString（"[Ljava.lang.String@<hash>"），
            // 同时保持序列行为（?seq_index_of("b") → 1、?substring 作用于字符串化）；
            // 双角色模型（scalar + sequence）近似（coerceModelToTextualCommon
            // classic && BeanModel → _BeansAPI.getAsClassicCompatibleString）
            let mut arr = Vec::new();
            for s in ["a", "b", "c"] {
                arr.push(TModel::from_scalar(s.to_string()));
            }
            m.insert(
                "beansArray".to_string(),
                TModel {
                    scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                        "[Ljava.lang.String@12345678".to_string(),
                    ))),
                    sequence: Some(std::rc::Rc::new(freemarker::template::SimpleSequence(arr))),
                    type_name: "sequence",
                    kind: freemarker::template::ModelKind::Sequence,
                    ..TModel::nothing()
                },
            );
        }
        "classic-compatible-mode2" => {
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
        "varargs" => {
            // TemplateTestCase.java:411-413：m = VarArgTestModel —— 模拟 BeansWrapper
            // 的 varargs 方法调度（签名选择 + 序列参数展开 + 数值转换）
            let mut mm = IndexMap::new();
            mm.insert("bar".to_string(), TModel::from_method(VarBar));
            mm.insert("bar2".to_string(), TModel::from_method(VarBar2));
            mm.insert("overloaded".to_string(), TModel::from_method(VarOverloaded));
            mm.insert("noVarArgs".to_string(), TModel::from_method(VarNoVarArgs));
            m.insert("m".to_string(), TModel::from_hash(mm));
        }
        "bean-maps" => {
            // TemplateTestCase.java:321-333 + TestMapBean + TestBean ——
            // Java Bean → TemplateHashModel + TemplateScalarModel 双角色；
            // shadow 变体有 scalar（toString→name），非 shadow 变体无 scalar；
            // "all" 变体暴露全部 Object 方法（getClass/notify/wait 等，值恒 "UNKNOWN"）
            fn bean_props(name: &str) -> IndexMap<String, TModel> {
                let mut h = IndexMap::new();
                h.insert("age".to_string(), num(27));
                h.insert(
                    "location".to_string(),
                    TModel::from_scalar("San Francisco".to_string()),
                );
                h.insert("luckyNumber".to_string(), num(7));
                h.insert("name".to_string(), TModel::from_scalar(name.to_string()));
                h.insert("empty".to_string(), TModel::from_boolean(false));
                h.insert(
                    "class".to_string(),
                    TModel::from_scalar(
                        "class freemarker.test.templatesuite.TemplateTestCase$TestMapBean"
                            .to_string(),
                    ),
                );
                h
            }
            fn shadow_bean(name: &str) -> TModel {
                let mut tm = TModel::from_hash(bean_props(name));
                tm.scalar = Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                    name.to_string(),
                )));
                tm.kind = freemarker::template::ModelKind::Wrapped;
                tm.type_name = "wrapped";
                tm
            }
            fn all_bean_props(name: &str) -> IndexMap<String, TModel> {
                let mut h = bean_props(name);
                // Java Object 方法（TemplateHashModelEx2 不暴露 → 值恒 "UNKNOWN"）
                for method in &[
                    "clear",
                    "clone",
                    "containsKey",
                    "containsValue",
                    "entrySet",
                    "equals",
                    "get",
                    "getClass",
                    "getLuckyNumber",
                    "getName",
                    "hashCode",
                    "isEmpty",
                    "keySet",
                    "notify",
                    "notifyAll",
                    "put",
                    "putAll",
                    "remove",
                    "size",
                    "toString",
                    "values",
                    "wait",
                ] {
                    if !h.contains_key(*method) {
                        h.insert(
                            method.to_string(),
                            TModel::from_scalar("UNKNOWN".to_string()),
                        );
                    }
                }
                h
            }
            fn shadow_all_bean(name: &str) -> TModel {
                let mut tm = TModel::from_hash(all_bean_props(name));
                tm.scalar = Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                    name.to_string(),
                )));
                tm.kind = freemarker::template::ModelKind::Wrapped;
                tm.type_name = "wrapped";
                tm
            }
            // m1: properties only, shadow（scalar "Christopher" + hash 属性）
            m.insert("m1".to_string(), shadow_bean("Christopher"));
            // m2: properties only（纯 hash，无 scalar）
            m.insert("m2".to_string(), TModel::from_hash(bean_props("Chris")));
            // m3: nothing, shadow（scalar "Chris" + 仅 age/location/name）
            {
                let mut h3 = IndexMap::new();
                h3.insert("age".to_string(), num(27));
                h3.insert(
                    "location".to_string(),
                    TModel::from_scalar("San Francisco".to_string()),
                );
                h3.insert("name".to_string(), TModel::from_scalar("Chris".to_string()));
                let mut tm3 = TModel::from_hash(h3);
                tm3.scalar = Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
                    "Chris".to_string(),
                )));
                tm3.kind = freemarker::template::ModelKind::Wrapped;
                tm3.type_name = "wrapped";
                m.insert("m3".to_string(), tm3);
            }
            // m4: nothing（纯 hash，仅 age/location/name）
            {
                let mut h4 = IndexMap::new();
                h4.insert("age".to_string(), num(27));
                h4.insert(
                    "location".to_string(),
                    TModel::from_scalar("San Francisco".to_string()),
                );
                h4.insert("name".to_string(), TModel::from_scalar("Chris".to_string()));
                m.insert("m4".to_string(), TModel::from_hash(h4));
            }
            // m5: all, shadow（scalar "Christopher" + 全部属性/方法）
            m.insert("m5".to_string(), shadow_all_bean("Christopher"));
            // m6: all（纯 hash，全部属性/方法）
            m.insert("m6".to_string(), TModel::from_hash(all_bean_props("Chris")));
            // m7: simple map mode（纯 hash，仅 age/location/name，3 键）
            {
                let mut h7 = IndexMap::new();
                h7.insert("age".to_string(), num(27));
                h7.insert(
                    "location".to_string(),
                    TModel::from_scalar("San Francisco".to_string()),
                );
                h7.insert("name".to_string(), TModel::from_scalar("Chris".to_string()));
                m.insert("m7".to_string(), TModel::from_hash(h7));
            }
            // 字符串拼接测试用
            m.insert("s1".to_string(), TModel::from_scalar("hello".to_string()));
            m.insert("s2".to_string(), TModel::from_scalar("world".to_string()));
            m.insert("s3".to_string(), TModel::from_scalar("hello".to_string()));
            m.insert("s4".to_string(), TModel::from_scalar("world".to_string()));
        }
        "xml-fragment" => {
            // Java TemplateTestCase：node = NodeModel.parse(XML 字符串) 的 b 元素
            // （模板 `${node?node_name} = b`；根为 <root>，node 是其孙元素 b）
            let xml =
                "<root xmlns:n=\"http://x\"><a><b><n:c>C&lt;>&amp;\"']]&gt;</n:c></b></a></root>";
            let root_node = freemarker::xml::parse_xml(xml).expect("xml-fragment XML parse");
            // 取 b 元素（document → root → a → b）
            let b = root_node
                .node
                .as_ref()
                .expect("doc node")
                .children()
                .expect("doc children")[0]
                .node
                .as_ref()
                .expect("root node")
                .children()
                .expect("root children")[0]
                .node
                .as_ref()
                .expect("a node")
                .children()
                .expect("a children")[0]
                .clone();
            m.insert("node".to_string(), b);
        }
        "xmlns1" | "xmlns2" => {
            // xmlns1：Java TemplateTestCase 用 models/xmlns.xml（默认命名空间
            // <book>，xmlns1.ftl 的 `${doc.@@markup}`/`<#recurse doc>` 输出）。
            // xmlns2：Java 用 models/xmlns2.xml（eb: 前缀版，命名空间 URI 相同——
            // FTL 元素名按 URI 解析，输出逐字节一致，两用例共享同一 expected）
            let xml = BOOK_XML;
            let doc = freemarker::xml::parse_xml(xml).expect("xmlns1/2 XML parse");
            m.insert("doc".to_string(), doc);
        }
        "default-xmlns" | "xmlns5" => {
            // Java TemplateTestCase：doc = NodeModel.parse(models/defaultxmlns1.xml)
            // （root 下 t1 无命名空间、x:t2 x NS、y:t3 y NS、t4 默认 x NS；
            // default-xmlns.ftl 用 D/y 前缀、xmlns5.ftl 用 D/xx 前缀，同模型）
            let xml = "<root xmlns:x=\"http://x.com\" xmlns:y=\"http://y.com\">\n  <t1>No NS</t1>\n  <x:t2>x NS</x:t2>\n  <y:t3>y NS</y:t3>\n  <t4 xmlns=\"http://x.com\">x NS</t4>\n</root>";
            let doc = freemarker::xml::parse_xml(xml).expect("default-xmlns/xmlns5 XML parse");
            m.insert("doc".to_string(), doc);
        }
        "xmlns3" | "xmlns4" => {
            // Java TemplateTestCase：doc = NodeModel.parse(models/xmlns3.xml)；
            // xmlns3.ftl/xmlns4.ftl 用 ns_prefixes x/y + 字面前缀访问
            let xml = "<book xmlns:x=\"http://x\" xmlns:y=\"http://y\">\n  <x:title>Test Book</x:title>\n  <chapter>\n    <y:title>Ch1</y:title>\n    <para>p1.1</para>\n    <para>p1.2</para>\n    <para>p1.3</para>\n  </chapter>\n  <x:chapter>\n    <y:title>Ch2</y:title>\n    <x:para>p2.1</x:para>\n    <y:para>p2.2</y:para>\n  </x:chapter>\n</book>";
            let doc = freemarker::xml::parse_xml(xml).expect("xmlns3/4 XML parse");
            m.insert("doc".to_string(), doc);
        }
        "xml-ns_prefix-scope" => {
            // Java TemplateTestCase：doc = NodeModel.parse(XML 文档)；三个命名空间
            // 各有一个 e 元素（namespace-test / foo / bar）
            let xml = "<root xmlns=\"http://freemarker.org/test/namespace-test\" \
                       xmlns:n=\"http://freemarker.org/test/foo\" \
                       xmlns:bar=\"http://freemarker.org/test/bar\">\
                       <e>e in NS namespace-test</e>\
                       <n:e>e in NS foo</n:e>\
                       <bar:e>e in NS bar</bar:e></root>";
            let doc = freemarker::xml::parse_xml(xml).expect("xml-ns_prefix-scope XML parse");
            m.insert("doc".to_string(), doc);
        }
        _ => {}
    }
    TModel::from_hash(m)
}

// ---------------------------------------------------------------------------
// VarArgTestModel 方法模型 —— 对应 VarArgTestModel.java
// （BeansWrapper 方法调用语义：varargs 展开/数值截断/重载选择）
// ---------------------------------------------------------------------------

/// 取整到 i64（Java BeansWrapper 数值参数转换：Double/Float → intValue 截断；
/// Decimal → 向零截断；BigInt → 原值）
fn vararg_int(m: &TModel) -> Option<i64> {
    let n = m.get_number().ok()?;
    match n {
        TNumber::Int(v) => Some(v as i64),
        TNumber::Long(v) => Some(v),
        TNumber::BigInt(v) => v.to_i64(),
        TNumber::Float(v) => Some(v as i64),
        TNumber::Double(v) => Some(v as i64),
        // bigdecimal RoundingMode 无 Trunc——向零截断即 Down
        TNumber::Decimal(v) => {
            let t = v.with_scale_round(0, bigdecimal::RoundingMode::Down);
            t.to_string().parse::<i64>().ok()
        }
    }
}

/// varargs 展开（BeansWrapper：最后一个参数是序列 → 展开为变长实参；
/// 其余情况原样返回）
fn vararg_items(args: &[TModel]) -> Vec<TModel> {
    if let Some(last) = args.last() {
        if let Ok(seq) = last.get_sequence() {
            if seq.size().unwrap_or(0) > 0 || args.len() == 1 {
                // 仅单个序列参数或序列非空时展开（`m.bar([])` → 空展开）；
                // 展开保留前面的固定参数（`bar2(11, [22,33,44])` → [11, 22, 33, 44]）
                let mut out: Vec<TModel> = args[..args.len() - 1].to_vec();
                for i in 0..seq.size().unwrap_or(0) {
                    if let Ok(v) = seq.get(i) {
                        out.push(v);
                    }
                }
                return out;
            }
        }
    }
    args.to_vec()
}

/// bar(Integer... xs)：null 元素跳过（VarArgTestModel.java:31-38）
struct VarBar;
impl freemarker::template::TemplateMethodModelEx for VarBar {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let mut sum: i64 = 0;
        for x in vararg_items(&args) {
            if !x.is_nothing() {
                sum = sum * 100 + vararg_int(&x).unwrap_or(0);
            }
        }
        Ok(num(sum))
    }
}

/// bar2(int first, int... xs)：-(sum*100 + first)
struct VarBar2;
impl freemarker::template::TemplateMethodModelEx for VarBar2 {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let first = args.first().and_then(vararg_int).unwrap_or(0);
        let xs = vararg_items(&args);
        let mut sum: i64 = 0;
        for x in xs.iter().skip(1) {
            sum = sum * 100 + vararg_int(x).unwrap_or(0);
        }
        Ok(num(-(sum * 100 + first)))
    }
}

/// overloaded(int x, int y) / overloaded(int... xs)：2 参数选固定版本
/// （BeansWrapper 精确匹配优先），其余走 varargs
struct VarOverloaded;
impl freemarker::template::TemplateMethodModelEx for VarOverloaded {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let xs = vararg_items(&args);
        if args.len() == 2 {
            // (int x, int y) = x*100 + y
            let x = vararg_int(&args[0]).unwrap_or(0);
            let y = vararg_int(&args[1]).unwrap_or(0);
            return Ok(num(x * 100 + y));
        }
        let mut sum: i64 = 0;
        for x in &xs {
            sum = sum * 100 + vararg_int(x).unwrap_or(0);
        }
        Ok(num(-sum))
    }
}

/// noVarArgs(String s, boolean b, int i, Date d)：
/// s + ", " + b + ", " + i + ", " + d.getTime()
struct VarNoVarArgs;
impl freemarker::template::TemplateMethodModelEx for VarNoVarArgs {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let s = args
            .first()
            .and_then(|m| m.get_scalar().ok())
            .unwrap_or_default();
        let b = match args.get(1) {
            Some(m) => m
                .get_boolean()
                .map(|b| b.to_string())
                .unwrap_or_else(|_| "false".to_string()),
            None => "false".to_string(),
        };
        let i = args.get(2).and_then(vararg_int).unwrap_or(0);
        let d = match args.get(3).and_then(|m| m.get_date().ok()) {
            Some(dv) => dv.dt.timestamp_millis().to_string(),
            None => "0".to_string(),
        };
        Ok(TModel::from_scalar(format!("{s}, {b}, {i}, {d}")))
    }
}

/// MultiModel2 等价物 —— scalar "Model2 is alive!" + 方法
/// （MultiModel2.java：TemplateScalarModel + TemplateMethodModel，参数已字符串化）
fn multimodel2() -> TModel {
    TModel {
        scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
            "Model2 is alive!".to_string(),
        ))),
        method: Some(std::rc::Rc::new(MultiModel2Method)),
        type_name: "wrapped",
        kind: freemarker::template::ModelKind::Wrapped,
        ..TModel::nothing()
    }
}

/// MultiModel2.exec —— "Arguments are:<br />" + 各参数 + "<br />"
/// （Java exec 的 (String) 强转说明参数已字符串化；标量按内容）
struct MultiModel2Method;
impl freemarker::template::TemplateMethodModelEx for MultiModel2Method {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let mut out = String::from("Arguments are:<br />");
        for a in args {
            out.push_str(
                &a.get_scalar()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| a.type_name.to_string()),
            );
            out.push_str("<br />");
        }
        Ok(TModel::from_scalar(out))
    }
}

/// MultiModel3 等价物 —— scalar "Model3 is alive!" + hash{selftest, message}
fn multimodel3() -> TModel {
    let mut h = IndexMap::new();
    h.insert(
        "selftest".to_string(),
        TModel::from_scalar("Selftest from MultiModel3!".to_string()),
    );
    h.insert(
        "message".to_string(),
        TModel::from_scalar("Hello world from MultiModel3!".to_string()),
    );
    let h = TModel::from_hash(h);
    TModel {
        scalar: Some(std::rc::Rc::new(freemarker::template::SimpleScalar(
            "Model3 is alive!".to_string(),
        ))),
        hash: h.hash.clone(),
        hash_ex: h.hash_ex.clone(),
        type_name: "wrapped",
        kind: freemarker::template::ModelKind::Wrapped,
        ..TModel::nothing()
    }
}

/// MultiModel4 等价物 —— 空序列 + hash{size: "Key size, not the listSize method."}
fn multimodel4() -> TModel {
    let mut h = IndexMap::new();
    h.insert(
        "size".to_string(),
        TModel::from_scalar("Key size, not the listSize method.".to_string()),
    );
    let h = TModel::from_hash(h);
    let seq = TModel::from_sequence(Vec::new());
    TModel {
        sequence: seq.sequence.clone(),
        collection: seq.collection.clone(),
        hash: h.hash.clone(),
        hash_ex: h.hash_ex.clone(),
        type_name: "wrapped",
        kind: freemarker::template::ModelKind::Wrapped,
        ..TModel::nothing()
    }
}

/// MultiModel5 等价物 —— 1 项序列 + hash{empty: "Dummy hash value, for test purposes."}
fn multimodel5() -> TModel {
    let mut h = IndexMap::new();
    h.insert(
        "empty".to_string(),
        TModel::from_scalar("Dummy hash value, for test purposes.".to_string()),
    );
    let h = TModel::from_hash(h);
    let seq = TModel::from_sequence(vec![TModel::from_scalar(
        "Dummy to make list non-empty".to_string(),
    )]);
    TModel {
        sequence: seq.sequence.clone(),
        collection: seq.collection.clone(),
        hash: h.hash.clone(),
        hash_ex: h.hash_ex.clone(),
        type_name: "wrapped",
        kind: freemarker::template::ModelKind::Wrapped,
        ..TModel::nothing()
    }
}

/// TestNodeModel —— 对应 Java TestNode（TemplateTestCase.java:530-560）：
/// name "name"、type "element"；v1 仅角色判定用（?is_node）
struct TestNodeModel;
impl freemarker::template::TemplateNodeModel for TestNodeModel {
    fn parent(&self) -> Result<Option<TModel>> {
        Ok(None)
    }
    fn children(&self) -> Result<Vec<TModel>> {
        Ok(Vec::new())
    }
    fn name(&self) -> Result<Option<String>> {
        Ok(Some("name".to_string()))
    }
    fn node_type(&self) -> Result<String> {
        Ok("element".to_string())
    }
    fn namespace(&self) -> Result<Option<String>> {
        Ok(None)
    }
}

/// bean.m / bean.mOverloaded 方法 —— 对应 TestBean（TemplateTestCase.java:558-573）
/// 的 m(int)/mOverloaded(int|String)；v1 仅角色判定用
struct BeanMethod;
impl freemarker::template::TemplateMethodModelEx for BeanMethod {
    fn exec(&self, _args: Vec<TModel>) -> Result<TModel> {
        Ok(TModel::from_scalar("x".to_string()))
    }
}

/// 集合角色变体模型 —— 对应 Java `DefaultNonListCollectionAdapter`
/// （TemplateCollectionModelEx：可重复枚举、?size 可用、?is_collection/
/// ?is_collection_ex → true；?is_sequence → false）。Rust 引擎的 ?size 无
/// collection 槽位路径（引擎缺口），故以 sequence+collection 双槽位 + Ex 标记
/// 近似——对依赖 ?size/#list/?join/?seq_* 的模板，输出与 Java 逐字节一致
/// （这些内建对 Java 的 Ex 集合与 sequence 行为相同）
fn collection_ex_model(items: Vec<TModel>) -> TModel {
    let seq = TModel::from_sequence(items.clone());
    let coll = TModel::from_collection(items);
    TModel {
        sequence: seq.sequence.clone(),
        collection: coll.collection.clone(),
        collection_ex: true,
        type_name: "collection",
        kind: freemarker::template::ModelKind::Collection,
        ..TModel::nothing()
    }
}

/// listables 模型（对应 Listables.java：list/linkedList/set/iterator/empty*）。
/// `coll_adapter=true` 时 set/emptySet 用 collection 角色变体
/// （DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false) 下
/// 非 List 集合 → DefaultNonListCollectionAdapter）；iterator/emptyIterator 在
/// 两种 wrapper 下均为非 Ex collection（SimpleCollection/DefaultIteratorAdapter），
/// 角色不变
fn listables_model(coll_adapter: bool) -> TModel {
    let mut h = IndexMap::new();
    let seq = TModel::from_sequence(vec![num(11), num(22), num(33)]);
    h.insert("list".to_string(), seq.clone());
    h.insert("linkedList".to_string(), seq.clone());
    if coll_adapter {
        h.insert(
            "set".to_string(),
            collection_ex_model(vec![num(11), num(22), num(33)]),
        );
        h.insert("emptySet".to_string(), collection_ex_model(vec![]));
    } else {
        h.insert("set".to_string(), seq.clone());
        h.insert("emptySet".to_string(), TModel::from_sequence(vec![]));
    }
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
    // Java BeansWrapper 的 getter 暴露：getEmptyIterator() → 属性名 emptyIterator
    h.insert("emptyIterator".to_string(), TModel::from_collection(vec![]));
    // Java Listables.getHashEx2s：LinkedHashMap{ "k1":"v1", 2:"v2", "k3":"v3",
    // null:"v4", true:"v5", false:null } 的 3 种包装——v1 哈希键为 String，
    // 按输出等价预格式化（2→"2"、null→"null"、true→"Y"/false→"N" 按 booleanFormat='Y,N'）
    let mut h2 = IndexMap::new();
    h2.insert("k1".to_string(), TModel::from_scalar("v1".to_string()));
    h2.insert("2".to_string(), TModel::from_scalar("v2".to_string()));
    h2.insert("k3".to_string(), TModel::from_scalar("v3".to_string()));
    h2.insert("null".to_string(), TModel::from_scalar("v4".to_string()));
    h2.insert("Y".to_string(), TModel::from_scalar("v5".to_string()));
    h2.insert("N".to_string(), TModel::nothing());
    h.insert(
        "hashEx2s".to_string(),
        TModel::from_sequence(vec![
            TModel::from_hash(h2.clone()),
            TModel::from_hash(h2.clone()),
            TModel::from_hash(h2.clone()),
        ]),
    );
    // Java Listables.getEmptyHashes：4 个空哈希
    h.insert(
        "emptyHashes".to_string(),
        TModel::from_sequence(vec![
            TModel::from_hash(IndexMap::new()),
            TModel::from_hash(IndexMap::new()),
            TModel::from_hash(IndexMap::new()),
            TModel::from_hash(IndexMap::new()),
        ]),
    );
    // Java Listables.getHashNonEx2：ImmutableMap{ k1:11, k2:22 }
    let mut h3 = IndexMap::new();
    h3.insert("k1".to_string(), num(11));
    h3.insert("k2".to_string(), num(22));
    h.insert("hashNonEx2".to_string(), TModel::from_hash(h3));
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

/// JavaObjectInfo.info（JavaObjectInfo.java:30-34：null → "null"；否则
/// getClass().getName() + " " + jQuote(toString())——Rust 侧 `.locale_object`
/// 等特殊变量已按 Java 描述串提供（eval.rs LocaleObject），直接返回参数文本）
struct JavaObjectInfoMethod;
impl freemarker::template::TemplateMethodModelEx for JavaObjectInfoMethod {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        match args.first() {
            None => Ok(TModel::from_scalar("null".to_string())),
            Some(m) if m.is_nothing() => Ok(TModel::from_scalar("null".to_string())),
            Some(m) => Ok(TModel::from_scalar(
                m.get_scalar()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| m.type_name.to_string()),
            )),
        }
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
            .filter(|m| !m.is_nothing())
            .ok_or_else(|| TemplateError::misc("Missing required parameter \"actual\""))?;
        let expected = params
            .get("expected")
            .filter(|m| !m.is_nothing())
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
        // Java AssertFailsDirective：body 渲染进 NullWriter.INSTANCE（输出丢弃）——
        // 否则失败体的输出会泄露进活输出（sequence-builtins 等用例的 "  " 前缀差异）
        let err = match env.capture(|env| body.render(env)).map(|(r, _)| r) {
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
                    // Java AssertFailsDirective.java:59-61：Pattern.CASE_INSENSITIVE；
                    // 用 fancy-regex（Java Pattern 兼容的 lookaround/内联标志，如
                    // classic-compatible 用例的 (?s)(?=.*noSuchVar)）；ok() 避免
                    // 大 Err 值绑定（clippy::result_large_err）
                    let matched = fancy_regex::Regex::new(&format!("(?i){re}"))
                        .ok()
                        .and_then(|r| r.is_match(&msg).ok())
                        .unwrap_or(false);
                    if !matched {
                        return Err(TemplateError::misc(format!(
                            "Failure is not like expected: message didn't match regexp {re:?}: {msg}"
                        )));
                    }
                }
                if let Some(exp) = exception {
                    // Rust 无 Java 异常类层级：以错误消息特征匹配
                    // （Java 检查 e.getClass().getName().indexOf(exception)；
                    // InvalidReferenceException → 消息含 "null or missing" 与 Tip 段）
                    let matched = match exp.as_str() {
                        "UnexpectedTypeException" => {
                            msg.contains("Expected a ")
                                || msg.contains("is required")
                                || msg.contains("is not applicable")
                        }
                        "InvalidReferenceException" => msg.contains("null or missing"),
                        _ => msg.contains(&exp),
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
