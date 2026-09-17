//! XML 节点模型测试 —— 自 node.rs 拆出（#[cfg(test)] 模块）。

#[cfg(test)]
mod tests {
    use super::super::{parse_xml, XmlNode};
    use crate::core::Environment;
    use crate::template::TModel;
    use crate::template::{Configuration, Template};
    use crate::xml::ns_prefixes::NsPrefixes;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn markup_with_prefixes(doc: &XmlNode, ns_prefixes: HashMap<String, String>) -> String {
        let mut template = Template::new(
            "xml-test.ftl".to_string(),
            Vec::new(),
            HashMap::new(),
            Rc::new(Configuration::new()),
        );
        template.ns_prefixes = ns_prefixes;
        let mut out = Vec::new();
        let mut env = Environment::new(&template, TModel::nothing(), &mut out);
        doc.markup(&mut env)
    }

    #[test]
    fn ns_prefixes_default_ns() {
        let mut m = HashMap::new();
        m.insert("D".to_string(), "http://d".to_string());
        m.insert("n".to_string(), "http://n".to_string());
        let p = NsPrefixes::new(m);
        assert_eq!(p.get_default_ns(), Some("http://d"));
        assert_eq!(p.get_namespace_for_prefix("n"), Some("http://n"));
        assert_eq!(p.get_prefix_for_namespace("http://d"), Some(""));
        assert_eq!(p.get_prefix_for_namespace("http://n"), Some("n"));
        assert_eq!(p.get_prefix_for_namespace(""), Some("N"));
    }

    #[test]
    fn xml_markup_document() {
        let doc = XmlNode::parse("<root xmlns:n=\"http://x\"><a><b><c xmlns=\"http://x\">C&lt;&gt;&amp;\"']]&gt;</c></b></a></root>").unwrap();
        assert_eq!(
            markup_with_prefixes(&doc, HashMap::new()),
            "<root xmlns:a=\"http://x\"><a><b><a:c>C&lt;>&amp;\"']]&gt;</a:c></b></a></root>"
        );
    }

    #[test]
    fn xml_markup_default_ns() {
        let doc = XmlNode::parse(
            "<eb:book xmlns:eb=\"http://example.com/eBook\">\n  <eb:title>Test Book</eb:title>\n</eb:book>",
        )
        .unwrap();
        let mut m = HashMap::new();
        m.insert("D".to_string(), "http://example.com/eBook".to_string());
        assert_eq!(
            markup_with_prefixes(&doc, m),
            "<book xmlns=\"http://example.com/eBook\">\n  <title>Test Book</title>\n</book>"
        );
    }

    #[test]
    fn node_list_model_keeps_hash_navigation_role() {
        let doc =
            parse_xml("<root><a><b><c>one</c></b></a><a><b><c>two</c></b></a></root>").unwrap();
        let template = Template::new(
            "xml-test.ftl".to_string(),
            Vec::new(),
            HashMap::new(),
            Rc::new(Configuration::new()),
        );
        let mut out = Vec::new();
        let mut env = Environment::new(&template, TModel::nothing(), &mut out);

        let root = doc
            .node_hash
            .as_ref()
            .unwrap()
            .get(&mut env, "root")
            .unwrap()
            .unwrap();
        let a = root
            .node_hash
            .as_ref()
            .unwrap()
            .get(&mut env, "a")
            .unwrap()
            .unwrap();
        assert_eq!(a.sequence.as_ref().unwrap().size().unwrap(), 2);
        assert!(a.node_hash.is_some());

        let b = a
            .node_hash
            .as_ref()
            .unwrap()
            .get(&mut env, "b")
            .unwrap()
            .unwrap();
        assert_eq!(b.sequence.as_ref().unwrap().size().unwrap(), 2);
        let c = b
            .node_hash
            .as_ref()
            .unwrap()
            .get(&mut env, "c")
            .unwrap()
            .unwrap();
        assert_eq!(c.sequence.as_ref().unwrap().size().unwrap(), 2);

        let missing = a
            .node_hash
            .as_ref()
            .unwrap()
            .get(&mut env, "missing")
            .unwrap()
            .unwrap()
            .node_hash
            .as_ref()
            .unwrap()
            .get(&mut env, "child")
            .unwrap()
            .unwrap();
        assert_eq!(missing.sequence.as_ref().unwrap().size().unwrap(), 0);
    }
}

#[cfg(test)]
mod sibling_tests {
    use super::super::XmlNode;
    use crate::template::TemplateNodeModel;

    /// ?next_sibling / ?previous_sibling（BuiltInsForNodes）：兄弟链不含注释/PI
    #[test]
    fn node_siblings() {
        let doc = XmlNode::parse("<root><a/>text<b/><!--c--><c/></root>").unwrap();
        // root 元素（doc 是文档节点）的 children：[a, text, b, c]（注释被过滤）
        let root_el = doc.node().children().next().unwrap();
        let children = root_el.children().collect::<Vec<_>>();
        let a = children
            .iter()
            .find(|n| n.tag_name().name() == "a")
            .unwrap();
        let b = children
            .iter()
            .find(|n| n.tag_name().name() == "b")
            .unwrap();
        let c = children
            .iter()
            .find(|n| n.tag_name().name() == "c")
            .unwrap();
        let a_node = XmlNode {
            tree: doc.tree.clone(),
            node_id: a.id(),
            attr: None,
            doctype: false,
        };
        let b_node = XmlNode {
            tree: doc.tree.clone(),
            node_id: b.id(),
            attr: None,
            doctype: false,
        };
        let c_node = XmlNode {
            tree: doc.tree.clone(),
            node_id: c.id(),
            attr: None,
            doctype: false,
        };

        // a 的下一个兄弟 = text 节点（不是 b；b 是 text 之后）
        let nxt = a_node.next_sibling().unwrap().unwrap();
        assert_eq!(nxt.node.as_ref().unwrap().node_type().unwrap(), "text");
        // b 的上一兄弟 = text
        let prev = b_node.previous_sibling().unwrap().unwrap();
        assert_eq!(prev.node.as_ref().unwrap().node_type().unwrap(), "text");
        // b 的下一兄弟 = c（注释被跳过）
        let nxt = b_node.next_sibling().unwrap().unwrap();
        assert_eq!(nxt.node.as_ref().unwrap().node_type().unwrap(), "element");
        assert_eq!(nxt.node.as_ref().unwrap().name().unwrap().unwrap(), "c");
        // c 无下一兄弟
        assert!(c_node.next_sibling().unwrap().is_none());
        // a 无上一兄弟
        assert!(a_node.previous_sibling().unwrap().is_none());
    }
}
