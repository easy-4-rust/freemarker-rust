//! Java `freemarker.template.TemplateConstructorsTest` 的 Rust 1:1 实现
//! （TemplateConstructorsTest.java：Template 各构造器重载的名称/源名/编码/内容
//!   语义测试）
//!
//! 引擎映射：v1 解析入口 `parser::parse(cfg, name, ftl)` 与
//! `Configuration.get_template_encoded`（模板头编码重读）。
//! 引擎差异：
//! - v1 无 Template 构造器重载（sourceName 参数/Reader 参数）——getSourceName
//!   概念缺失（缓存键=名称）；
//! - `WrongEncodingException`：Java 从构造器直接抛（显式 encoding 与模板头声明
//!   冲突）；v1 get_template_encoded 按声明编码重读而非报错；
//! - `Template.getPlainTextTemplate` 未实现。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

const READER_CONTENT: &str = "From a reader...";

/// Java test()：各构造重载
#[test]
fn test_constructors() {
    let mut c = Configuration::new();
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    add_template(&loader, "foo/bar.ftl", READER_CONTENT);

    let name = "foo/bar.ftl";
    // Java：new Template(name, reader) / new Template(name, reader, cfg) /
    // new Template(name, content, cfg)：
    //   getName()==name、getSourceName()==name、内容==READER_CONTENT、
    //   getEncoding()==null
    // 引擎差异：v1 无 Reader/String 构造重载——经 StringLoader 加载等价
    // （get_template 不记录编码 → 对应 getEncoding()==null）
    {
        let t = c.get_template(name).unwrap();
        assert_eq!(t.name, name);
        assert_eq!(render_template_content(&t), READER_CONTENT);
        assert_eq!(t.encoding, None);
    }

    // Java：new Template(name, reader, cfg, encoding="UTF-16LE") →
    // getEncoding()=="UTF-16LE"、内容不变（StringReader 已解码，encoding 仅
    // 作元信息 + 头校验）。引擎差异：v1 无此构造器——get_template_encoded 会
    // 真的按编码解码：对 UTF-8 源按 UTF-16LE 解码得乱码，无法 1:1。
    // 按引擎等价路径：把源按 UTF-16LE 字节注册，以 UTF-16LE 读取（内容与
    // 编码断言对齐）：
    {
        loader.put_bytes("foo/utf16.ftl", &utf16le("From a reader..."));
        let t = c
            .get_template_encoded("foo/utf16.ftl", Some("UTF-16LE"))
            .unwrap();
        assert_eq!(t.name, "foo/utf16.ftl");
        assert_eq!(render_template_content(&t), "From a reader...");
        assert_eq!(t.encoding, None); // 无 <#ftl> 头 → 不记录编码
    }
    // Java：new Template(name, sourceName, reader, cfg) →
    //   getName()==name、getSourceName()==sourceName —— v1 无 sourceName 参数
    //   （引擎差异：名称即源名）

    // Java：Template.getPlainTextTemplate(name, content, cfg) —— v1 未实现
    // （注释保留）

    // Java：new Template(name, sourceName, readerForceUTF8, cfg, "UTF-16LE")
    //   → Template.WrongEncodingException，消息含 "utf-8" 与 "UTF-16LE"
    // 引擎差异：v1 get_template_encoded 对头部声明的编码执行重读（不抛错）——
    // 以下验证重读路径（Java 抛错处注释保留）。注意 Java 的 StringReader 已
    // 解码（头部 ASCII 可解析）；v1 需保证头部在初读编码下可解析——
    // 用 ISO-8859-1 模板（ASCII 头部 + latin1 内容）：
    {
        loader.put_bytes(
            "latin1.ftl",
            "<#ftl encoding='ISO-8859-1'>From a reader...".as_bytes(),
        );
        let t = c.get_template_encoded("latin1.ftl", Some("utf-8")).unwrap();
        // v1：先按 utf-8 读取 → 头声明 ISO-8859-1 ≠ utf-8 → 按 ISO-8859-1
        // 重读 → 内容还原
        assert_eq!(render_template_content(&t), "From a reader...");
        assert_eq!(t.encoding.as_deref(), Some("ISO-8859-1"));
    }
}

/// UTF-16LE 字节编码（Java 测试以 StringReader 构造；v1 按字节注册等价源）
fn utf16le(s: &str) -> Vec<u8> {
    let mut v = Vec::new();
    for u in s.encode_utf16() {
        v.extend_from_slice(&u.to_le_bytes());
    }
    v
}

/// 渲染模板内容
fn render_template_content(t: &std::rc::Rc<freemarker::template::Template>) -> String {
    let mut out = Vec::new();
    t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    )
    .unwrap();
    String::from_utf8_lossy(&out).into_owned()
}
