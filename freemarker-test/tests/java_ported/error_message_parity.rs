//! M5 错误对齐验收：70 个错误场景的消息与 Java 2.3.34 基线逐字对齐
//!
//! 场景表直接派生自 `scripts/java_probe/ProbeErrors.java` 的 SCENARIOS 表
//! （`include_str!` 编译期读取 + 解析），每个场景：
//!   1. 用 StringLoader 注册 `{name}.ftl` 模板（include 场景额外注册子模板）
//!   2. 用 JSON 数据构造数据模型（TModel；与 ProbeErrors 的 JsonParser 语义一致）
//!   3. 渲染并捕获完整错误消息（`TemplateException.getMessage()` 等价物——
//!      含 FTL stack trace 段；解析错误为 `Syntax error in template ...`）
//!   4. 按 docs/09 §4 容忍清单归一化后与 `freemarker/src/error/expected_messages/{name}.txt`
//!      逐字比较
//!
//! 归一化（与 /tmp/fmprobe/compare.py 同口径，对应 docs/09 §4 容忍差异清单）：
//!   - ` (wrapper: ...)` 后缀（Java 包装器类名）
//!   - ` (X wrapped into f.t.Y)` 后缀
//!   - `\n----\nJava stack trace ...` 段（Java 堆栈帧，Rust 用 FTL 指令栈替代）
//!   - `\nThe name was interpreted by this TemplateLoader:...` 行（机器相关路径）
//!   逐行 rstrip（尾部空白）
//!
//! 场景特例：
//!   - `missing_in_nested_if`：Java 不报错（`<#if false>` 内层不执行 `${missing}`）
//!     → 无基线文件，跳过并注明
//!   - `type_string_method`：有基线但 ProbeErrors 无场景（Java 原场景缺失）——
//!     按基线栈帧重建 `${x?matches("a")}` + `{"x": "abc"}`

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use indexmap::IndexMap;
use std::sync::Arc;

/// ProbeErrors.java 源码（编译期读取——场景表与探针单源）
const PROBE_ERRORS_JAVA: &str = include_str!("../../../scripts/java_probe/ProbeErrors.java");
/// 基线目录（相对本文件：freemarker-test/tests/java_ported/ → freemarker/src/error/expected_messages/）
const BASELINE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../freemarker/src/error/expected_messages/"
);

/// 场景表项（对应 ProbeErrors.SCENARIOS 一行：name, template, data-json-or-null）
struct Scenario {
    name: String,
    template: String,
    data_json: Option<String>,
}

/// 解析单个 Java 字符串字面量（`"..."`；仅处理场景表出现的 `\"` 转义），
/// 返回 (字面量值, 消费后剩余串)。失败返回 None（非字符串起点）
fn parse_java_string(s: &str) -> Option<(String, &str)> {
    let rest = s.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((out, &rest[i + c.len_utf8()..])),
            '\\' => match chars.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                Some((_, other)) => {
                    out.push('\\');
                    out.push(other);
                }
                None => return None,
            },
            _ => out.push(c),
        }
    }
    None
}

/// 从 ProbeErrors.java 解析 SCENARIOS 表（`{"name", "template", data-or-null}` 行）
fn parse_scenarios() -> Vec<Scenario> {
    let start = PROBE_ERRORS_JAVA
        .find("static final String[][] SCENARIOS = {")
        .expect("SCENARIOS 声明未找到");
    let body = &PROBE_ERRORS_JAVA[start..];
    let end = body.find("\n    };").expect("SCENARIOS 表结束未找到");
    let mut out = Vec::new();
    for line in body[..end].lines() {
        let line = line.trim();
        let rest = line.strip_prefix('{');
        let Some(mut rest) = rest else { continue };
        // 第 1 字段：场景名
        let (name, r) =
            parse_java_string(rest).unwrap_or_else(|| panic!("场景名解析失败: {line:?}"));
        rest = r.trim_start().strip_prefix(',').expect("逗号缺失");
        // 第 2 字段：模板源码
        let (template, r) = parse_java_string(rest.trim_start())
            .unwrap_or_else(|| panic!("模板解析失败: {line:?}"));
        rest = r.trim_start().strip_prefix(',').expect("逗号缺失");
        // 第 3 字段：数据 JSON 或 null
        let data = if rest.trim_start().starts_with("null") {
            None
        } else {
            let (j, _) = parse_java_string(rest.trim_start())
                .unwrap_or_else(|| panic!("数据 JSON 解析失败: {line:?}"));
            Some(j)
        };
        out.push(Scenario {
            name,
            template,
            data_json: data,
        });
    }
    out
}

/// JSON → TModel（对应 ProbeErrors.JsonParser 语义：对象→hash、数组→sequence、
/// 数字→Int/Long/Double、布尔/字符串/null 直映）
fn json_to_model(v: &serde_json::Value) -> TModel {
    match v {
        serde_json::Value::Null => TModel::nothing(),
        serde_json::Value::Bool(b) => TModel::from_boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TModel::from_number(TNumber::from_i64(i))
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f.is_finite() {
                    TModel::from_number(TNumber::from_i64(f as i64))
                } else {
                    TModel::from_number(TNumber::Double(f))
                }
            } else {
                TModel::nothing()
            }
        }
        serde_json::Value::String(s) => TModel::from_scalar(s.clone()),
        serde_json::Value::Array(arr) => {
            TModel::from_sequence(arr.iter().map(json_to_model).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = IndexMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_model(v));
            }
            TModel::from_hash(map)
        }
    }
}

/// 归一化 —— docs/09 §4 容忍清单（与 /tmp/fmprobe/compare.py normalize 同口径）：
/// 剥 `(wrapper: ...)`、`(X wrapped into f.t.Y)`、`Java stack trace` 段、
/// `The name was interpreted by this TemplateLoader` 行；逐行 rstrip
fn normalize(msg: &str) -> String {
    let mut s = msg.to_string();
    // ` (wrapper: SimpleSequence)` 后缀（Java 包装器类名）
    loop {
        let before = s.clone();
        if let Some(i) = s.find(" (wrapper: ") {
            if let Some(j) = s[i + 11..].find(')') {
                s.replace_range(i..i + 11 + j + 1, "");
            }
        }
        // ` (HashMap wrapped into f.t.DefaultMapAdapter)` 后缀（含前导空格，
        // 对齐 compare.py 的 ` \([^)]* wrapped into f\.t\.[^)]*\)`）
        if let Some(i) = s.find(" wrapped into f.t.") {
            let start = s[..i].rfind('(');
            if let Some(st) = start {
                if let Some(j) = s[i..].find(')') {
                    // 连前导空格一起移除（Java 描述 "a sequence (X wrapped ...)" 的
                    // 空格属于 wrapper 后缀；正则式同 compare.py）
                    let lead = if st > 0 && s.as_bytes()[st - 1] == b' ' {
                        st - 1
                    } else {
                        st
                    };
                    s.replace_range(lead..i + j + 1, "");
                }
            }
        }
        if s == before {
            break;
        }
    }
    // Java stack trace 段（`\n----\nJava stack trace ...`；Rust 用 FTL 指令栈替代）
    if let Some(i) = s.find("\n----\nJava stack trace") {
        s.truncate(i);
    }
    // 机器相关 loader 行（include_not_found：Java FileTemplateLoader 临时目录路径）。
    // 注意跳过前导 `\n`（否则 find 立即命中位置 0，end==i，剥除落空）
    if let Some(i) = s.find("\nThe name was interpreted by this TemplateLoader:") {
        let rest = &s[i + 1..];
        let end = rest.find('\n').map(|e| i + 1 + e).unwrap_or(s.len());
        s.replace_range(i..end, "");
    }
    // 逐行 rstrip（Java 基线行尾空白 vs Rust 输出）
    s.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end_matches('\n')
        .to_string()
}

/// 渲染单个场景，返回完整错误消息（Java e.getMessage() 等价物；解析错误在
/// get_template 阶段抛出——Java ParseException 同为加载期错误）
fn render_scenario(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    name: &str,
    template: &str,
    data_json: Option<&str>,
) -> String {
    loader.put(&format!("{name}.ftl"), template);
    // include 场景的子模板（Java ProbeErrors.runScenario 同款）
    if name == "include_parse_error" {
        loader.put("broken.ftl", "<#if x>");
    }
    if name == "missing_in_include_body" {
        loader.put("sub.ftl", "${missing}");
    }
    let root = match data_json {
        Some(j) => json_to_model(&serde_json::from_str(j).expect("数据 JSON 非法")),
        None => TModel::from_hash(IndexMap::new()),
    };
    let t = match c.get_template(&format!("{name}.ftl")) {
        Ok(t) => t,
        Err(e) => return e.to_user_message(),
    };
    let mut out = Vec::new();
    match t.process(root, &mut out) {
        Ok(_) => panic!(
            "模板未报错（Java 同场景必失败）: {name}: {}",
            String::from_utf8_lossy(&out)
        ),
        Err(e) => e.to_user_message(),
    }
}

/// 单场景断言：渲染 → 归一化 → 与基线逐字比较（失败时打印完整 diff 信息）
fn assert_scenario(c: &Configuration, loader: &Arc<StringLoader>, scenario: &Scenario) {
    let got_raw = render_scenario(
        c,
        loader,
        &scenario.name,
        &scenario.template,
        scenario.data_json.as_deref(),
    );
    let expected_raw = std::fs::read_to_string(format!("{BASELINE_DIR}{}.txt", scenario.name))
        .unwrap_or_else(|e| panic!("基线文件缺失 {}.txt: {e}", scenario.name));
    let got = normalize(&got_raw);
    let expected = normalize(&expected_raw);
    assert_eq!(
        got, expected,
        "场景 {} 消息与 Java 2.3.34 基线不一致\n\
         --- 基线（归一化后） ---\n{}\n\
         --- Rust（归一化后） ---\n{}\n\
         --- Rust 原始消息 ---\n{}",
        scenario.name, expected, got, got_raw
    );
}

/// 配置 —— 对齐 Java ProbeErrors.runScenario：
/// ICI 2.3.34（引擎固定）、numberFormat 默认、booleanFormat 默认 legacy "true,false"
/// （type_index_boolean/boolean_format_legacy/type_string_method 依赖）、
/// RETHROW handler（Settings 默认 "rethrow"）；模板加载走 StringLoader
fn parity_config() -> (Configuration, Arc<StringLoader>) {
    let mut c = Configuration::default();
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    (c, loader)
}

/// 全量 70 场景逐字对齐（Java 2.3.34 jar 实测基线）
#[test]
fn error_message_parity_all_scenarios() {
    let scenarios = parse_scenarios();
    assert_eq!(scenarios.len(), 70, "ProbeErrors.SCENARIOS 应为 70 个场景");
    let (c, loader) = parity_config();
    let mut checked = 0usize;
    let mut skipped = Vec::new();
    for sc in &scenarios {
        if sc.name == "missing_in_nested_if" {
            // Java 不报错：`<#if true><#if false>${missing}</#if></#if>` 内层
            // false 分支不求值，ProbeErrors 抛 "Template did NOT fail!" ——
            // 无基线文件，跳过（引擎行为一致：不报错）
            skipped.push(sc.name.clone());
            continue;
        }
        assert_scenario(&c, &loader, sc);
        checked += 1;
    }
    // type_string_method：基线存在但 SCENARIOS 无对应场景（Java 原场景缺失）——
    // 按基线栈帧 `${x?matches("a")}` 重建：x="abc" 时 ?matches 返回布尔，
    // 插值打印走 legacy boolean_format 报错（type_string_method.txt 同款消息）
    let extra = Scenario {
        name: "type_string_method".to_string(),
        template: "${x?matches(\"a\")}".to_string(),
        data_json: Some("{\"x\": \"abc\"}".to_string()),
    };
    assert_scenario(&c, &loader, &extra);
    checked += 1;
    assert_eq!(
        checked, 70,
        "应校验 70 个场景（69 场景表 + type_string_method）"
    );
    assert_eq!(
        skipped,
        vec!["missing_in_nested_if".to_string()],
        "仅 missing_in_nested_if 应跳过"
    );
    println!("error_message_parity: {checked}/70 场景与 Java 2.3.34 基线逐字对齐");
}
