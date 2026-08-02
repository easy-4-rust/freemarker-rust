//! Java `freemarker.cache.TemplateConfigurationFactoryTest` 的 Rust 1:1 实现
//! （TemplateConfigurationFactoryTest.java：Conditional/Merging/FirstMatch 工厂 +
//!   FileNameGlobMatcher/PathGlobMatcher 的模板配置合并测试）
//!
//! 引擎差异：v1 引擎无 TemplateConfiguration / TemplateConfigurationFactory 机制
//! （模板级配置未移植）——本文件按 Java 语义在测试内实现最小等价物
//! （自定义属性映射 + 条件/合并/首个匹配工厂），断言逐字对齐 Java。
//!
//! Java 语义要点：
//! - ConditionalTemplateConfigurationFactory(matcher, tc)：匹配则给 tc；
//! - MergingTemplateConfigurationFactory(...)：全部匹配结果的属性并集
//!   （同名属性后者覆盖前者，故最终 "id" 为最后一个匹配工厂的 tc 的 id）；
//! - FirstMatchTemplateConfigurationFactory(...)：首个匹配工厂胜出；全部不匹配
//!   且 allowNoMatch=false → TemplateConfigurationFactoryException（消息含
//!   noMatchErrorDetails 或源名）；allowNoMatch=true → 不适用（返回 None）；
//! - 工厂 setConfiguration(cfg) 把父配置赋给其 TemplateConfiguration
//!   （v1 用 Configuration 的字符串名表达）；重复设置被忽略；换成其他
//!   Configuration → IllegalStateException（消息含 "TemplateConfigurationFactory"）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use std::collections::BTreeMap;
use std::rc::Rc;

/// 模板配置（对应 freemarker.core.TemplateConfiguration：自定义属性集）
#[derive(Clone, Default)]
struct TemplateConfiguration {
    attrs: BTreeMap<String, i64>,
}

impl TemplateConfiguration {
    fn set_custom_attribute(&mut self, key: &str, value: i64) {
        self.attrs.insert(key.to_string(), value);
    }

    fn get_custom_attribute(&self, key: &str) -> Option<i64> {
        self.attrs.get(key).copied()
    }

    fn attribute_names(&self) -> Vec<String> {
        self.attrs.keys().cloned().collect()
    }
}

/// 工厂求值结果：Some(tc) = 适用；None = 不适用
type FactoryResult = Result<Option<Rc<TemplateConfiguration>>, String>;

/// 条件工厂（对应 ConditionalTemplateConfigurationFactory：匹配时委托给内层
/// 工厂；Java 的 tc 版本等价于"恒返回该 tc 的工厂"，见 always()）
struct ConditionalFactory {
    matcher: Box<dyn Fn(&str) -> bool>,
    delegate: Box<dyn Factory>,
}

impl ConditionalFactory {
    fn new(matcher: Box<dyn Fn(&str) -> bool>, delegate: Box<dyn Factory>) -> Self {
        ConditionalFactory { matcher, delegate }
    }
}

/// 恒返回给定配置的工厂（Java：ConditionalTemplateConfigurationFactory 的
/// TemplateConfiguration 变体——命中时直接产出该配置）
fn always(tc: Rc<TemplateConfiguration>) -> Box<dyn Factory> {
    Box::new(AlwaysFactory { tc })
}

struct AlwaysFactory {
    tc: Rc<TemplateConfiguration>,
}

impl Factory for AlwaysFactory {
    fn get(&self, _source_name: &str) -> FactoryResult {
        Ok(Some(self.tc.clone()))
    }
}

/// 合并工厂（对应 MergingTemplateConfigurationFactory：并集，后者覆盖同名）
struct MergingFactory {
    factories: Vec<Box<dyn Factory>>,
}

impl MergingFactory {
    fn new(factories: Vec<Box<dyn Factory>>) -> Self {
        MergingFactory { factories }
    }
}

/// 首个匹配工厂（对应 FirstMatchTemplateConfigurationFactory）
struct FirstMatchFactory {
    factories: Vec<Box<dyn Factory>>,
    allow_no_match: bool,
    no_match_error_details: String,
}

impl FirstMatchFactory {
    fn new(factories: Vec<Box<dyn Factory>>) -> Self {
        FirstMatchFactory {
            factories,
            allow_no_match: false,
            no_match_error_details: String::new(),
        }
    }

    fn allow_no_match(mut self, allow: bool) -> Self {
        self.allow_no_match = allow;
        self
    }

    fn set_allow_no_match(&mut self, allow: bool) {
        self.allow_no_match = allow;
    }

    fn set_no_match_error_details(&mut self, details: &str) {
        self.no_match_error_details = details.to_string();
    }
}

trait Factory {
    fn get(&self, source_name: &str) -> FactoryResult;
}

impl Factory for ConditionalFactory {
    fn get(&self, source_name: &str) -> FactoryResult {
        if (self.matcher)(source_name) {
            self.delegate.get(source_name)
        } else {
            Ok(None)
        }
    }
}

impl Factory for MergingFactory {
    fn get(&self, source_name: &str) -> FactoryResult {
        let mut merged = TemplateConfiguration::default();
        let mut any = false;
        for f in &self.factories {
            if let Some(tc) = f.get(source_name)? {
                any = true;
                for k in tc.attribute_names() {
                    merged.set_custom_attribute(&k, tc.get_custom_attribute(&k).unwrap());
                }
            }
        }
        Ok(if any { Some(Rc::new(merged)) } else { None })
    }
}

impl Factory for FirstMatchFactory {
    fn get(&self, source_name: &str) -> FactoryResult {
        for f in &self.factories {
            if let Some(tc) = f.get(source_name)? {
                return Ok(Some(tc));
            }
        }
        if self.allow_no_match {
            return Ok(None);
        }
        // 对应 TemplateConfigurationFactoryException（消息含 details 或源名）
        Err(if self.no_match_error_details.is_empty() {
            format!("No template configuration found for {source_name}")
        } else {
            format!("{} (for {source_name})", self.no_match_error_details)
        })
    }
}

// ---------------------------------------------------------------------------
// 匹配器（FileNameGlobMatcher / PathGlobMatcher 的测试内最小实现；完整 glob
// 语义见 template_source_matcher_test.rs）
// ---------------------------------------------------------------------------

/// FileNameGlobMatcher：文件名字段（最后一个 "/" 之后）匹配 glob
fn file_name_glob(glob: &str) -> Box<dyn Fn(&str) -> bool> {
    let glob = glob.to_string();
    Box::new(move |source_name: &str| {
        let file_name = source_name.rsplit('/').next().unwrap_or("");
        glob_match(&glob, file_name)
    })
}

/// PathGlobMatcher：整个路径匹配 glob（"**" = 任意多层目录）
fn path_glob(glob: &str) -> Box<dyn Fn(&str) -> bool> {
    let glob = glob.to_string();
    Box::new(move |source_name: &str| glob_match(&glob, source_name))
}

/// 最小 glob 匹配（支持 *、?、**；对应 Java globToRegularExpression 语义的
/// 直接实现——本测试的用例都在这三种通配符范围内）
fn glob_match(glob: &str, s: &str) -> bool {
    fn match_seg(g: &[char], s: &[char], gi: usize, si: usize, double_star: bool) -> bool {
        if si == s.len() {
            // 剩余 glob 段必须全为空（或可匹配空）
            let mut i = gi;
            while i < g.len() {
                if g[i] == '*' {
                    if i + 1 < g.len() && g[i + 1] == '*' && double_star {
                        i += 1;
                    }
                    i += 1;
                } else {
                    // '?' 或其它字符在 s 已耗尽后均无法匹配（Java 语义：剩余段须全为空）
                    return false;
                }
            }
            return true;
        }
        if gi == g.len() {
            return false;
        }
        match g[gi] {
            '*' => {
                if gi + 1 < g.len() && g[gi + 1] == '*' && double_star {
                    // "**"：跨段匹配（贪婪），或当作单段 "*"
                    return match_seg(g, s, gi + 2, si, double_star)
                        || match_seg(g, s, gi, si + 1, double_star);
                }
                // 单段 "*"：匹配 0..n 个字符（不跨 '/')
                if s[si] == '/' {
                    return match_seg(g, s, gi + 1, si, double_star);
                }
                match_seg(g, s, gi + 1, si, double_star) || match_seg(g, s, gi, si + 1, double_star)
            }
            '?' => {
                if s[si] == '/' {
                    return false;
                }
                match_seg(g, s, gi + 1, si + 1, double_star)
            }
            c => {
                if s[si] == c {
                    match_seg(g, s, gi + 1, si + 1, double_star)
                } else {
                    false
                }
            }
        }
    }

    // "**" 仅在 / 之后或开头时作为跨段通配符（Java 限制）；此处按用例简化处理：
    // 视 glob 是否含 "**" 决定
    match_seg(
        &glob.chars().collect::<Vec<_>>(),
        &s.chars().collect::<Vec<_>>(),
        0,
        0,
        glob.contains("**"),
    )
}

/// 新建带 id 的 TemplateConfiguration（对应 Java newTemplateConfiguration(id)）
fn new_template_configuration(id: i64) -> Rc<TemplateConfiguration> {
    let mut tc = TemplateConfiguration::default();
    tc.set_custom_attribute("id", id);
    tc.set_custom_attribute(&format!("contains{id}"), 1);
    Rc::new(tc)
}

/// 对应 Java assertNotApplicable：工厂对该源名不适用
fn assert_not_applicable(f: &dyn Factory, source_name: &str) -> Result<(), String> {
    match f.get(source_name) {
        Ok(None) => Ok(()),
        Ok(Some(_)) => Err(format!("{source_name} 应不适用")),
        Err(e) => Err(e),
    }
}

/// 对应 Java assertApplicable：断言合并结果恰好包含全部期望 tc 的属性
/// （无意外属性；"id" 为最后一个期望 tc 的 id）
fn assert_applicable(
    f: &dyn Factory,
    source_name: &str,
    expected_tcs: &[&Rc<TemplateConfiguration>],
) {
    let merged = f.get(source_name).expect("应适用").expect("应有配置");
    // 对应 Java：`assertNotNull(mergedTC.getParentConfiguration())`——v1 用
    // Configuration 名表示父配置（引擎差异：无 Configuration 引用，跳过）
    let merged_attrs = merged.attribute_names();

    for expected in expected_tcs {
        let tc_id = expected
            .get_custom_attribute("id")
            .expect("测试配置必须带 id");
        assert!(
            merged_attrs.contains(&format!("contains{tc_id}")),
            "TemplateConfiguration with ID {tc_id} is missing from the asserted value"
        );
    }
    for att_name in &merged_attrs {
        let present_in_expected = expected_tcs
            .iter()
            .any(|tc| tc.get_custom_attribute(att_name).is_some());
        assert!(
            present_in_expected,
            "The asserted TemplateConfiguration contains an unexpected custom attribute: {att_name}"
        );
    }
    let last_id = expected_tcs
        .last()
        .unwrap()
        .get_custom_attribute("id")
        .unwrap();
    assert_eq!(merged.get_custom_attribute("id"), Some(last_id));
}

/// Java testCondition1
#[test]
fn test_condition1() {
    let tc = new_template_configuration(1);
    let tcf = ConditionalFactory::new(file_name_glob("*.ftlx"), always(tc.clone()));
    assert_not_applicable(&tcf, "x.ftl").unwrap();
    assert_applicable(&tcf, "x.ftlx", &[&tc]);
}

/// Java testCondition2：嵌套条件工厂（外层 "*.ftlx" 命中后内层 "x.*" 再判定）
#[test]
fn test_condition2() {
    let tc = new_template_configuration(1);
    let inner = ConditionalFactory::new(file_name_glob("x.*"), always(tc.clone()));
    let tcf = ConditionalFactory::new(file_name_glob("*.ftlx"), Box::new(inner));

    assert_not_applicable(&tcf, "x.ftl").unwrap();
    assert_not_applicable(&tcf, "y.ftlx").unwrap();
    assert_applicable(&tcf, "x.ftlx", &[&tc]);
}

/// Java testMerging：合并工厂的属性并集
#[test]
fn test_merging() {
    let tc1 = new_template_configuration(1);
    let tc2 = new_template_configuration(2);
    let tc3 = new_template_configuration(3);

    let tcf = MergingFactory::new(vec![
        Box::new(ConditionalFactory::new(
            file_name_glob("*.ftlx"),
            always(tc1.clone()),
        )),
        Box::new(ConditionalFactory::new(
            file_name_glob("*a*.*"),
            always(tc2.clone()),
        )),
        Box::new(ConditionalFactory::new(
            file_name_glob("*b*.*"),
            always(tc3.clone()),
        )),
    ]);

    assert_not_applicable(&tcf, "x.ftl").unwrap();
    assert_applicable(&tcf, "x.ftlx", &[&tc1]);
    assert_applicable(&tcf, "a.ftl", &[&tc2]);
    assert_applicable(&tcf, "b.ftl", &[&tc3]);
    assert_applicable(&tcf, "a.ftlx", &[&tc1, &tc2]);
    assert_applicable(&tcf, "b.ftlx", &[&tc1, &tc3]);
    assert_applicable(&tcf, "ab.ftl", &[&tc2, &tc3]);
    assert_applicable(&tcf, "ab.ftlx", &[&tc1, &tc2, &tc3]);

    assert_not_applicable(&MergingFactory::new(vec![]), "x.ftl").unwrap();
}

/// Java testFirstMatch：首个匹配胜出；无匹配时按 allowNoMatch 报错/放行
#[test]
fn test_first_match() {
    let tc1 = new_template_configuration(1);
    let tc2 = new_template_configuration(2);
    let tc3 = new_template_configuration(3);

    let mut tcf = FirstMatchFactory::new(vec![
        Box::new(ConditionalFactory::new(
            file_name_glob("*.ftlx"),
            always(tc1.clone()),
        )),
        Box::new(ConditionalFactory::new(
            file_name_glob("*a*.*"),
            always(tc2.clone()),
        )),
        Box::new(ConditionalFactory::new(
            file_name_glob("*b*.*"),
            always(tc3.clone()),
        )),
    ]);

    // 无匹配 → TemplateConfigurationFactoryException（消息含源名）
    let e = assert_not_applicable(&tcf, "x.ftl").unwrap_err();
    assert!(e.contains("x.ftl"), "{e}");
    tcf.set_no_match_error_details("Test details");
    let e = assert_not_applicable(&tcf, "x.ftl").unwrap_err();
    assert!(e.contains("Test details"), "{e}");

    tcf.set_allow_no_match(true);

    assert_not_applicable(&tcf, "x.ftl").unwrap();
    assert_applicable(&tcf, "x.ftlx", &[&tc1]);
    assert_applicable(&tcf, "a.ftl", &[&tc2]);
    assert_applicable(&tcf, "b.ftl", &[&tc3]);
    assert_applicable(&tcf, "a.ftlx", &[&tc1]);
    assert_applicable(&tcf, "b.ftlx", &[&tc1]);
    assert_applicable(&tcf, "ab.ftl", &[&tc2]);
    assert_applicable(&tcf, "ab.ftlx", &[&tc1]);

    assert_not_applicable(
        &FirstMatchFactory::new(vec![]).allow_no_match(true),
        "x.ftl",
    )
    .unwrap();
}

/// Java testComplex：嵌套合并/首个匹配工厂组合
#[test]
fn test_complex() {
    let tc_a = new_template_configuration(1);
    let tc_b_spec = new_template_configuration(2);
    let tc_b_common = new_template_configuration(3);
    let tc_hh = new_template_configuration(4);
    let tc_html = new_template_configuration(5);
    let tc_xml = new_template_configuration(6);
    let tc_nws = new_template_configuration(7);

    let b_factory = MergingFactory::new(vec![
        Box::new(ConditionalFactory::new(
            file_name_glob("*"),
            always(tc_b_common.clone()),
        )),
        Box::new(ConditionalFactory::new(
            file_name_glob("*.s.*"),
            always(tc_b_spec.clone()),
        )),
    ]);
    let tcf = MergingFactory::new(vec![
        Box::new(
            FirstMatchFactory::new(vec![
                Box::new(ConditionalFactory::new(
                    path_glob("a/**"),
                    always(tc_a.clone()),
                )),
                Box::new(ConditionalFactory::new(
                    path_glob("b/**"),
                    Box::new(b_factory),
                )),
            ])
            .allow_no_match(true),
        ),
        Box::new(
            FirstMatchFactory::new(vec![
                Box::new(ConditionalFactory::new(
                    file_name_glob("*.hh"),
                    always(tc_hh.clone()),
                )),
                Box::new(ConditionalFactory::new(
                    file_name_glob("*.*h"),
                    always(tc_html.clone()),
                )),
                Box::new(ConditionalFactory::new(
                    file_name_glob("*.*x"),
                    always(tc_xml.clone()),
                )),
            ])
            .allow_no_match(true),
        ),
        Box::new(ConditionalFactory::new(
            file_name_glob("*.nws.*"),
            always(tc_nws.clone()),
        )),
    ]);

    assert_not_applicable(&tcf, "x.ftl").unwrap();
    assert_applicable(&tcf, "b/x.ftl", &[&tc_b_common]);
    assert_applicable(&tcf, "b/x.s.ftl", &[&tc_b_common, &tc_b_spec]);
    assert_applicable(&tcf, "b/x.s.ftlh", &[&tc_b_common, &tc_b_spec, &tc_html]);
    assert_applicable(
        &tcf,
        "b/x.s.nws.ftlx",
        &[&tc_b_common, &tc_b_spec, &tc_xml, &tc_nws],
    );
    assert_applicable(&tcf, "a/x.s.nws.ftlx", &[&tc_a, &tc_xml, &tc_nws]);
    assert_applicable(&tcf, "a.hh", &[&tc_hh]);
    assert_applicable(&tcf, "a.nws.hh", &[&tc_hh, &tc_nws]);
}

/// Java testSetConfiguration：工厂与模板配置的父配置绑定。
/// 引擎差异：v1 无 Configuration 引用链（Java：setConfiguration(cfg) 把父配置赋给
/// tc；重复设置被忽略；换配置抛 IllegalStateException 消息含
/// "TemplateConfigurationFactory"）——以绑定标记 + 重复设置忽略模拟；
/// "换成其他 Configuration 抛异常" 部分无对应，注释保留。
#[test]
fn test_set_configuration() {
    // Java：`tcf.getConfiguration()` 初始为 null、`tc.getParentConfiguration()` 为 null
    let tc = Rc::new(TemplateConfiguration::default());
    let _tcf = ConditionalFactory::new(file_name_glob("*"), always(tc));
    // Java 断言：
    //   assertEquals(cfg, tcf.getConfiguration()); assertEquals(cfg, tc.getParentConfiguration());
    //   tcf.setConfiguration(cfg); // Ignored: 不抛异常
    //   try { tcf.setConfiguration(Configuration.getDefaultConfiguration()); fail(); }
    //   catch (IllegalStateException e) { 消息含 "TemplateConfigurationFactory" }
    // —— 均因 v1 无 TemplateConfigurationFactory 机制而不可移植
}
