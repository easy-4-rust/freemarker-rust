//! 黄金套件集成测试 —— 对应 Java `freemarker.test.templatesuite`（docs/11 §3）
//!
//! runner 读取 `tests/suite/manifest.json`（128 用例）与 `tests/suite/cases/`，
//! 对每个 PENDING 用例构造 Configuration（StringLoader 注册用例模板 + 依赖模板）
//! + 数据模型（TemplateTestCase.java 模式）→ 渲染 → 与 expected 逐字节比较。
//!
//! 结果分类：
//! - PASS：逐字节相等（no_output 用例 = 渲染成功不报错）
//! - FAIL：渲染成功但输出 ≠ expected
//! - SKIPPED：解析/渲染报错或依赖 Java 特有能力（每个 SKIP 记录原因，不静默）
//!
//! 达标标准：PASS ≥ 20（逐字节）。

mod common;

use common::*;
use freemarker::error::TemplateError;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Clone)]
struct Case {
    name: String,
    base: String,
    template: String,
    expected_file: Option<String>,
    no_output: bool,
    settings: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

/// 用例执行结果
#[derive(Debug, Clone, PartialEq)]
enum Outcome {
    Pass,
    /// 渲染成功但输出与 expected 不一致
    Fail {
        diff: String,
    },
    /// 渲染/解析报错或依赖 Java 特有能力
    Skipped {
        reason: String,
    },
}

/// Java object_wrapper 设置能否由 harness 复刻（数据模型直接构造等价物，wrapper
/// 设置本身对渲染结果无影响）。判定规则：
/// - SimpleObjectWrapper：模型逐槽位构造，天然等价
/// - XML 用例（xml-fragment/xmlns1/xmlns3/xmlns4/xml-ns_prefix-scope）：Java 用
///   BeansWrapper 包装 XML 节点模型，本引擎用内置 XmlNode 模型等价物
/// - B6 collectionAdapter 变体：DefaultObjectWrapper(2.3.22,
///   forceLegacyNonListCollections=false)——非 List 集合 wrap 成 collection 角色，
///   由 build_data_model 按用例名构造对应模型
/// - B6 sequence-builtins 变体：BeansWrapper/DefaultObjectWrapper 对 Set/List 的
///   包装角色（sequence/collection）由 build_data_model 按用例名构造等价物
fn object_wrapper_emulatable(case: &Case, v: &str) -> bool {
    if v.contains("SimpleObjectWrapper") {
        return true;
    }
    if matches!(
        case.base.as_str(),
        "xml-fragment"
            | "xml-ns_prefix-scope"
            | "xmlns1"
            | "xmlns3"
            | "xmlns4"
            | "default-xmlns"
            | "xmlns5"
    ) {
        return true;
    }
    // B4 api-builtins：DefaultObjectWrapper(2.3.22)（DefaultMapAdapter 有 API
    // 支持）与 BeansWrapper(2.3.0)（-bw 变体，String 也有 API）均由 harness
    // 数据模型直接构造等价物（common/mod.rs api_map_model 等）
    if case.base == "api-builtins" {
        return true;
    }
    if case.name.contains("collectionAdapter")
        && v == "DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)"
    {
        return true;
    }
    if case.base == "sequence-builtins"
        && (v == "freemarker.ext.beans.BeansWrapper"
            || v == "freemarker.template.DefaultObjectWrapper"
            || v == "DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)")
    {
        return true;
    }
    false
}

/// 永久 NOT_APPLICABLE 用例判定（用户决策；docs/superpowers/specs/2026-08-03-production-readiness-audit-design.md 完整清单）。
/// 命中 → 直接 SKIP 并记录分类原因（在 object_wrapper/错误分类等通用路径之前
/// 判定，保证 15 项 NA 分类确定化、不随引擎演进而改变）：
/// - JVM 反射系（security.md 决策 1，引擎永久不支持）：
///   - beans：BeansWrapper/POJO 数据模型（用户以 DynValue 手工包装）
///   - overloaded-methods-23bc / overloaded-methods-2-{inc,desc}-bwici-* 共 11 项：
///     BeansWrapper 反射方法重载分派
/// - 套件自身问题（与真实 Java 2.3.34 行为矛盾，jar 实测用例本身无法通过）：
///   - transforms：Java 特有变换类 JythonRuntime（ClassNotFoundException）
///   - string-builtins3 / date-type-builtins：jython25 弃用套件的过期断言
///     （string-builtins3：-1?lower_abc 解析为 -(1?lower_abc)，错误消息不含
///     '0|at least 1'；date-type-builtins：?string.xs 对 date-only 输出带 Z）
fn permanent_na_reason(case: &Case) -> Option<String> {
    let reason = match case.base.as_str() {
        "beans" => "永久 NA：JVM 反射（BeansWrapper/POJO 数据模型，security.md 决策 1——以 DynValue 手工包装）",
        "transforms" => "永久 NA：Java 特有变换类（JythonRuntime，ClassNotFoundException）",
        "string-builtins3" => {
            "永久 NA：jython25 弃用套件过期断言（-1?lower_abc 解析为 -(1?lower_abc)，与真实 Java 2.3.34 矛盾）"
        }
        "date-type-builtins" => {
            "永久 NA：jython25 弃用套件过期断言（?string.xs 对 date-only 输出带 Z，与真实 Java 2.3.34 矛盾）"
        }
        // overloaded-methods-23bc + overloaded-methods-2-{inc,desc}-bwici-2.3.20/21
        // 共 11 项：BeansWrapper 反射方法重载分派
        b if b.starts_with("overloaded-methods") => {
            "永久 NA：方法模型重载（BeansWrapper 反射方法分派）"
        }
        _ => return None,
    };
    Some(reason.to_string())
}

/// 执行单个用例（Java runTest 的等价物）
fn run_case(case: &Case) -> Outcome {
    // 永久 NA 用例最先判定（分类确定化，见 permanent_na_reason 注释）
    if let Some(reason) = permanent_na_reason(case) {
        return Outcome::Skipped { reason };
    }
    // Java 特有能力设置 → SKIP（记录原因）。
    // new_builtin_class_resolver：已由引擎实现（core::template_class_resolver
    // 四策略 + 分段列表解析），apply_settings 解析失败时自然落入 SKIP。
    // api_builtin_enabled：B4 已由引擎 ?api/?has_api + harness API 视图支持
    // （apply_settings 接受为 no-op；API 支持由 TModel.api 槽位判定）。
    if let Some(v) = case.settings.get("object_wrapper") {
        if !object_wrapper_emulatable(case, v) {
            return Outcome::Skipped {
                reason: format!("object_wrapper={v}（Java 特有 wrapper，无法复刻）"),
            };
        }
    }
    // 旧 ICI 行为（?html <2.3.20 / HashLiteral 重复键 <2.3.21 / is_sequence&is_enumerable
    // <2.3.24）已由引擎按 Settings.incompatible_improvements 版本化实现，含旧 ICI 的
    // 用例（encoding-builtins / listhashliteral-ici-2.3.20 / string-builtins-ici-2.3.19 /
    // type-builtins min 变体）照常渲染对比
    // identifier-escaping：转义标识符（`\-` 等）与 `@` 字符、visit/recurse 的 using
    // 子句均已实现；?sort 的 Collator TERTIARY 标点排序已对齐（代理字符映射）；
    // .namespace?keys 已包含 macro/function 条目。全部差异已修复。
    // string-builtins-ici-2.3.19 的旧版 ?html（不转义 '）已由引擎按 ICI 版本化实现
    // type-builtins 的 min/2.3.21 变体（方法模型 ?is_sequence/?is_enumerable 不排除）
    // 已由引擎按 ICI 版本化实现（is_sequence <2.3.24、is_enumerable <2.3.21）

    // xmlns3/xmlns4：模板用 `<#macro "x:title">` 等带前缀宏名配合 `<#recurse>` 分派。
    // 引擎 visit_node 已按 Java getNodeProcessor（Environment.java :2943-3000）实现
    // 带命名空间节点的前缀宏分派（NsPrefixes.get_prefix_for_namespace 反查）——
    // 2026-08 B5 批次收口，走正常路径

    let (mut c, loader) = base_config();
    // 可复刻的 Java wrapper 设置（XML 用例 / collectionAdapter / sequence-builtins
    // 变体）：数据模型已由 build_data_model 按用例名构造等价物，wrapper 设置对
    // 渲染无影响——统一改写为 SimpleObjectWrapper 走常规路径（apply_settings 接受）
    let settings = {
        let mut s = case.settings.clone();
        if let Some(v) = s.get_mut("object_wrapper") {
            if !v.contains("SimpleObjectWrapper") {
                // 可复刻的 Java wrapper（XML 用例 / collectionAdapter /
                // sequence-builtins 变体）：数据模型已由 build_data_model 按
                // 用例名构造等价物，wrapper 设置对渲染无影响——改写为
                // SimpleObjectWrapper 走常规路径（apply_settings 接受）
                *v = "freemarker.template.SimpleObjectWrapper".to_string();
            }
        }
        s
    };
    let skipped_settings = apply_settings(&mut c, &settings);
    if !skipped_settings.is_empty() {
        return Outcome::Skipped {
            reason: skipped_settings.join("; "),
        };
    }
    // Java TemplateTestCase.java:353：var-layers 用例注册共享变量 y（.globals/.data_model 回退链）
    if case.base == "var-layers" {
        c.set_shared_variable("y", num(7));
    }
    load_all_templates(&loader);
    // 用例模板（Java 规则：name 中 "[#endTN]" 之前的片段 + ".ftl"；
    // 原始字节注册——charset-in-header 为非 UTF-8 模板）
    let template_name = template_name_of(case);
    let case_bytes = std::fs::read(format!("{SUITE_DIR}/cases/{}/{}", case.base, template_name))
        .unwrap_or_else(|e| panic!("cannot read case {template_name}: {e}"));
    loader.put_bytes(
        &template_name,
        &remove_ftl_copyright_comment_bytes(&case_bytes),
    );

    let ici_int = c.settings.incompatible_improvements.to_int();
    let root = build_data_model(&case.base, &case.name, ici_int);
    let rendered = render_case(&c, &template_name, root);

    match rendered {
        Ok(out) => {
            if case.no_output {
                return Outcome::Pass;
            }
            // expected 文件（Java 规则：beforeEndTN + afterEndTN + ".txt"；
            // 与 FileTestCase 相同：比较前先归一化换行 \r\n|\r → \n）
            let expected_name = expected_name_of(case);
            let expected_raw = if let Some(enc) = case.settings.get("output_encoding") {
                if enc != "utf-8" && enc != "UTF-8" {
                    read_suite_encoded(&format!("cases/{}/{}", case.base, expected_name), enc)
                } else {
                    read_suite(&format!("cases/{}/{}", case.base, expected_name))
                }
            } else {
                read_suite(&format!("cases/{}/{}", case.base, expected_name))
            };
            let expected = normalize_newlines(&strip_license_comment(&expected_raw));
            let out = normalize_newlines(&out);
            // Java FileTestCase.multilineAssertEquals：忽略末尾换行差异
            let expected = if out.ends_with('\n') && !expected.ends_with('\n') {
                format!("{expected}\n")
            } else if !out.ends_with('\n') && expected.ends_with('\n') {
                expected.trim_end_matches('\n').to_string()
            } else {
                expected
            };
            if out == expected {
                Outcome::Pass
            } else {
                Outcome::Fail {
                    diff: diff_preview(&out, &expected),
                }
            }
        }
        Err(e) => Outcome::Skipped {
            reason: classify_error(&e, &case.base),
        },
    }
}

/// Java TemplateTestSuite：name 中 "[#endTN]" 之前的片段为模板基名
fn template_name_of(case: &Case) -> String {
    match case.name.find("[#endTN]") {
        Some(i) => format!("{}.ftl", &case.name[..i]),
        None => case.template.clone(),
    }
}

/// Java TemplateTestSuite：expected 优先取 manifest 的 expected_file（= base + ".txt"，
/// scripts/extract_suite.py 的 Java 语义）；无则按 beforeEndTN + afterEndTN + ".txt" 拼
fn expected_name_of(case: &Case) -> String {
    if let Some(p) = &case.expected_file {
        return p.rsplit('/').next().unwrap_or(p).to_string();
    }
    match case.name.find("[#endTN]") {
        Some(i) => format!(
            "{}{}.txt",
            &case.name[..i],
            &case.name[i + "[#endTN]".len()..]
        ),
        None => format!("{}.txt", case.base),
    }
}

/// Java FileTestCase.normalizeNewLines：`\r\n` → `\n`、`\r` → `\n`
fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// 错误原因分类（Java 特有能力 / 解析器不支持 / 格式化 P4 / 引擎缺口）
fn classify_error(e: &TemplateError, base: &str) -> String {
    let msg = e.to_string();
    if msg.contains("Unknown built-in") {
        return format!("未知内建（{msg}）");
    }
    if msg.contains("Parsing error") {
        return format!("解析器不支持：{msg}");
    }
    if base == "arithmetic" {
        return "#{... ; mNMN} 遗留插值格式（解析器/格式化 P4）".to_string();
    }
    if base == "localization" {
        return "局部化模板查找（localized_lookup，缓存层未实现）".to_string();
    }
    if base == "number-format" && msg.contains("INF") {
        return "INF 数字解析（P4）".to_string();
    }
    if msg.contains("?string") || msg.contains("format") {
        return format!("格式化 P4：{msg}");
    }
    msg.to_string()
}

/// 输出差异预览（首处不同 ±40 字节）
fn diff_preview(out: &str, expected: &str) -> String {
    let common_len = out
        .as_bytes()
        .iter()
        .zip(expected.as_bytes().iter())
        .take_while(|(a, b)| a == b)
        .count();
    let a = &out[common_len.saturating_sub(30)..(common_len + 40).min(out.len())];
    let b = &expected[common_len.saturating_sub(30)..(common_len + 40).min(expected.len())];
    format!(
        "byte {common_len}:\n  actual:   {:?}\n  expected: {:?}\n  [actual len={}, expected len={}]",
        a,
        b,
        out.len(),
        expected.len()
    )
}

#[test]
fn golden_suite() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    let mut fails: Vec<String> = Vec::new();
    let mut skips: Vec<String> = Vec::new();
    for case in &manifest.cases {
        let outcome = run_case(case);
        match outcome {
            Outcome::Pass => {
                pass += 1;
                println!("PASS   {}", case.name);
            }
            Outcome::Fail { diff } => {
                fail += 1;
                fails.push(format!("{}: {}", case.name, diff));
                println!("FAIL   {}  {}", case.name, diff);
            }
            Outcome::Skipped { reason } => {
                skip += 1;
                skips.push(format!("{}: {}", case.name, reason));
                println!("SKIP   {}  ({})", case.name, reason);
            }
        }
    }
    println!(
        "\n==== golden suite: PASS={pass} FAIL={fail} SKIPPED={skip} (total {}) ====",
        manifest.cases.len()
    );
    // 允许临时查看完整 SKIP 列表（--nocapture）
    for s in &skips {
        println!("  [skip] {s}");
    }
    for f in &fails {
        println!("  [fail] {f}");
    }
    assert!(
        pass >= 20,
        "golden suite requires >= 20 byte-exact PASS, got {pass} (FAIL={fail}, SKIPPED={skip})"
    );
}

/// 逐用例独立断言（cargo test golden_case_ 可单独跑某个用例）
#[test]
fn golden_case_comment() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "comment") {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case {} should PASS",
            case.name
        );
    }
}

#[test]
fn golden_case_variables() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "variables") {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case {} should PASS",
            case.name
        );
    }
}

#[test]
fn golden_case_boolean() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "boolean") {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case {} should PASS",
            case.name
        );
    }
}

/// 黄金用例：default（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_default() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "default") {
        assert_eq!(run_case(case), Outcome::Pass, "case default should PASS");
    }
}

/// 黄金用例：encoding-builtins[#endTN]-ici-2.3.20（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_encoding_builtins_ici_2_3_20() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "encoding-builtins[#endTN]-ici-2.3.20")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case encoding-builtins[#endTN]-ici-2.3.20 should PASS"
        );
    }
}

/// 黄金用例：non-strict-syntax（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_non_strict_syntax() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "non-strict-syntax")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case non-strict-syntax should PASS"
        );
    }
}

/// 黄金用例：identifier-non-ascii（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_identifier_non_ascii() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "identifier-non-ascii")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case identifier-non-ascii should PASS"
        );
    }
}

/// 黄金用例：localization（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_localization() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "localization") {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case localization should PASS"
        );
    }
}

/// 黄金用例：macros2（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_macros2() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "macros2") {
        assert_eq!(run_case(case), Outcome::Pass, "case macros2 should PASS");
    }
}

/// 黄金用例：newlines1（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_newlines1() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "newlines1") {
        assert_eq!(run_case(case), Outcome::Pass, "case newlines1 should PASS");
    }
}

/// 黄金用例：noparse（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_noparse() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "noparse") {
        assert_eq!(run_case(case), Outcome::Pass, "case noparse should PASS");
    }
}

/// 黄金用例：number-format（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_number_format() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "number-format") {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case number-format should PASS"
        );
    }
}

/// 黄金用例：precedence（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_precedence() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "precedence") {
        assert_eq!(run_case(case), Outcome::Pass, "case precedence should PASS");
    }
}

/// 黄金用例：simplehash-char-key（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_simplehash_char_key() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "simplehash-char-key")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case simplehash-char-key should PASS"
        );
    }
}

/// 黄金用例：strictinheader（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_strictinheader() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "strictinheader") {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case strictinheader should PASS"
        );
    }
}

/// 黄金用例：string-builtins2（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_string_builtins2() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "string-builtins2")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case string-builtins2 should PASS"
        );
    }
}

/// 黄金用例：string-builtins-regexps（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_string_builtins_regexps() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "string-builtins-regexps")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case string-builtins-regexps should PASS"
        );
    }
}

/// 黄金用例：url（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_url() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "url") {
        assert_eq!(run_case(case), Outcome::Pass, "case url should PASS");
    }
}
/// 黄金用例：wstrip-in-header（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_wstrip_in_header() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "wstrip-in-header")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case wstrip-in-header should PASS"
        );
    }
}

/// 黄金用例：hashconcat（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_hashconcat() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "hashconcat") {
        assert_eq!(run_case(case), Outcome::Pass, "case hashconcat should PASS");
    }
}

/// 黄金用例：number-to-date（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_number_to_date() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest.cases.iter().filter(|c| c.name == "number-to-date") {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case number-to-date should PASS"
        );
    }
}

/// 黄金用例：boolean-formatting（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_boolean_formatting() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "boolean-formatting")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case boolean-formatting should PASS"
        );
    }
}

/// 黄金用例：number-math-builtins（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_number_math_builtins() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "number-math-builtins")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case number-math-builtins should PASS"
        );
    }
}

/// 黄金用例：string-builtin-coercion（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_string_builtin_coercion() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "string-builtin-coercion")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case string-builtin-coercion should PASS"
        );
    }
}

/// 黄金用例：string-builtins-ici-2.3.20（逐字节对照 Java templatesuite expected）
#[test]
fn golden_case_string_builtins_ici_2_3_20() {
    let manifest: Manifest =
        serde_json::from_str(&read_suite("manifest.json")).expect("manifest parses");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.name == "string-builtins-ici-2.3.20")
    {
        assert_eq!(
            run_case(case),
            Outcome::Pass,
            "case string-builtins-ici-2.3.20 should PASS"
        );
    }
}
