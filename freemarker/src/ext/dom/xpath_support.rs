//! XPath 支持接口 —— 对应 Java `freemarker.ext.dom.XPathSupport`
//! （XPath 查询的抽象接口；Java 有 Jaxen/Xalan 三个实现，Rust 侧由
//!  `XmlNode::xpath_subset` 方法提供有限子集支持）

/// Java 接口锚点：`XPathSupport`
///
/// Java `XPathSupport` 定义了 `executeQuery(Object context, String xpathQuery)` 方法，
/// 有三个实现：`JaxenXPathSupport`/`XalanXPathSupport`/`SunInternalXalanXPathSupport`。
/// Rust 的等价实现在 `xml/node.rs` 的 `xpath_subset` 方法中（有限 XPath 子集）。
#[allow(dead_code)]
pub(crate) struct XPathSupport;
