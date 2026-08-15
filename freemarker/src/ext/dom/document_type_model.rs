//! DocumentTypeModel —— 对应 Java `freemarker.ext.dom.DocumentTypeModel`
//! （DOCTYPE 节点模型；Rust 降级真实现：roxmltree 无 doctype API，自扫源文本注入）
//!
//! 行为对齐 Java DocumentTypeModel：
//! - `getNodeName()` = `@document_type$` + name
//! - `getChildren()` 抛 "entering the child nodes of a DTD node is not currently supported"
//! - `get(key)` 抛 "accessing properties of a DTD is not currently supported"
//! - `isEmpty()` = true
//! - `getAsString()` 返回原始声明串（Java 的 ProcessingInstruction 误转型怪癖——
//!   按 DOM 真实语义降级为返回原始声明串并注释说明）

/// DOCTYPE 节点模型（Rust 降级真实现）
///
/// 实际语义由 `XmlNode.doctype` 标志 + `XmlTree.doctype` 字段承载。
/// 本文件提供文档与测试。
///
/// # 实现细节
///
/// roxmltree 0.21.1 无任何 doctype API，因此：
/// 1. `XmlTree::parse` 前/后自扫源文本，正则扫描 `<!DOCTYPE name ...>` 声明
/// 2. 提取 name 和原始声明串，存入 `XmlTree.doctype: Option<DoctypeInfo>`
/// 3. `XmlNode` 增加 `doctype: bool` 标志
/// 4. 文档节点的 `child_nodes()` 在 doctype 存在时按 DOM 顺序注入该节点
#[allow(dead_code)]
pub(crate) struct DocumentTypeModel;

#[cfg(test)]
mod tests {
    use crate::template::TemplateNodeModel;
    use crate::xml::XmlNode;

    #[test]
    fn doctype_html_detected() {
        let doc = XmlNode::parse("<!DOCTYPE html><html><body>Hello</body></html>").unwrap();
        // 文档节点的 children 应包含 doctype 节点
        let kids = doc.children().unwrap();
        assert!(
            kids.len() >= 2,
            "expected at least doctype + html, got {}",
            kids.len()
        );
        // 第一个子节点应是 doctype
        let doctype_node = &kids[0];
        assert_eq!(
            doctype_node.node.as_ref().unwrap().name().unwrap().unwrap(),
            "@document_type$html"
        );
        assert_eq!(
            doctype_node.node.as_ref().unwrap().node_type().unwrap(),
            "document_type"
        );
    }

    #[test]
    fn doctype_system_detected() {
        let doc = XmlNode::parse(r#"<!DOCTYPE note SYSTEM "x.dtd"><note><body>Test</body></note>"#)
            .unwrap();
        let kids = doc.children().unwrap();
        assert!(kids.len() >= 2);
        let doctype_node = &kids[0];
        assert_eq!(
            doctype_node.node.as_ref().unwrap().name().unwrap().unwrap(),
            "@document_type$note"
        );
    }

    #[test]
    fn doctype_scalar_value_is_raw_declaration() {
        let doc = XmlNode::parse("<!DOCTYPE html><html></html>").unwrap();
        let kids = doc.children().unwrap();
        let doctype_node = &kids[0];
        let scalar = doctype_node.scalar.as_ref().unwrap().as_string().unwrap();
        assert_eq!(scalar, "<!DOCTYPE html>");
    }

    #[test]
    fn doctype_children_throws() {
        let doc = XmlNode::parse("<!DOCTYPE html><html></html>").unwrap();
        let kids = doc.children().unwrap();
        let doctype_node = &kids[0];
        let result = doctype_node.node.as_ref().unwrap().children();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("entering the child nodes of a DTD node is not currently supported"));
    }

    #[test]
    fn doctype_hash_get_throws() {
        use crate::core::Environment;
        use crate::template::{Configuration, TModel, Template};
        use std::collections::HashMap;
        use std::rc::Rc;

        let doc = XmlNode::parse("<!DOCTYPE html><html></html>").unwrap();
        let kids = doc.children().unwrap();
        let doctype_node = &kids[0];
        let cfg = Rc::new(Configuration::new());
        let template = Template::new("test.ftl".to_string(), Vec::new(), HashMap::new(), cfg);
        let mut out = Vec::new();
        let mut env = Environment::new(&template, TModel::nothing(), &mut out);
        // 通过 NodeHashModel trait 调用 get
        let hash = doctype_node.node_hash.as_ref().unwrap();
        let result = hash.get(&mut env, "some_key");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("accessing properties of a DTD is not currently supported"));
    }

    #[test]
    fn no_doctype_behavior_unchanged() {
        let doc = XmlNode::parse("<root><child/></root>").unwrap();
        let kids = doc.children().unwrap();
        // 无 doctype 时，第一个子节点应是根元素
        assert_eq!(kids.len(), 1);
        assert_eq!(
            kids[0].node.as_ref().unwrap().name().unwrap().unwrap(),
            "root"
        );
        assert_eq!(
            kids[0].node.as_ref().unwrap().node_type().unwrap(),
            "element"
        );
    }

    #[test]
    fn doctype_is_first_child_before_element() {
        let doc = XmlNode::parse("<!DOCTYPE html><html><body/></html>").unwrap();
        let kids = doc.children().unwrap();
        // 子节点顺序：doctype, html
        assert_eq!(kids.len(), 2);
        assert_eq!(
            kids[0].node.as_ref().unwrap().node_type().unwrap(),
            "document_type"
        );
        assert_eq!(
            kids[1].node.as_ref().unwrap().node_type().unwrap(),
            "element"
        );
    }
}
