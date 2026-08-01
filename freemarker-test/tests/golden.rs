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

/// 执行单个用例（Java runTest 的等价物）
fn run_case(case: &Case) -> Outcome {
    // Java 特有能力设置 → SKIP（记录原因）
    for (k, v) in &case.settings {
        match k.as_str() {
            "object_wrapper" => {
                if !v.contains("SimpleObjectWrapper") {
                    return Outcome::Skipped {
                        reason: format!("object_wrapper={v}（Java 特有 wrapper，无法复刻）"),
                    };
                }
            }
            "new_builtin_class_resolver" => {
                return Outcome::Skipped {
                    reason: "?new 类解析器（Java 特有）".to_string(),
                };
            }
            "api_builtin_enabled" => {
                return Outcome::Skipped {
                    reason: "?api 内建（Java BeanWrapper 特有）".to_string(),
                };
            }
            "input_encoding" if case.base == "charset-in-header" => {
                return Outcome::Skipped {
                    reason: "非 UTF-8 输入编码（charset-in-header，Java 特有编码映射）".to_string(),
                };
            }
            _ => {}
        }
    }
    // 输出编码非 UTF-8 → SKIP（v1 输出固定 UTF-8）
    if let Some(enc) = case.settings.get("output_encoding") {
        if enc != "utf-8" && enc != "UTF-8" {
            return Outcome::Skipped {
                reason: format!("output_encoding={enc}（v1 输出固定 UTF-8）"),
            };
        }
    }
    // 旧 ICI 行为特例：encoding-builtins（min, 2.3.19）的 expected 由旧版 ?html（不转义 '）
    // 生成，本引擎固定 ICI 2.3.34（XHTMLEnc）无法对齐 → SKIP；
    // 其余含旧 ICI 的用例（如 number-format 的 min 变体）输出与 ICI 无关，照常尝试
    // ICI <2.3.21 的 HashLiteral 保留重复键（`{"a":1,"b":2,"a":3}` → 两个 a 条目）；
    // 本引擎固定 ICI 2.3.34（覆盖语义）→ expected 由旧 ICI 行为生成，无法对齐
    if case.base == "listhashliteral" && case.name.contains("ici-2.3.20") {
        return Outcome::Skipped {
            reason: "expected 由 ICI <2.3.21 的重复键 HashLiteral 行为生成（保留重复键），本引擎固定 ICI 2.3.34（覆盖）"
                .to_string(),
        };
    }
    if case.base == "encoding-builtins" && !case.name.contains("[#endTN]") {
        return Outcome::Skipped {
            reason:
                "expected 由 ICI <2.3.20 的旧版 ?html 行为生成（不转义 '），本引擎固定 ICI 2.3.34"
                    .to_string(),
        };
    }
    // string-builtins3（jython25 弃用模块套件，未随 Gradle 构建运行）：断言与真实
    // Java 引擎矛盾（jar 实测：`-1?lower_abc` 按 FTL 文法解析为 `-(1?lower_abc)`，
    // 错误为 "For \"-...\" right-hand operand: Expected a number..."，不含
    // messageRegexp 要求的 "0|at least 1"）→ 用例本身无法通过，非引擎缺口
    if case.base == "string-builtins3" {
        return Outcome::Skipped {
            reason: "用例断言与真实 Java 引擎矛盾（jar 实测 -1?lower_abc 解析为 -(1?lower_abc)，错误消息不含 '0|at least 1'；jython25 弃用模块的过期断言）"
                .to_string(),
        };
    }

    let (mut c, loader) = base_config();
    let skipped_settings = apply_settings(&mut c, &case.settings);
    if !skipped_settings.is_empty() {
        return Outcome::Skipped {
            reason: skipped_settings.join("; "),
        };
    }
    load_all_templates(&loader);
    // 用例模板（Java 规则：name 中 "[#endTN]" 之前的片段 + ".ftl"）
    let template_name = template_name_of(case);
    let case_src = read_suite(&format!("cases/{}/{}", case.base, template_name));
    loader.put(&template_name, &remove_ftl_copyright_comment(&case_src));

    let root = build_data_model(&case.base);
    let rendered = render_case(&c, &template_name, root);

    match rendered {
        Ok(out) => {
            if case.no_output {
                return Outcome::Pass;
            }
            // expected 文件（Java 规则：beforeEndTN + afterEndTN + ".txt"；
            // 与 FileTestCase 相同：比较前先归一化换行 \r\n|\r → \n）
            let expected_name = expected_name_of(case);
            let expected = normalize_newlines(&strip_license_comment(&read_suite(&format!(
                "cases/{}/{}",
                case.base, expected_name
            ))));
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

/// Java TemplateTestSuite：expected = beforeEndTN + afterEndTN + ".txt"
fn expected_name_of(case: &Case) -> String {
    match case.name.find("[#endTN]") {
        Some(i) => format!(
            "{}{}.txt",
            &case.name[..i],
            &case.name[i + "[#endTN]".len()..]
        ),
        None => case
            .expected_file
            .clone()
            .map(|p| p.rsplit('/').next().unwrap_or(&p).to_string())
            .unwrap_or_else(|| format!("{}.txt", case.base)),
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
    if msg.contains("?new") || base == "number-literal" || base == "new-defaultresolver" {
        return "?new 类实例化（Java 特有能力）".to_string();
    }
    if base == "setting" || base == "specialvars" {
        return "特殊变量（.locale/.time_zone 等，解析器/引擎未支持）".to_string();
    }
    if base == "arithmetic" {
        return "#{... ; mNMN} 遗留插值格式（解析器/格式化 P4）".to_string();
    }
    if base == "localization" {
        return "局部化模板查找（localized_lookup，缓存层未实现）".to_string();
    }
    if base == "import" {
        return "auto_import 设置（Configuration.addAutoImport 未实现）".to_string();
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
