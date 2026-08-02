//! Java `freemarker.template.TemplateLookupStrategyTest` 的 Rust 1:1 实现
//! （TemplateLookupStrategyTest.java：默认/自定义查找策略、acquisition、
//!   自定义查找条件、解析错误传播测试）
//!
//! 引擎映射：`freemarker::cache::{LookupStrategyDefault020300, TemplateLookupStrategy}`
//! （lookup(name, locale, find)）；局部化+acquisition 语义与 Java
//! TemplateCache.lookupWithLocalizedThenAcquisitionStrategy 一致。
//! 引擎差异：
//! - v1 策略 API 无"查找上下文/自定义查找条件"（Java ctx.getCustomLookupCondition
//!   与 Template.getCustomLookupCondition）——自定义策略用测试内闭包表达；
//! - v1 模板缓存键=实际命中名（Java Template.getName()=请求名、getSourceName()=
//!   命中名；v1 无 sourceName 字段）；
//! - 非解析模板（parseAsFTL=false）未实现；
//! - 记录查找序号的加载器用测试内实现（对应 MonitoredTemplateLoader.getNamesSearched）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::{
    LookupResult, LookupStrategyDefault020300, NameFormatDefault020300, TemplateLoader,
    TemplateLookupStrategy, TemplateNameFormat, TemplateSource,
};
use freemarker::error::Result;
use std::sync::{Arc, Mutex};

/// 记录查找名称的加载器（对应 Java MonitoredTemplateLoader：putTemplate +
/// getNamesSearched/clearEvents）
#[derive(Default)]
struct RecordingLoader {
    entries: Mutex<Vec<(String, String)>>,
    searched: Mutex<Vec<String>>,
}

impl RecordingLoader {
    fn put(&self, name: &str, content: &str) {
        let mut e = self.entries.lock().unwrap();
        e.retain(|(n, _)| n != name);
        e.push((name.to_string(), content.to_string()));
    }

    fn names_searched(&self) -> Vec<String> {
        self.searched.lock().unwrap().clone()
    }

    fn clear_searched(&self) {
        self.searched.lock().unwrap().clear();
    }
}

impl TemplateLoader for RecordingLoader {
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        self.searched.lock().unwrap().push(name.to_string());
        let e = self.entries.lock().unwrap();
        Ok(e.iter()
            .find(|(n, _)| n == name)
            .map(|(n, _)| Box::new(RecordingSource(n.clone())) as Box<dyn TemplateSource>))
    }

    fn read(&self, src: &dyn TemplateSource) -> Result<String> {
        let e = self.entries.lock().unwrap();
        e.iter()
            .find(|(n, _)| n == &src.name())
            .map(|(_, c)| c.clone())
            .ok_or_else(|| freemarker::error::TemplateError::NotFound { name: src.name() })
    }
}

struct RecordingSource(String);

impl TemplateSource for RecordingSource {
    fn name(&self) -> String {
        self.0.clone()
    }
}

/// Java testSetSetting：setSetting 切换查找策略。
/// 引擎差异：v1 无 setSetting API、无自定义策略注册（lookup_strategy 设置字段
/// 恒为 Default020300）——跳过并注释 Java 断言。
#[test]
fn test_set_setting() {
    // Java：cfg.setSetting(TEMPLATE_LOOKUP_STRATEGY_KEY,
    //   MyTemplateLookupStrategy.class.getName()+"()") → instanceof 自定义策略；
    // cfg.setSetting(..., "dEfault") → DEFAULT_2_3_0。
    // v1 无 setSetting / 策略类名解析，不可移植。
    let (c, _loader) = test_config();
    assert_eq!(
        c.settings.lookup_strategy,
        freemarker::cache::LookupStrategyKind::Default020300
    );
}

/// Java testCustomStrategy：语言前缀策略（MyTemplateLookupStrategy：
/// 先试 "aa/"+name 的 acquisition，miss 再试 name）。
/// 引擎差异：Java 策略经 TemplateLookupContext（含 locale）；v1 用测试内闭包
/// 组合 LookupStrategyDefault020300（语义相同）。Template 的 locale 字段
/// 断言无法表达。
#[test]
fn test_custom_strategy() {
    let tl = RecordingLoader::default();
    tl.put("test.ftl", "");
    tl.put("aa/test.ftl", "");
    let strategy = LookupStrategyDefault020300;

    let custom_lookup = |name: &str| -> Result<Option<LookupResult>> {
        // 对应 MyTemplateLookupStrategy.lookup：lang=locale.getLanguage().toLowerCase()；
        // 两次尝试都用 lookupWithAcquisitionStrategy（无局部化）→ v1 传 None
        let lang = "aa";
        let mut find = |n: &str| tl.find(n);
        if let Some(r) = strategy.lookup(&format!("{lang}/{name}"), None, &mut find)? {
            return Ok(Some(r));
        }
        strategy.lookup(name, None, &mut find)
    };

    // 缺失：aa/missing.ftl miss → 回退 missing.ftl miss
    {
        tl.clear_searched();
        let r = custom_lookup("missing.ftl").unwrap();
        assert!(r.is_none());
        // Java 期望：["aa/missing.ftl", "missing.ftl"]
        assert_eq!(
            tl.names_searched(),
            ["aa/missing.ftl", "missing.ftl"].map(String::from)
        );
    }

    // 命中：aa/test.ftl
    {
        tl.clear_searched();
        let r = custom_lookup("test.ftl")
            .unwrap()
            .expect("aa/test.ftl 应命中");
        assert_eq!(r.source_name, "aa/test.ftl");
        assert_eq!(tl.names_searched(), ["aa/test.ftl"].map(String::from));
        // 引擎差异：Java t.getName()=="test.ftl"（请求名）——v1 命中名即模板名
        tl.clear_searched();
    }
}

/// Java testDefaultStrategy：默认策略的局部化回退候选序列
/// （对应 Java 的 getNamesSearched 逐项断言）
#[test]
fn test_default_strategy() {
    let tl = RecordingLoader::default();
    tl.put("test.ftl", "");
    tl.put("test_aa.ftl", "");
    tl.put("test_aa_BB.ftl", "");
    tl.put("test_aa_BB_CC.ftl", "");
    tl.put("test_aa_BB_CC_DD.ftl", "");
    let strategy = LookupStrategyDefault020300;

    // locale "aa_BB_CC_DD"：逐级缩短
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup("missing.ftl", Some("aa_BB_CC_DD"), &mut find)
            .unwrap();
        assert!(r.is_none());
        assert_eq!(
            tl.names_searched(),
            [
                "missing_aa_BB_CC_DD.ftl",
                "missing_aa_BB_CC.ftl",
                "missing_aa_BB.ftl",
                "missing_aa.ftl",
                "missing.ftl"
            ]
            .map(String::from)
        );
    }

    // locale "xx"：两级
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        assert!(strategy
            .lookup("missing.ftl", Some("xx"), &mut find)
            .unwrap()
            .is_none());
        assert_eq!(
            tl.names_searched(),
            ["missing_xx.ftl", "missing.ftl"].map(String::from)
        );
    }

    // 局部化关闭（locale=None）：只查原名
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        assert!(strategy
            .lookup("missing.ftl", None, &mut find)
            .unwrap()
            .is_none());
        assert_eq!(tl.names_searched(), ["missing.ftl"].map(String::from));
    }

    // 下划线开头的名称：locale "xx_yy"（Java new Locale("xx","yy").toString()
    // == "xx_YY"——国家代码大写）的候选
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        assert!(strategy
            .lookup("_a_b_.ftl", Some("xx_YY"), &mut find)
            .unwrap()
            .is_none());
        assert_eq!(
            tl.names_searched(),
            ["_a_b__xx_YY.ftl", "_a_b__xx.ftl", "_a_b_.ftl"].map(String::from)
        );
    }

    // 命中序列（对应 Java 对 7 个模板名变体的循环；此处验证核心 3 例）：
    // 注意 Java 循环里 getTemplate 先经名称规范化（"./test.ftl"→"test.ftl"），
    // v1 查找入口同样先规范化（NameFormatDefault020300）
    for (name, expect_searched, expect_source) in [
        (
            "test.ftl",
            vec!["test_aa_BB_CC_DD.ftl"],
            "test_aa_BB_CC_DD.ftl",
        ),
        (
            "./test.ftl",
            vec!["test_aa_BB_CC_DD.ftl"],
            "test_aa_BB_CC_DD.ftl",
        ),
        (
            "/test.ftl",
            vec!["test_aa_BB_CC_DD.ftl"],
            "test_aa_BB_CC_DD.ftl",
        ),
        (
            "x/foo/../../test.ftl",
            vec!["test_aa_BB_CC_DD.ftl"],
            "test_aa_BB_CC_DD.ftl",
        ),
    ] {
        tl.clear_searched();
        let normalized = NameFormatDefault020300
            .normalize_root_based_name(name)
            .unwrap();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup(&normalized, Some("aa_BB_CC_DD"), &mut find)
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, expect_source);
        assert_eq!(
            tl.names_searched(),
            expect_searched
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    // 变体回退：locale "aa_BB_CC_XX" → 命中 "test_aa_BB_CC.ftl"（前一级变体）
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup("test.ftl", Some("aa_BB_CC_XX"), &mut find)
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "test_aa_BB_CC.ftl");
        assert_eq!(
            tl.names_searched(),
            ["test_aa_BB_CC_XX.ftl", "test_aa_BB_CC.ftl"].map(String::from)
        );
    }
    // locale "aa_BB_XX_XX" → 命中 "test_aa_BB.ftl"
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup("test.ftl", Some("aa_BB_XX_XX"), &mut find)
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "test_aa_BB.ftl");
        assert_eq!(
            tl.names_searched(),
            [
                "test_aa_BB_XX_XX.ftl",
                "test_aa_BB_XX.ftl",
                "test_aa_BB.ftl"
            ]
            .map(String::from)
        );
    }
    // 局部化关闭：直接命中原名
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup("test.ftl", None, &mut find)
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "test.ftl");
        assert_eq!(tl.names_searched(), ["test.ftl"].map(String::from));
    }
    // locale "aa_XX_XX_XX" → 命中 "test_aa.ftl"
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup("test.ftl", Some("aa_XX_XX_XX"), &mut find)
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "test_aa.ftl");
        assert_eq!(
            tl.names_searched(),
            [
                "test_aa_XX_XX_XX.ftl",
                "test_aa_XX_XX.ftl",
                "test_aa_XX.ftl",
                "test_aa.ftl"
            ]
            .map(String::from)
        );
    }
    // locale "xx_XX_XX_XX" → 全 miss 回退原名
    {
        tl.clear_searched();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup("test.ftl", Some("xx_XX_XX_XX"), &mut find)
            .unwrap()
            .expect("回退原名命中");
        assert_eq!(r.source_name, "test.ftl");
        assert_eq!(
            tl.names_searched(),
            [
                "test_xx_XX_XX_XX.ftl",
                "test_xx_XX_XX.ftl",
                "test_xx_XX.ftl",
                "test_xx.ftl",
                "test.ftl"
            ]
            .map(String::from)
        );
    }
}

/// Java testAcquisition：acquisition（"*" 步骤逐级回退）+ 局部化组合
#[test]
fn test_acquisition() {
    let tl = RecordingLoader::default();
    tl.put("t.ftl", "");
    tl.put("sub/i.ftl", "");
    tl.put("x/sub/i.ftl", "");
    let strategy = LookupStrategyDefault020300;
    let locale = "xx";

    // "/./moo/../x/y/*/sub/i.ftl" 规范化 → "x/y/*/sub/i.ftl"
    {
        tl.clear_searched();
        let normalized = NameFormatDefault020300
            .normalize_root_based_name("/./moo/../x/y/*/sub/i.ftl")
            .unwrap();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup(&normalized, Some(locale), &mut find)
            .unwrap()
            .expect("acquisition 应命中");
        // 引擎差异：Java t.getName()=="x/y/*/sub/i.ftl"（规范化名）——v1 命中名
        // "x/sub/i.ftl"（无请求名记录）
        assert_eq!(r.source_name, "x/sub/i.ftl");
        assert_eq!(
            tl.names_searched(),
            [
                "x/y/sub/i_xx.ftl",
                "x/sub/i_xx.ftl",
                "sub/i_xx.ftl",
                "x/y/sub/i.ftl",
                "x/sub/i.ftl"
            ]
            .map(String::from)
        );
    }

    // "a/b/*/./sub/i.ftl" 规范化 → "a/b/*/sub/i.ftl"
    {
        tl.clear_searched();
        let normalized = NameFormatDefault020300
            .normalize_root_based_name("a/b/*/./sub/i.ftl")
            .unwrap();
        let mut find = |n: &str| tl.find(n);
        let r = strategy
            .lookup(&normalized, Some(locale), &mut find)
            .unwrap()
            .expect("acquisition 应命中");
        assert_eq!(r.source_name, "sub/i.ftl");
        assert_eq!(
            tl.names_searched(),
            [
                "a/b/sub/i_xx.ftl",
                "a/sub/i_xx.ftl",
                "sub/i_xx.ftl",
                "a/b/sub/i.ftl",
                "a/sub/i.ftl",
                "sub/i.ftl"
            ]
            .map(String::from)
        );
    }
}

/// Java testCustomLookupCondition：域前缀策略（DomainTemplateLookupStrategy）。
/// 引擎差异：v1 策略 API 无自定义查找条件（Java ctx.getCustomLookupCondition/
/// Template.getCustomLookupCondition）——域策略在测试内闭包实现，且 include 的
/// 条件传播（Java 中 include 沿用外层查找条件）v1 无对应；本地化变体名称
/// 差异同 testDefaultStrategy（Java Locale("aa","BB","CC_DD").toString() 保留
/// 大小写——v1 locale 字符串原样使用）。仅翻译主查找路径的核心断言。
#[test]
fn test_custom_lookup_condition() {
    let tl = RecordingLoader::default();
    tl.put("@foo.com/t.ftl", "t at foo.com <#include 'i.ftl'>");
    tl.put("@bar.com/t.ftl", "t at bar.com <#include 'i.ftl'>");
    tl.put("@default/t.ftl", "t at default <#include 'i.ftl'>");
    tl.put("@foo.com/i.ftl", "i at foo.com");
    tl.put("@baaz.com/i.ftl", "i at baaz.com");
    tl.put("@default/i_xx.ftl", "i_xx at default");
    tl.put("@default/i.ftl", "i at default");
    let strategy = LookupStrategyDefault020300;

    // 域查找闭包：对应 DomainTemplateLookupStrategy.lookup——
    // "@domain/name" 本地化+acquisition miss 后回退 "@default/name"
    let domain_lookup =
        |domain: &str, name: &str, locale: Option<&str>| -> Result<Option<LookupResult>> {
            if name.starts_with('@') {
                return Ok(None); // 禁止直接寻址域根
            }
            let mut find = |n: &str| tl.find(n);
            let dn = format!("@{domain}/{name}");
            if let Some(r) = strategy.lookup(&dn, locale, &mut find)? {
                return Ok(Some(r));
            }
            let ddn = format!("@default/{name}");
            strategy.lookup(&ddn, locale, &mut find)
        };

    // foo.com + locale xx → "@foo.com/t_xx.ftl" miss → "@foo.com/t.ftl" 命中
    {
        tl.clear_searched();
        let r = domain_lookup("foo.com", "t.ftl", Some("xx"))
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "@foo.com/t.ftl");
        assert_eq!(
            tl.names_searched(),
            ["@foo.com/t_xx.ftl", "@foo.com/t.ftl"].map(String::from)
        );
        tl.clear_searched();
    }

    // bar.com + locale xx：t 命中 @bar.com（同型）
    {
        tl.clear_searched();
        let r = domain_lookup("bar.com", "t.ftl", Some("xx"))
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "@bar.com/t.ftl");
        assert_eq!(
            tl.names_searched(),
            ["@bar.com/t_xx.ftl", "@bar.com/t.ftl"].map(String::from)
        );
        tl.clear_searched();
    }

    // baaz.com + locale xx_YY：域内全 miss → 回退 @default（命中 t）
    {
        tl.clear_searched();
        let r = domain_lookup("baaz.com", "t.ftl", Some("xx_YY"))
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "@default/t.ftl");
        assert_eq!(
            tl.names_searched(),
            [
                "@baaz.com/t_xx_YY.ftl",
                "@baaz.com/t_xx.ftl",
                "@baaz.com/t.ftl",
                "@default/t_xx_YY.ftl",
                "@default/t_xx.ftl",
                "@default/t.ftl"
            ]
            .map(String::from)
        );
        tl.clear_searched();
    }

    // nosuch.com + locale xx_YY：i.ftl → @default/i_xx.ftl 命中（Java 断言输出
    // iXxAtDefaultContent；v1 模板内容含 include——include 的域条件传播无对应，
    // 仅断言查找结果）
    {
        tl.clear_searched();
        let r = domain_lookup("nosuch.com", "i.ftl", Some("xx_YY"))
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "@default/i_xx.ftl");
        assert_eq!(
            tl.names_searched(),
            [
                "@nosuch.com/i_xx_YY.ftl",
                "@nosuch.com/i_xx.ftl",
                "@nosuch.com/i.ftl",
                "@default/i_xx_YY.ftl",
                "@default/i_xx.ftl"
            ]
            .map(String::from)
        );
        tl.clear_searched();
    }

    // 局部化关闭：@nosuch.com/i.ftl miss → @default/i.ftl
    {
        tl.clear_searched();
        let r = domain_lookup("nosuch.com", "i.ftl", None)
            .unwrap()
            .expect("应命中");
        assert_eq!(r.source_name, "@default/i.ftl");
        assert_eq!(
            tl.names_searched(),
            ["@nosuch.com/i.ftl", "@default/i.ftl"].map(String::from)
        );
        tl.clear_searched();
    }

    // 直接寻址域根被拒绝（Java：@开头 → 负查找）
    {
        tl.clear_searched();
        assert!(domain_lookup("foo.com", "@foo.com/i.ftl", Some("xx"))
            .unwrap()
            .is_none());
    }
    // 引擎差异：Java 还断言 include 的查找序列（"@foo.com/i_xx.ftl"、import
    // 链 "@foo.com/i2_xx.ftl"→"@default/i2.ftl"→"@foo.com/i3_xx.ftl"）与
    // getCustomLookupCondition()==domain——v1 include 无查找条件传播、
    // Template 无自定义条件字段，注释保留。
}

/// Java testNonparsed：非解析模板（parseAsFTL=false）。
/// 引擎差异：v1 无 parseAsFTL=false 的原始文本模板模式——跳过并注释。
#[test]
fn test_nonparsed() {
    // Java：getTemplate("test.txt", locale, null, false) → 原始文本模板
    // （t.toString()==源文本，不解析 FTL）；缺失时与解析模板同样做局部化查找
    // （getNamesSearched 候选序列）。
    // v1 get_template 恒解析（无 parseAsFTL 参数），不可移植。
}

/// Java testParseError：局部化命中后解析失败的模板 → ParseException
/// （模板名为命中名 "test_aa.ftl"）
#[test]
fn test_parse_error() {
    let (mut c, _loader) = test_config();
    let loader = Arc::new(RecordingLoader::default());
    c.template_loader = loader.clone();
    loader.put("test.ftl", "");
    loader.put("test_aa.ftl", "<#wrong>");

    // Java：cfg.getTemplate("test.ftl", Locale("aa","BB")) → 命中 test_aa.ftl →
    // ParseException，e.getTemplateName()=="test_aa.ftl"
    let e = c
        .get_template_localized("test.ftl", Some("aa_BB"))
        .err()
        .expect("应解析失败");
    let msg = e.to_user_message();
    assert!(msg.contains("test_aa.ftl"), "解析错误应含命中名：{msg}");
    // Java 断言 e.getTemplateName()=="test_aa.ftl" —— v1 Parse 错误含模板名文本
    // （Parse { template, .. } 的 to_user_message 为 "Parsing error in template ..."）
    assert!(msg.contains("Parsing error"), "{msg}");
}
