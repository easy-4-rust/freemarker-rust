//! XML 节点模型 —— 对应 Java `freemarker.ext.dom.NodeModel`（roxmltree 只读 DOM）

use crate::core::Environment;
use crate::error::{Result, TemplateError};
use crate::template::{ModelKind, NodeHashModel, TModel, TemplateNodeModel};
use roxmltree::{Node, NodeId};
use std::rc::Rc;

use super::ns_prefixes::NsPrefixes;
use super::tree::XmlTree;

/// XML 节点 —— 对应 Java `NodeModel`（wrap 后的 W3C DOM 节点）。
/// roxmltree 的 `Node` 是 Copy 的轻量句柄（借树）；`node_id` + `Rc<XmlTree>` 复现
/// 节点引用（Rust 无法直接持有带生命周期 Node 的 Self 引用结构）。
#[derive(Clone)]
pub struct XmlNode {
    tree: Rc<XmlTree>,
    node_id: NodeId,
    /// Some(属性名)：包装的是 node_id 元素上的属性节点（Java Attr 节点）
    attr: Option<String>,
    /// true: this node represents a DocumentType node
    doctype: bool,
}

impl XmlNode {
    /// 文档节点（Java NodeModel.wrap(Document)；parse 入口）
    pub fn parse(s: &str) -> Result<XmlNode> {
        let tree = XmlTree::parse(s)?;
        Ok(XmlNode {
            node_id: tree.doc.root().id(),
            tree,
            attr: None,
            doctype: false,
        })
    }

    /// 取 roxmltree 节点（借用 &self → 树必然存活）
    fn node(&self) -> Node<'_, 'static> {
        // get_node 对根（文档节点）返回 root；NodeId 0 = root
        self.tree
            .doc
            .get_node(self.node_id)
            .expect("XmlNode 的 node_id 必然有效")
    }

    fn is_attr(&self) -> bool {
        self.attr.is_some()
    }

    /// 元素/文档节点的属性迭代（xmlns:* 声明不算属性）
    fn attributes(&self) -> Vec<roxmltree::Attribute<'_, 'static>> {
        let n = self.node();
        if !n.is_element() {
            return Vec::new();
        }
        n.attributes()
            .filter(|a| !a.name().starts_with("xmlns"))
            .collect()
    }

    /// 子节点（Java simplify 后：注释/PI 已移除；text 与元素保留；
    ///  文档节点含 doctype 时按 DOM 顺序注入 DocumentType 节点）
    fn child_nodes(&self) -> Vec<XmlNode> {
        let mut out = Vec::new();
        // 文档节点且有 doctype 时，在第一个元素子节点前注入 DocumentType 节点
        let is_root = self.node().node_type() == roxmltree::NodeType::Root;
        let mut doctype_injected = false;
        for c in self.node().children() {
            if matches!(
                c.node_type(),
                roxmltree::NodeType::Comment | roxmltree::NodeType::PI
            ) {
                continue;
            }
            // 在第一个元素子节点前注入 doctype 节点（DOM 顺序：doctype 在元素前）
            if is_root && !doctype_injected && c.is_element() && self.tree.doctype.is_some() {
                out.push(XmlNode {
                    tree: self.tree.clone(),
                    node_id: self.node_id, // 复用根节点 id（doctype 不是真实 DOM 节点）
                    attr: None,
                    doctype: true,
                });
                doctype_injected = true;
            }
            out.push(XmlNode {
                tree: self.tree.clone(),
                node_id: c.id(),
                attr: None,
                doctype: false,
            });
        }
        // 若没有元素子节点但有 doctype（如空文档），仍注入
        if is_root && !doctype_injected && self.tree.doctype.is_some() {
            out.push(XmlNode {
                tree: self.tree.clone(),
                node_id: self.node_id,
                attr: None,
                doctype: true,
            });
        }
        out
    }

    /// 下一兄弟节点（Java getNextSibling：simplify 后兄弟链不含注释/PI；
    /// 属性节点无兄弟）
    fn next_sibling_node(&self) -> Option<XmlNode> {
        if self.is_attr() {
            return None;
        }
        let mut seen_self = false;
        for c in self.node().parent()?.children() {
            if c.id() == self.node_id {
                seen_self = true;
                continue;
            }
            if seen_self
                && !matches!(
                    c.node_type(),
                    roxmltree::NodeType::Comment | roxmltree::NodeType::PI
                )
            {
                return Some(XmlNode {
                    tree: self.tree.clone(),
                    node_id: c.id(),
                    attr: None,
                    doctype: false,
                });
            }
        }
        None
    }

    /// 上一兄弟节点（Java getPreviousSibling）
    fn previous_sibling_node(&self) -> Option<XmlNode> {
        if self.is_attr() {
            return None;
        }
        let mut prev = None;
        for c in self.node().parent()?.children() {
            if c.id() == self.node_id {
                return prev;
            }
            if !matches!(
                c.node_type(),
                roxmltree::NodeType::Comment | roxmltree::NodeType::PI
            ) {
                prev = Some(XmlNode {
                    tree: self.tree.clone(),
                    node_id: c.id(),
                    attr: None,
                    doctype: false,
                });
            }
        }
        None
    }

    /// 子元素（Java NodeListModel 按名称过滤时只考虑元素）
    fn child_elements(&self) -> Vec<XmlNode> {
        self.child_nodes()
            .into_iter()
            .filter(|c| c.node().is_element())
            .collect()
    }

    /// 全部后代元素（文档序；Java `**` / XPath `//*`）
    fn descendant_elements(&self) -> Vec<XmlNode> {
        let mut out = Vec::new();
        for d in self.node().descendants() {
            if d.is_element() {
                out.push(XmlNode {
                    tree: self.tree.clone(),
                    node_id: d.id(),
                    attr: None,
                    doctype: false,
                });
            }
        }
        out
    }

    /// 元素本地名 / 文本 / 属性名（Java getNodeName 的各实现）
    fn node_name(&self) -> Option<String> {
        if self.doctype {
            // Java DocumentTypeModel.getNodeName："@document_type$" + name
            let name = self.tree.doctype.as_ref()?.name.clone();
            return Some(format!("@document_type${name}"));
        }
        if let Some(an) = &self.attr {
            // Java AttributeNodeModel.getNodeName：localName
            return Some(local_part(an));
        }
        let n = self.node();
        match n.node_type() {
            roxmltree::NodeType::Element => {
                // Java ElementModel.getNodeName：getLocalName()
                Some(n.tag_name().name().to_string())
            }
            roxmltree::NodeType::Root => Some("@document".to_string()),
            roxmltree::NodeType::Text => Some("@text".to_string()),
            roxmltree::NodeType::Comment => Some("@comment".to_string()),
            roxmltree::NodeType::PI => {
                // Java PINodeModel.getNodeName："@pi$" + target
                Some(format!("@pi${}", n.pi().map(|p| p.target).unwrap_or("")))
            }
        }
    }

    /// 节点类型（Java NodeModel.getNodeType：CDATA 也算 "text"；PI = "pi"；
    ///  doctype = "document_type"）
    fn node_type(&self) -> String {
        if self.doctype {
            return "document_type".to_string();
        }
        if self.is_attr() {
            return "attribute".to_string();
        }
        match self.node().node_type() {
            roxmltree::NodeType::Element => "element".to_string(),
            roxmltree::NodeType::Root => "document".to_string(),
            roxmltree::NodeType::Text => "text".to_string(),
            roxmltree::NodeType::Comment => "comment".to_string(),
            roxmltree::NodeType::PI => "pi".to_string(),
        }
    }

    /// 节点命名空间（Java NodeModel.getNodeNamespace：元素无命名空间 → ""；
    /// 属性 → URI 或 null；其余节点 → null）
    fn node_namespace(&self) -> Option<String> {
        if self.is_attr() {
            let n = self.node();
            if n.is_element() {
                for a in n.attributes() {
                    if local_part(a.name()) == self.attr.as_deref().unwrap_or("") {
                        let ns = a.namespace();
                        return match ns {
                            Some(ns) if !ns.is_empty() => Some(ns.to_string()),
                            _ => None,
                        };
                    }
                }
            }
            return None;
        }
        match self.node().node_type() {
            roxmltree::NodeType::Element => {
                Some(self.node().tag_name().namespace().unwrap_or("").to_string())
            }
            _ => None,
        }
    }

    /// 文本内容（Java NodeModel.getText：text → data；element → 子文本拼接；
    /// document → 文档元素文本；其余 → ""）
    fn text_content(&self) -> String {
        if self.is_attr() {
            return self.attribute_value().unwrap_or_default();
        }
        let n = self.node();
        match n.node_type() {
            roxmltree::NodeType::Text => n.text().unwrap_or("").to_string(),
            roxmltree::NodeType::Element => {
                let mut out = String::new();
                for c in n.children() {
                    if c.is_text() {
                        out.push_str(c.text().unwrap_or(""));
                    } else if c.is_element() {
                        // 子元素内文本（Java getText 递归；注释/PI 不产出文本）
                        let sub = XmlNode {
                            tree: self.tree.clone(),
                            node_id: c.id(),
                            attr: None,
                            doctype: false,
                        };
                        out.push_str(&sub.text_content());
                    }
                }
                out
            }
            roxmltree::NodeType::Root => {
                let mut out = String::new();
                for c in n.children() {
                    if c.is_element() {
                        let sub = XmlNode {
                            tree: self.tree.clone(),
                            node_id: c.id(),
                            attr: None,
                            doctype: false,
                        };
                        out.push_str(&sub.text_content());
                    }
                }
                out
            }
            _ => String::new(),
        }
    }

    /// 标量值（Java 各模型 getAsString：元素仅允许无元素子节点；文本 = data；
    /// 属性 = value；PI = data；注释 = data；doctype = 原始声明串）
    fn scalar_value(&self) -> Result<String> {
        // doctype 节点：返回原始声明串（Java 的 getAsString 是 ProcessingInstruction
        // 误转型怪癖——按 DOM 真实语义降级为返回原始声明串）
        if self.doctype {
            return Ok(self
                .tree
                .doctype
                .as_ref()
                .map(|d| d.raw.clone())
                .unwrap_or_default());
        }
        if let Some(v) = self.attribute_value() {
            return Ok(v);
        }
        let n = self.node();
        match n.node_type() {
            roxmltree::NodeType::Element => {
                // Java ElementModel.getAsString：子元素 → 报错
                for c in n.children() {
                    if c.is_element() {
                        return Err(TemplateError::misc(format!(
                            "Only elements with no child elements can be processed as text.\nThis element with name \"{}\" has a child element named: {}",
                            self.node_name().unwrap_or_default(),
                            c.tag_name().name()
                        )));
                    }
                }
                Ok(self.text_content())
            }
            roxmltree::NodeType::Text | roxmltree::NodeType::Comment => {
                Ok(n.text().unwrap_or("").to_string())
            }
            roxmltree::NodeType::PI => Ok(n
                .pi()
                .map(|p| p.value.unwrap_or(""))
                .unwrap_or("")
                .to_string()),
            _ => Err(TemplateError::type_mismatch("string", "node")),
        }
    }

    /// 属性值（仅 attr 包装 / 元素属性查询）
    fn attribute_value(&self) -> Option<String> {
        let an = self.attr.as_deref()?;
        let n = self.node();
        if !n.is_element() {
            return None;
        }
        // Java ElementModel.getAttribute：先精确 qname，再按前缀解析命名空间
        for a in n.attributes() {
            if a.name() == an || attr_qualified_name(n, a) == an {
                return Some(a.value().to_string());
            }
        }
        None
    }

    /// 父节点（Java getParentNode：元素的父可能是 document；属性的父 = 宿主元素）
    fn parent_node(&self) -> Option<XmlNode> {
        if self.attr.is_some() {
            // 属性节点：父 = 宿主元素
            return Some(XmlNode {
                tree: self.tree.clone(),
                node_id: self.node_id,
                attr: None,
                doctype: false,
            });
        }
        let n = self.node();
        if n.node_type() == roxmltree::NodeType::Root {
            return None;
        }
        n.parent().map(|p| XmlNode {
            tree: self.tree.clone(),
            node_id: p.id(),
            attr: None,
            doctype: false,
        })
    }

    /// 文档节点（Java getDocumentNodeModel）
    fn document_node(&self) -> XmlNode {
        XmlNode {
            tree: self.tree.clone(),
            node_id: self.tree.doc.root().id(),
            attr: None,
            doctype: false,
        }
    }

    /// 根元素（Java DocumentModel.getRootElement / Document.getDocumentElement）
    fn root_element(&self) -> Option<XmlNode> {
        let n = self.node().document().root_element();
        Some(XmlNode {
            tree: self.tree.clone(),
            node_id: n.id(),
            attr: None,
            doctype: false,
        })
    }

    // -----------------------------------------------------------------------
    // 哈希键访问（Java NodeModel.get / ElementModel.get / DocumentModel.get）
    // -----------------------------------------------------------------------

    /// 构造 TModel（node + node_hash + scalar 角色；Java NodeModel 单对象多角色）
    pub(crate) fn into_model(self) -> TModel {
        // Java：element/text/comment/attr/PI 实现 TemplateScalarModel；document 不实现；
        // doctype 实现 TemplateScalarModel（返回原始声明串）
        let is_document = self.node_type() == "document" && !self.doctype;
        let mut m = TModel::nothing();
        m.node = Some(Rc::new(self.clone()) as Rc<dyn TemplateNodeModel>);
        m.node_hash = Some(Rc::new(self.clone()) as Rc<dyn NodeHashModel>);
        if !is_document {
            m.scalar = Some(Rc::new(self) as Rc<dyn crate::template::TemplateScalarModel>);
        }
        m.type_name = "node";
        m.kind = ModelKind::Node;
        m
    }
}

impl TemplateNodeModel for XmlNode {
    fn parent(&self) -> Result<Option<TModel>> {
        Ok(self.parent_node().map(|p| p.into_model()))
    }

    fn children(&self) -> Result<Vec<TModel>> {
        if self.doctype {
            // Java DocumentTypeModel.getChildren：DTD 子节点不支持
            return Err(TemplateError::misc(
                "entering the child nodes of a DTD node is not currently supported",
            ));
        }
        if self.is_attr() {
            return Ok(Vec::new());
        }
        Ok(self
            .child_nodes()
            .into_iter()
            .map(|c| c.into_model())
            .collect())
    }

    fn next_sibling(&self) -> Result<Option<TModel>> {
        Ok(self.next_sibling_node().map(|n| n.into_model()))
    }

    fn previous_sibling(&self) -> Result<Option<TModel>> {
        Ok(self.previous_sibling_node().map(|n| n.into_model()))
    }

    fn name(&self) -> Result<Option<String>> {
        Ok(self.node_name())
    }

    fn node_type(&self) -> Result<String> {
        Ok(self.node_type())
    }

    fn namespace(&self) -> Result<Option<String>> {
        Ok(self.node_namespace())
    }
}

/// 标量角色（Java element/text/attr/PI/comment 的 getAsString；document 无此角色）
impl crate::template::TemplateScalarModel for XmlNode {
    fn as_string(&self) -> Result<String> {
        self.scalar_value()
    }
}

impl NodeHashModel for XmlNode {
    fn get(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        Ok(self.hash_get(env, key)?.map(ensure_node_list))
    }
}

/// FreeMarker `NodeListModel` 语义：查询结果既是节点序列，又支持继续用哈希键
/// （子元素名/@attr/XPath 子集）导航——对每个成员节点求键、拼接为新节点列表
/// （空列表 → 空结果，不报错；对应 Java `freemarker.ext.dom.NodeListModel.get`）。
struct NodeListModel {
    nodes: Vec<TModel>,
}

impl NodeHashModel for NodeListModel {
    fn get(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        let mut merged: Vec<TModel> = Vec::new();
        for nm in &self.nodes {
            if let Some(nh) = &nm.node_hash {
                if let Some(res) = nh.get(env, key)? {
                    flatten_node_result(&mut merged, res);
                }
            }
        }
        Ok(Some(make_node_list(merged)))
    }
}

fn flatten_node_result(out: &mut Vec<TModel>, m: TModel) {
    if let Some(seq) = &m.sequence {
        let n = seq.size().unwrap_or(0);
        for i in 0..n {
            if let Ok(item) = seq.get(i) {
                out.push(item);
            }
        }
    } else {
        out.push(m);
    }
}

fn make_node_list(nodes: Vec<TModel>) -> TModel {
    let mut out = TModel::from_sequence(nodes.clone());
    out.node_hash = Some(Rc::new(NodeListModel { nodes }) as Rc<dyn NodeHashModel>);
    out
}

fn ensure_node_list(m: TModel) -> TModel {
    if m.node_hash.is_none() {
        if let Some(seq) = &m.sequence {
            let n = seq.size().unwrap_or(0);
            let mut nodes = Vec::with_capacity(n);
            for i in 0..n {
                if let Ok(item) = seq.get(i) {
                    nodes.push(item);
                }
            }
            return make_node_list(nodes);
        }
    }
    m
}

/// XML 解析入口 —— 对应 Java `NodeModel.parse(InputSource)`（simplify：注释/PI 移除）
pub fn parse_xml(s: &str) -> Result<TModel> {
    Ok(XmlNode::parse(s)?.into_model())
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

/// 本地名（去掉前缀）
fn local_part(name: &str) -> String {
    match name.rsplit_once(':') {
        Some((_, l)) => l.to_string(),
        None => name.to_string(),
    }
}

/// 属性的 qualified name（DOM4J getQualifiedName：`prefix:local` 或 `local`）。
/// roxmltree Attribute 只有本地名 + namespace URI，前缀从元素的 xmlns 声明反查。
fn attr_qualified_name(el: roxmltree::Node, attr: roxmltree::Attribute) -> String {
    let local = attr.name();
    let Some(ns) = attr.namespace() else {
        return local.to_string();
    };
    // 在元素上查找 xmlns:prefix="uri" 声明，反查前缀
    for a in el.attributes() {
        if let Some(prefix) = a.name().strip_prefix("xmlns:") {
            if a.value() == ns {
                return format!("{prefix}:{local}");
            }
        }
    }
    local.to_string()
}

/// 属性/文本编码（Java StringUtil.XMLEncQAttr：< > & " → 实体；' 不转义）
fn split_qname(s: &str) -> Result<(Option<String>, String, bool)> {
    if s == "*" {
        return Ok((None, String::new(), true));
    }
    if s.is_empty() || s.starts_with('@') || s.starts_with('[') {
        return Err(TemplateError::misc(format!(
            "Unsupported XPath query: //{s}"
        )));
    }
    match s.split_once(':') {
        Some((p, l)) if !p.is_empty() && !l.is_empty() => {
            Ok((Some(p.to_string()), l.to_string(), false))
        }
        Some(_) => Err(TemplateError::misc(format!(
            "Unsupported XPath query: //{s}"
        ))),
        None => Ok((None, s.to_string(), false)),
    }
}

/// Java NodeOutputter.buildPrefixLookup：递归子树，nsURI → 前缀（模板映射或生成）
fn build_prefix_lookup(
    n: &Node<'_, 'static>,
    prefixes: &NsPrefixes,
    has_default_ns: bool,
    default_ns: Option<&str>,
    lookup: &mut Vec<(Option<String>, String)>,
    next_gen: &mut usize,
) {
    let ns_uri = n.tag_name().namespace();
    if let Some(uri) = ns_uri {
        if !uri.is_empty() {
            let prefix = match prefixes.get_prefix_for_namespace(uri) {
                Some(p) => p.to_string(),
                None => {
                    let existing = lookup
                        .iter()
                        .find(|(u, _)| u.as_deref() == Some(uri))
                        .map(|(_, p)| p.clone());
                    match existing {
                        Some(p) => p,
                        None => {
                            // 生成前缀（Java StringUtil.toLowerABC：a, b, ..., aa...）
                            loop {
                                let mut m = *next_gen;
                                *next_gen += 1;
                                let mut p = String::new();
                                while m > 0 {
                                    m -= 1;
                                    p.insert(0, char::from(b'a' + (m % 26) as u8));
                                    m /= 26;
                                }
                                if prefixes.get_namespace_for_prefix(&p).is_none() {
                                    break p;
                                }
                            }
                        }
                    }
                }
            };
            if !lookup.iter().any(|(u, _)| u.as_deref() == Some(uri)) {
                lookup.push((Some(uri.to_string()), prefix));
            }
        }
    } else if has_default_ns {
        if let Some(dns) = default_ns {
            if !lookup.iter().any(|(u, _)| u.as_deref() == Some(dns)) {
                // 当前节点没有命名空间时，默认命名空间仍须以空前缀声明；
                // 使用字面量 "D" 会错误地把子节点序列化成 `D:book`。
                lookup.push((Some(dns.to_string()), String::new()));
            }
        }
    }
    for c in n.children() {
        if c.is_element() {
            build_prefix_lookup(&c, prefixes, has_default_ns, default_ns, lookup, next_gen);
        }
    }
}

// 查询/序列化与测试按 #[path] 聚合拆分（参照 grammar.rs 模式）：
// - node_query.rs —— hash 导航查询族与 @@markup 序列化
// - node_tests.rs —— 测试（#[cfg(test)]）
#[path = "node_query.rs"]
mod node_query;
#[cfg(test)]
#[path = "node_tests.rs"]
mod node_tests;
