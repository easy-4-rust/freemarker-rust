//! XML 节点模型深度覆盖 —— VALUE_ADD 测试（Java DOMTest 的模板侧语义；
//! 覆盖 xml/node.rs 的哈希键访问/XPath 子集/节点内建各分支）
//!
//! Java 语义锚点：freemarker.ext.dom.NodeModel 的 get(key) 各分支
//! （子元素名 / @attr / @@key / * / ** / //name / [n] / ?children 等）

/// 渲染配置（对齐 java_ported/util.rs 的 test_config）
fn test_config() -> (
    freemarker::template::Configuration,
    std::sync::Arc<freemarker::cache::StringLoader>,
) {
    let mut c = freemarker::template::Configuration::new();
    c.settings.locale = "en_US".to_string();
    c.settings.time_zone = "Etc/GMT-1"
        .parse()
        .unwrap_or(freemarker::core::TzSetting::Fixed(
            chrono::FixedOffset::east_opt(0).unwrap(),
        ));
    c.settings.time_zone_id = "GMT+01:00".to_string();
    let loader = std::sync::Arc::new(freemarker::cache::StringLoader::default());
    c.template_loader = loader.clone();
    (c, loader)
}

fn doc_node(xml: &str) -> freemarker::template::TModel {
    freemarker::xml::parse_xml(xml).unwrap()
}

/// 渲染 `${expr}`（数据模型注入 node 变量）
/// 渲染模板体（Java DOMTest 全部声明 ns_prefixes；默认命名空间元素需
/// 前缀注册后才能无前缀访问——Java NodeModel.matchesName 语义）
fn render_body(xml: &str, body: &str) -> String {
    let (c, _loader) = test_config();
    let cfg = std::rc::Rc::new(c.clone());
    let ftl = "<#ftl ns_prefixes = {\"D\" : \"http://example.com/book\"}>".to_string() + body;
    let t = freemarker::parser::parse(&cfg, "adhoc", &ftl)
        .unwrap_or_else(|e| panic!("parse failed: {e}"));
    let mut root = indexmap::IndexMap::new();
    root.insert("node".to_string(), doc_node(xml));
    let mut out = Vec::new();
    t.process(freemarker::template::TModel::from_hash(root), &mut out)
        .unwrap_or_else(|e| panic!("process failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}

fn render_expr(xml: &str, expr: &str) -> String {
    render_body(xml, &format!("${{{expr}}}"))
}

const BOOK_XML: &str = r#"<?xml version="1.0"?>
<book xmlns="http://example.com/book" category="fiction" price="29.95">
  <title lang="en">Everyday Italian</title>
  <author>Giada De Laurentiis</author>
  <chapter id="c1">
    <title>Chapter 1</title>
    <para>First <b>bold</b> paragraph.</para>
    <para>Second paragraph.</para>
  </chapter>
  <chapter id="c2"><title>Chapter 2</title></chapter>
</book>"#;

/// 子元素名访问（Java NodeModel.get("title") → NodeListModel 序列；
/// 同名多元素全匹配，单个取 [0]）
#[test]
fn child_element_by_name() {
    assert_eq!(
        render_expr(BOOK_XML, "node.book.title[0]?node_name"),
        "title"
    );
    assert_eq!(
        render_expr(BOOK_XML, "node.book.chapter[1]?node_name"),
        "chapter"
    );
    assert_eq!(render_expr(BOOK_XML, "node.book.title?node_name"), "title");
}

/// @attr 属性访问（Java NodeModel.get("@category")）
#[test]
fn attribute_access() {
    assert_eq!(render_expr(BOOK_XML, r#"node.book.@category"#), "fiction");
    assert_eq!(render_expr(BOOK_XML, r#"node.book.@price"#), "29.95");
    assert_eq!(render_expr(BOOK_XML, r#"node.book.title[0].@lang"#), "en");
}

/// @@key 特殊键（Java AtAtKey：@@text/@@markup 等；@@nodeName 为扩展键——
/// 返回节点名（元素标签名），与 ?node_name 一致）
#[test]
fn at_at_key() {
    // @@nodeName：节点名（元素标签名），与 ?node_name 一致
    assert_eq!(render_expr(BOOK_XML, "node.book.@@nodeName"), "book");
    // @@nodeName 的文档/文本节点等价（?node_name 语义：@document/@text）
    assert_eq!(render_expr(BOOK_XML, "node.@@nodeName"), "@document");
    assert_eq!(
        render_expr(BOOK_XML, "node.book.author.@@nodeName"),
        "author"
    );
    // @@text：元素文本内容拼接
    assert_eq!(
        render_expr(BOOK_XML, "node.book.author.@@text"),
        "Giada De Laurentiis"
    );
}

/// * 所有子元素 / ** 全部后代
#[test]
fn wildcard_children() {
    assert_eq!(render_expr(BOOK_XML, "node.book.*?size"), "4");
    assert!(
        render_expr(BOOK_XML, "node.book.**?size")
            .parse::<usize>()
            .unwrap()
            > 5
    );
}

/// XPath 子集 //name 后代查找（Java Jaxen：无前缀名经
/// translateNamespacePrefixToUri("") 解析为模板默认命名空间
/// （JaxenXPathSupport.customNamespaceContext + Template.getNamespaceForPrefix("")）
/// → 元素命名空间 == 默认命名空间时匹配；同名多元素返回序列）
#[test]
fn xpath_descendant() {
    // BOOK_XML 元素在默认命名空间 http://example.com/book（ns_prefixes 声明 D）
    assert_eq!(
        render_expr(BOOK_XML, r#"node['//title'][0]?node_name"#),
        "title"
    );
    // 单个匹配返回节点（Java NodeListModel 单元素语义；author 全文档唯一）
    assert_eq!(
        render_expr(BOOK_XML, r#"node['//author']?node_name"#),
        "author"
    );
    // 多匹配 → 序列；`//para` 在默认命名空间下同样命中
    assert_eq!(render_expr(BOOK_XML, r#"node['//para']?size"#), "2");
    // 无命名空间元素在模板声明默认命名空间时不匹配无前缀名（Java matchesName：
    // 无 ns 元素须用 N: 前缀，DomStringUtil.java:79-84）
    let plain = r#"<book><title>Everyday Italian</title></book>"#;
    assert_eq!(render_expr(plain, r#"node['//title']?size"#), "0");
    // `//D:title` 显式前缀同样命中默认命名空间元素
    assert_eq!(
        render_expr(BOOK_XML, r#"node['//D:title'][0]?node_name"#),
        "title"
    );
}

/// 数字键 [n] 索引（Java DynamicKeyName 数字键 → 序列下标）
#[test]
fn numeric_index() {
    assert_eq!(render_expr(BOOK_XML, "node.book.chapter[0].@id"), "c1");
    assert_eq!(render_expr(BOOK_XML, "node.book.chapter[1].@id"), "c2");
}

/// 节点内建（Java ?children/?parent/?root/?node_name/?node_type/?node_namespace）
#[test]
fn node_builtins() {
    // ?children：Java getChildNodes 全部子节点（4 元素 + 5 空白文本）
    assert_eq!(render_expr(BOOK_XML, "node.book?children?size"), "9");
    // ?parent
    assert_eq!(
        render_expr(BOOK_XML, "node.book.title[0]?parent?node_name"),
        "book"
    );
    // ?root
    assert_eq!(
        render_expr(BOOK_XML, "node.book.chapter[0]?root?node_name"),
        "@document"
    );
    // ?node_type：文档 = "document"、元素 = "element"
    assert_eq!(render_expr(BOOK_XML, "node?node_type"), "document");
    assert_eq!(render_expr(BOOK_XML, "node.book?node_type"), "element");
    // ?node_namespace：带命名空间元素返回 URI
    assert_eq!(
        render_expr(BOOK_XML, "node.book?node_namespace"),
        "http://example.com/book"
    );
}

/// 命名空间前缀访问（Java NodeModel.get("n:title")——ns_prefixes 解析）
#[test]
fn namespaced_child() {
    let xml = r#"<r xmlns:n="http://y"><n:c>N</n:c><d>D</d></r>"#;
    // n: 前缀注册后 'n:c' 可访问（Java 前缀反查语义；模板声明 n 前缀）
    let (c, _loader) = test_config();
    let cfg = std::rc::Rc::new(c.clone());
    let ftl =
        "<#ftl ns_prefixes = {\"n\" : \"http://y\"}><#assign n2 = node.r['n:c']>${n2?node_name}";
    let t = freemarker::parser::parse(&cfg, "adhoc", ftl).unwrap();
    let mut root = indexmap::IndexMap::new();
    root.insert("node".to_string(), doc_node(xml));
    let mut out = Vec::new();
    t.process(freemarker::template::TModel::from_hash(root), &mut out)
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out), "c");
}

/// @@markup 序列化输出（Java NodeOutputter）
#[test]
fn markup_output() {
    let s = render_expr(BOOK_XML, "node.book.chapter[0].@@markup");
    assert!(s.contains("<chapter id=\"c1\">"), "{s}");
    assert!(s.contains("First"), "{s}");
}

/// 文本节点标量（Java TextModel：@@text 输出文本内容）
#[test]
fn text_node() {
    assert_eq!(
        render_expr(BOOK_XML, "node.book.title[0].@@text"),
        "Everyday Italian"
    );
}

/// 属性值字符串（Java AttrValue：@attr 值可直接输出/比较；
/// 布尔输出经 ?string 显式格式化）
#[test]
fn attr_string_value() {
    assert_eq!(
        render_expr(
            BOOK_XML,
            r#"(node.book.@category == 'fiction')?string('yes','no')"#
        ),
        "yes"
    );
    assert_eq!(
        render_expr(
            BOOK_XML,
            r#"(node.book.@category == 'nonfiction')?string('yes','no')"#
        ),
        "no"
    );
}
