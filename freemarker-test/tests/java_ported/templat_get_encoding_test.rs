//! Java `freemarker.core.TemplatGetEncodingTest` 的 Rust 1:1 实现
//! （对应 Java: TemplatGetEncodingTest —— Template.getEncoding() 的默认编码/
//!   显式编码加载与缓存语义）。
//!
//! 引擎差异总览（按引擎 API 翻译，见 Java TemplatGetEncodingTest）：
//! - Java `Template.getEncoding()` 返回实际用于读取模板的字符集（默认编码或
//!   getTemplate(name, encoding) 的显式编码）；本引擎 `Template.encoding` 字段只
//!   记录 `<#ftl encoding=...>` 头部声明的编码（parser/grammar.rs:56、345），
//!   非头部声明的读取编码不写入 → 下述 getEncoding() 断言按引擎实际值 None 断言。
//! - Java TemplateCache 按 `TemplateKey{name, locale, encoding, ...}` 分键缓存，
//!   `getTemplate("t","UTF-8")` 与 `getTemplate("t","ISO-8859-2")` 是不同缓存条目且
//!   同键命中返回同一实例；本引擎 `get_template_encoded` 不走模板缓存
//!   （configuration.rs:118 "v1 不走模板缓存"），`get_template` 仅按规范化名称缓存。
//! - Java `getTemplate(name, null, null, parseAsFTL=false)` 的 plainText 加载模式
//!   引擎无对应 API → NOT_APPLICABLE。
//! - Java `Template.getPlainTextTemplate` 引擎无 → NOT_APPLICABLE。
//!
//! NOT_APPLICABLE: test 方法中依赖 parseAsFTL=false / getPlainTextTemplate /
//!   per-encoding 缓存的片段（见上）；`new Template(null, "test", cfg)` 的 encoding
//!   == null 断言等价翻译（引擎 parse 无头声明时 encoding=None）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use std::rc::Rc;

/// Java test —— Template.getEncoding() + 模板缓存（引擎 API 等价翻译）
#[test]
fn test() {
    let (mut c, l) = test_config();
    // Java：cfg.setDefaultEncoding("ISO-8859-2") —— 引擎 Settings.input_encoding
    c.settings.input_encoding = Some("ISO-8859-2".to_string());
    add_template(&l, "t", "test");
    add_template(&l, "tnp", "<#test>");

    {
        let t_def_enc = c.get_template("t").unwrap();
        // Java：assertEquals("ISO-8859-2", tDefEnc.getEncoding())
        // 引擎差异：encoding 字段只记录 <#ftl encoding> 头部声明，读取编码不写入 → None
        assert_eq!(
            t_def_enc.encoding, None,
            "Java 期望 \"ISO-8859-2\"（读取编码），引擎仅记录头部声明编码"
        );
        // Java：assertSame(tDefEnc, cfg.getTemplate("t")) —— 引擎 get_template 按名缓存
        assert!(
            Rc::ptr_eq(&t_def_enc, &c.get_template("t").unwrap()),
            "get_template(\"t\") 缓存应返回同一实例"
        );

        // Java：getTemplate("t", (String) null) 同默认编码，assertSame
        let t_def_enc2 = c.get_template_encoded("t", None).unwrap();
        assert_eq!(t_def_enc2.encoding, None, "Java 期望 \"ISO-8859-2\"");
        // Java assertSame(tDefEnc, tDefEnc2)：引擎 get_template_encoded 不走缓存，
        // 返回新实例（引擎差异）
        assert!(
            !Rc::ptr_eq(&t_def_enc, &t_def_enc2),
            "引擎差异：Java 按 (name, encoding) 键缓存 → assertSame；引擎 get_template_encoded 不走缓存"
        );

        let t_utf8 = c.get_template_encoded("t", Some("UTF-8")).unwrap();
        assert_eq!(t_utf8.encoding, None, "Java 期望 \"UTF-8\"");
        // Java assertSame(tUTF8, getTemplate("t","UTF-8"))：引擎无 per-encoding 缓存 → 新实例
        assert!(
            !Rc::ptr_eq(&t_utf8, &c.get_template_encoded("t", Some("UTF-8")).unwrap()),
            "引擎差异：Java 按 (name, encoding) 键缓存 → assertSame；引擎 get_template_encoded 不走缓存"
        );
        // Java assertNotSame(tDefEnc, tUTF8)：引擎同样返回不同实例 ✓
        assert!(
            !Rc::ptr_eq(&t_def_enc, &t_utf8),
            "默认编码与显式 UTF-8 模板应是不同实例"
        );
    }

    {
        // Java：getTemplate("tnp", null, null, parseAsFTL=false) 以纯文本加载
        // （"<#test>" 不可解析为 FTL）；引擎无 parseAsFTL=false 模式 → NOT_APPLICABLE。
        // 等价引擎行为：get_template 解析失败。
        let err = c.get_template("tnp").err().unwrap();
        assert!(
            err.to_user_message().contains("test"),
            "解析 <#test> 应失败"
        );
    }

    {
        // Java：new Template(null, "test", cfg) → getEncoding() == null。
        // 等价引擎：parser::parse（无 <#ftl encoding> 头部）→ encoding None
        let cfg = Rc::new(c.clone());
        let non_stored_t = freemarker::parser::parse(&cfg, "adhoc", "test").unwrap();
        assert_eq!(non_stored_t.encoding, None);
    }

    {
        // Java：Template.getPlainTextTemplate(null, "<#test>", cfg) → getEncoding() == null。
        // 引擎无 getPlainTextTemplate API → NOT_APPLICABLE（Java 语义：纯文本模板
        // 不经编码读取，encoding 恒 null）。
    }
}
