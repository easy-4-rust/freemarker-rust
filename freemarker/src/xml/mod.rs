//! XML 节点模型 —— 对应 Java `freemarker.ext.dom.NodeModel`（roxmltree 只读 DOM）
//!
//! 支持：
//! - `?children`/`?parent`/`?root`/`?node_name`/`?node_type`/`?node_namespace` 节点内建
//! - 哈希键访问：`@@` 特殊键（`@@markup`/`@@text`/`@@namespace` 等）、子元素名
//!   （`doc['n:c']`/`doc.root.e`）、属性（`@attr`）、`*`/`**`、以及 XPath 子集
//!   （`/`、`//name`、`//*`、`[n]`）—— 完整 XPath 引擎不在范围内（文档化限制）
//! - `<#visit>`/`<#recurse>`/`<#on>`/`<#fallback>` 的节点处理器分派（exec.rs）
//!
//! 语义对照 Java（freemarker-core `freemarker.ext.dom`）：
//! - `parse` 等价 `NodeModel.parse(InputSource)`：注释/PI 节点从树中移除（simplify）
//! - `@@markup` 等价 `NodeModel.get("@@markup")` → `NodeOutputter` 序列化
//! - 前缀解析走当前命名空间模板的 `ns_prefixes`（Java
//!   `currentNamespace.getTemplate().getNamespaceForPrefix`）

use crate::core::Environment;
use crate::error::{Result, TemplateError};
use crate::template::{ModelKind, NodeHashModel, TModel, TemplateNodeModel};
use roxmltree::{Document, Node, NodeId};
use std::collections::HashMap;
use std::rc::Rc;

/// 命名空间前缀映射 —— 对应 Java `Template` 的 ns_prefixes（`<#ftl ns_prefixes=...>`）。
/// 特殊前缀："D"（`Template.DEFAULT_NAMESPACE_PREFIX`）注册为默认命名空间；
/// "N"（`Template.NO_NS_PREFIX`）保留、不可注册。
#[derive(Debug, Default)]
pub struct NsPrefixes {
    /// prefix → URI（不含 "D"——它进入 default_ns）
    prefix_to_uri: HashMap<String, String>,
    /// URI → prefix（getPrefixForNamespace 反查）
    uri_to_prefix: HashMap<String, String>,
    /// 默认命名空间（`D` 前缀）
    default_ns: Option<String>,
}

impl NsPrefixes {
    pub fn new(map: HashMap<String, String>) -> Self {
        let mut p = NsPrefixes {
            prefix_to_uri: HashMap::new(),
            uri_to_prefix: HashMap::new(),
            default_ns: None,
        };
        for (prefix, uri) in map {
            // Java Template.addNsPrefix（Template.java:920-951）："N" 保留非法；
            // "D" 注册为 defaultNS；其余入 prefixToNamespaceURILookup + 反查表
            // （同 URI 只能映射一个前缀，重复即非法——解析期已校验）
            if prefix == "N" {
                continue;
            }
            if prefix == "D" {
                p.default_ns = Some(uri);
            } else {
                p.uri_to_prefix.insert(uri.clone(), prefix.clone());
                p.prefix_to_uri.insert(prefix, uri);
            }
        }
        p
    }

    /// 默认命名空间 URI（Java Template.getDefaultNS）
    pub fn get_default_ns(&self) -> Option<&str> {
        self.default_ns.as_deref()
    }

    /// prefix → URI（Java Template.getNamespaceForPrefix："" → defaultNS 或 ""）
    pub fn get_namespace_for_prefix(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() {
            return Some(self.default_ns.as_deref().unwrap_or(""));
        }
        self.prefix_to_uri.get(prefix).map(|s| s.as_str())
    }

    /// URI → prefix（Java Template.getPrefixForNamespace：null → null；
    /// "" → defaultNS 为 null ? "" : "N"；== defaultNS → ""；否则反查表）
    pub fn get_prefix_for_namespace(&self, ns_uri: &str) -> Option<&str> {
        if ns_uri.is_empty() {
            return Some(if self.default_ns.is_none() { "" } else { "N" });
        }
        if self.default_ns.as_deref() == Some(ns_uri) {
            return Some("");
        }
        self.uri_to_prefix.get(ns_uri).map(|s| s.as_str())
    }
}

/// XML 树持有者：文本 + 解析后的 Document 同住（Rc 自引用，见 parse 的安全注释）
struct XmlTree {
    /// 注意字段声明顺序：doc 先析构（其借用目标 _text 后释放）
    doc: Document<'static>,
    _text: Rc<str>,
}

impl XmlTree {
    /// 解析 XML 文本。安全性说明：roxmltree 的 `Document` 借用输入文本，无法直接
    /// 自引用持有；这里把文本与 Document 同放入 Rc<XmlTree>（doc 字段声明在
    /// _text 之前 → 析构时 doc 先释放，借用先于借用目标失效），并把 Document 的
    /// 借用生命周期延长为 'static。由 `XmlNode` 持有 `Rc<XmlTree>` 保证树在节点
    /// 存活期间不释放，因此该借用恒有效（标准 Rc 自引用模式）。
    fn parse(text: &str) -> Result<Rc<XmlTree>> {
        let text: Rc<str> = Rc::from(text);
        let doc = Document::parse(&text)
            .map_err(|e| TemplateError::misc(format!("XML parsing failed: {e}")))?;
        // 安全：见上方注释 —— doc 与 text 同住 Rc<XmlTree>，且 doc 先于 text 析构
        let doc: Document<'static> = unsafe { std::mem::transmute(doc) };
        Ok(Rc::new(XmlTree { doc, _text: text }))
    }
}

/// XML 节点 —— 对应 Java `NodeModel`（wrap 后的 W3C DOM 节点）。
/// roxmltree 的 `Node` 是 Copy 的轻量句柄（借树）；`node_id` + `Rc<XmlTree>` 复现
/// 节点引用（Rust 无法直接持有带生命周期 Node 的 Self 引用结构）。
#[derive(Clone)]
pub struct XmlNode {
    tree: Rc<XmlTree>,
    node_id: NodeId,
    /// Some(属性名)：包装的是 node_id 元素上的属性节点（Java Attr 节点）
    attr: Option<String>,
}

impl XmlNode {
    /// 文档节点（Java NodeModel.wrap(Document)；parse 入口）
    pub fn parse(s: &str) -> Result<XmlNode> {
        let tree = XmlTree::parse(s)?;
        Ok(XmlNode {
            node_id: tree.doc.root().id(),
            tree,
            attr: None,
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

    /// 子节点（Java simplify 后：注释/PI 已移除；text 与元素保留）
    fn child_nodes(&self) -> Vec<XmlNode> {
        let mut out = Vec::new();
        for c in self.node().children() {
            if matches!(
                c.node_type(),
                roxmltree::NodeType::Comment | roxmltree::NodeType::PI
            ) {
                continue;
            }
            out.push(XmlNode {
                tree: self.tree.clone(),
                node_id: c.id(),
                attr: None,
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
                });
            }
        }
        out
    }

    /// 元素本地名 / 文本 / 属性名（Java getNodeName 的各实现）
    fn node_name(&self) -> Option<String> {
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

    /// 节点类型（Java NodeModel.getNodeType：CDATA 也算 "text"；PI = "pi"）
    fn node_type(&self) -> String {
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
    /// 属性 = value；PI = data；注释 = data）
    fn scalar_value(&self) -> Result<String> {
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
        })
    }

    /// 文档节点（Java getDocumentNodeModel）
    fn document_node(&self) -> XmlNode {
        XmlNode {
            tree: self.tree.clone(),
            node_id: self.tree.doc.root().id(),
            attr: None,
        }
    }

    /// 根元素（Java DocumentModel.getRootElement / Document.getDocumentElement）
    fn root_element(&self) -> Option<XmlNode> {
        let n = self.node().document().root_element();
        Some(XmlNode {
            tree: self.tree.clone(),
            node_id: n.id(),
            attr: None,
        })
    }

    // -----------------------------------------------------------------------
    // 哈希键访问（Java NodeModel.get / ElementModel.get / DocumentModel.get）
    // -----------------------------------------------------------------------

    /// 键访问入口（NodeHashModel::get）—— 返回 None = 键缺失
    pub(crate) fn hash_get(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        if key.starts_with("@@") {
            return Ok(Some(self.atat_key(env, key)?));
        }
        if self.is_attr() {
            // 属性节点：Java NodeModel.get → XPath（子集在属性节点上无意义）
            return Ok(None);
        }
        let n = self.node();
        match n.node_type() {
            roxmltree::NodeType::Element => self.element_key(env, key),
            roxmltree::NodeType::Root => self.document_key(env, key),
            // 文本/注释/PI：Java NodeModel.get → XPath（子集对无后代节点恒空）
            _ => Ok(Some(TModel::from_sequence(Vec::new()))),
        }
    }

    /// 元素节点键（Java ElementModel.get）
    fn element_key(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        match key {
            "*" => {
                // 全部直接子元素（NodeListModel，含空）
                let kids: Vec<TModel> = self
                    .child_elements()
                    .into_iter()
                    .map(|c| c.into_model())
                    .collect();
                Ok(Some(TModel::from_sequence(kids)))
            }
            "**" => {
                // 全部后代元素（Java getElementsByTagName("*")）
                let kids: Vec<TModel> = self
                    .descendant_elements()
                    .into_iter()
                    .map(|c| c.into_model())
                    .collect();
                Ok(Some(TModel::from_sequence(kids)))
            }
            _ if key.starts_with('@') => {
                // `@attr` → 属性节点（缺失 → 空序列）；`@*` → 全部属性
                if key == "@*" {
                    let attrs: Vec<TModel> = self
                        .attributes()
                        .into_iter()
                        .map(|a| {
                            XmlNode {
                                tree: self.tree.clone(),
                                node_id: self.node_id,
                                attr: Some(a.name().to_string()),
                            }
                            .into_model()
                        })
                        .collect();
                    return Ok(Some(TModel::from_sequence(attrs)));
                }
                let an = &key[1..];
                if !is_xml_name_like(an) {
                    // 非 XML 名（如 "@@" 已在上层处理；其余回退 XPath 子集）
                    return self.xpath_subset(env, key);
                }
                match self.lookup_attribute(env, an) {
                    Some(xn) => Ok(Some(xn.into_model())),
                    None => Ok(Some(TModel::from_sequence(Vec::new()))),
                }
            }
            _ if is_xml_name_like(key) => self.filter_child_by_name(env, key),
            _ => self.xpath_subset(env, key),
        }
    }

    /// 子元素按名过滤：恰 1 个 → 节点；否则序列（Java ElementModel.get 的
    /// filterByName 语义，ElementModel.java:123-124）
    fn filter_child_by_name(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        let matches: Vec<TModel> = self
            .child_elements()
            .into_iter()
            .filter(|c| c.matches_name(env, key))
            .map(|c| c.into_model())
            .collect();
        if matches.len() == 1 {
            Ok(Some(matches.into_iter().next().unwrap()))
        } else {
            Ok(Some(TModel::from_sequence(matches)))
        }
    }

    /// 文档节点键（Java DocumentModel.get）
    fn document_key(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        match key {
            "*" => {
                // 文档的 `*` → 根元素（Java getRootElement()）
                Ok(self.root_element().map(|r| r.into_model()))
            }
            "**" => {
                let kids: Vec<TModel> = self
                    .descendant_elements()
                    .into_iter()
                    .map(|c| c.into_model())
                    .collect();
                Ok(Some(TModel::from_sequence(kids)))
            }
            _ if is_xml_name_like(key) => {
                // 根元素名匹配（Java DocumentModel.get：matchesName → 根元素，否则空序列）
                match self.root_element() {
                    Some(root) if root.matches_name(env, key) => Ok(Some(root.into_model())),
                    _ => Ok(Some(TModel::from_sequence(Vec::new()))),
                }
            }
            _ => self.xpath_subset(env, key),
        }
    }

    /// 属性查找（Java ElementModel.getAttribute：精确 qname 优先，
    /// 带前缀时按环境 ns_prefixes 解析命名空间）
    fn lookup_attribute(&self, env: &mut Environment, qname: &str) -> Option<XmlNode> {
        let n = self.node();
        if !n.is_element() {
            return None;
        }
        let mut by_ns: Option<XmlNode> = None;
        for a in n.attributes() {
            if attr_qualified_name(n, a) == qname {
                return Some(XmlNode {
                    tree: self.tree.clone(),
                    node_id: self.node_id,
                    attr: Some(a.name().to_string()),
                });
            }
            // 前缀形式 `p:attr`：解析 p → URI，匹配 (URI, localName)
            if let Some((prefix, local)) = qname.split_once(':') {
                let uri = if prefix == "D" {
                    env.current_ns_prefixes()
                        .get_default_ns()
                        .map(str::to_string)
                } else {
                    env.current_ns_prefixes()
                        .get_namespace_for_prefix(prefix)
                        .filter(|u| !u.is_empty())
                        .map(str::to_string)
                };
                if let Some(uri) = uri {
                    if a.namespace() == Some(uri.as_str()) && a.name() == local {
                        by_ns = Some(XmlNode {
                            tree: self.tree.clone(),
                            node_id: self.node_id,
                            attr: Some(local.to_string()),
                        });
                    }
                }
            }
        }
        by_ns
    }

    /// 名称匹配（Java DomStringUtil.matchesName + ElementModel.matchesName）
    fn matches_name(&self, env: &mut Environment, qname: &str) -> bool {
        let node_name = self.node_name().unwrap_or_default();
        let ns_uri = self.node_namespace().unwrap_or_default();
        let prefixes = env.current_ns_prefixes();
        let default_ns = prefixes.get_default_ns();
        if let Some(dns) = default_ns {
            if dns == ns_uri {
                return qname == node_name || qname == format!("D:{node_name}");
            }
        }
        if ns_uri.is_empty() {
            return if default_ns.is_some() {
                qname == format!("N:{node_name}")
            } else {
                qname == node_name || qname == format!("N:{node_name}")
            };
        }
        match prefixes.get_prefix_for_namespace(&ns_uri) {
            Some(p) => qname == format!("{p}:{node_name}"),
            None => false,
        }
    }

    // -----------------------------------------------------------------------
    // @@ 特殊键（Java AtAtKey 集合 + 本实现扩展键）
    // -----------------------------------------------------------------------

    fn atat_key(&self, env: &mut Environment, key: &str) -> Result<TModel> {
        let text = |m: &XmlNode| Ok(TModel::from_scalar(m.text_content()));
        let mut markup = |m: &XmlNode| Ok(TModel::from_scalar(m.markup(env)));
        match key {
            "@@markup" => markup(self),
            "@@nested_markup" => {
                // Java：children 序列化
                let mut buf = String::new();
                for c in self.child_nodes() {
                    buf.push_str(&c.markup(env));
                }
                Ok(TModel::from_scalar(buf))
            }
            "@@text" => text(self),
            "@@namespace" => match self.node_namespace() {
                Some(ns) if !ns.is_empty() => Ok(TModel::from_scalar(ns)),
                _ => Ok(TModel::nothing()), // Java 返回 null → 缺失
            },
            "@@local_name" => {
                let name = self.node_name().unwrap_or_default();
                Ok(TModel::from_scalar(name))
            }
            "@@qname" => match self.qualified_name(env) {
                Some(q) => Ok(TModel::from_scalar(q)),
                None => Ok(TModel::nothing()),
            },
            // 元素专用键（Java ElementModel.get）
            "@@" => {
                // 属性节点序列
                let attrs: Vec<TModel> = self
                    .attributes()
                    .into_iter()
                    .map(|a| {
                        XmlNode {
                            tree: self.tree.clone(),
                            node_id: self.node_id,
                            attr: Some(a.name().to_string()),
                        }
                        .into_model()
                    })
                    .collect();
                Ok(TModel::from_sequence(attrs))
            }
            "@@start_tag" | "@@end_tag" => {
                let n = self.node();
                if !n.is_element() {
                    return Err(TemplateError::misc(format!(
                        "\"{key}\" is not supported for an XML node of type \"{}\".",
                        self.node_type()
                    )));
                }
                let mark = self.markup(env);
                if key == "@@start_tag" {
                    // <qname decl attrs>
                    let inner = mark
                        .strip_prefix('<')
                        .and_then(|s| s.split_once('>'))
                        .map(|(s, _)| format!("<{s}>"))
                        .unwrap_or(mark);
                    Ok(TModel::from_scalar(inner))
                } else {
                    // </qname>
                    let name = self.qualified_name(env).unwrap_or_default();
                    Ok(TModel::from_scalar(format!("</{name}>")))
                }
            }
            "@@attributes_markup" => {
                let mut buf = String::new();
                for a in self.attributes() {
                    buf.push(' ');
                    buf.push_str(&attr_qualified_name(self.node(), a));
                    buf.push_str("=\"");
                    buf.push_str(&xml_enc_qattr(a.value()));
                    buf.push('"');
                }
                Ok(TModel::from_scalar(buf.trim().to_string()))
            }
            "@@previous_sibling_element" | "@@next_sibling_element" => {
                let n = self.node();
                let target = if key == "@@previous_sibling_element" {
                    n.prev_sibling_element()
                } else {
                    n.next_sibling_element()
                };
                match target {
                    Some(t) => Ok(XmlNode {
                        tree: self.tree.clone(),
                        node_id: t.id(),
                        attr: None,
                    }
                    .into_model()),
                    None => Ok(TModel::from_sequence(Vec::new())),
                }
            }
            // ---- 本实现扩展键（对应任务清单；Java 2.3.34 无这些键，报 Unsupported）----
            "@@children" | "@@nested" => {
                let kids: Vec<TModel> = self
                    .child_nodes()
                    .into_iter()
                    .map(|c| c.into_model())
                    .collect();
                Ok(TModel::from_sequence(kids))
            }
            "@@tag_name" => {
                let n = self.node();
                if n.is_element() {
                    Ok(TModel::from_scalar(
                        self.qualified_name(env).unwrap_or_default(),
                    ))
                } else {
                    Ok(TModel::from_scalar(self.node_name().unwrap_or_default()))
                }
            }
            "@@name" => Ok(TModel::from_scalar(self.node_name().unwrap_or_default())),
            "@@type" => Ok(TModel::from_scalar(self.node_type())),
            _ => Err(TemplateError::misc(format!("Unsupported @@ key: {key}"))),
        }
    }

    /// 限定名（Java ElementModel.getQualifiedName：nsURI 经环境前缀映射；
    /// 无映射 → null）
    fn qualified_name(&self, env: &mut Environment) -> Option<String> {
        let node_name = self.node_name()?;
        if self.is_attr() {
            // Java AttributeNodeModel.getQualifiedName
            let ns = self.node_namespace();
            return match ns {
                None => Some(node_name),
                Some(ns) => {
                    let prefixes = env.current_ns_prefixes();
                    let default_ns = prefixes.get_default_ns();
                    let prefix = if default_ns == Some(ns.as_str()) {
                        "D".to_string()
                    } else {
                        prefixes.get_prefix_for_namespace(&ns)?.to_string()
                    };
                    Some(format!("{prefix}:{node_name}"))
                }
            };
        }
        let ns = self.node_namespace();
        match ns {
            None => Some(node_name),
            Some(ns) if ns.is_empty() => Some(node_name),
            Some(ns) => {
                let prefixes = env.current_ns_prefixes();
                let default_ns = prefixes.get_default_ns();
                let prefix = if default_ns == Some(ns.as_str()) {
                    ""
                } else {
                    prefixes.get_prefix_for_namespace(&ns)?
                };
                if prefix.is_empty() {
                    Some(node_name)
                } else {
                    Some(format!("{prefix}:{node_name}"))
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // XPath 子集（Java NodeModel.get 的非特殊键 → Xalan/Jaxen XPath 引擎；
    // 本实现仅支持任务规定的子集：`/`、`//name`、`//*`、`[n]`）
    // -----------------------------------------------------------------------

    fn xpath_subset(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        if key == "/" {
            // XPath "/"：上下文节点的文档根
            return Ok(Some(self.document_node().into_model()));
        }
        if key == "true()" {
            // XPath true() 函数（default-xmlns 用例：`doc["true()"]` → 布尔 true；
            // Java XPath 对常量函数求值）
            return Ok(Some(TModel::from_boolean(true)));
        }
        if let Some(rest) = key.strip_prefix("./") {
            // 相对路径 `./name`：当前节点的子元素按名过滤（XPath child::axis；
            // default-xmlns 用例 `r["./D:t4"]`）
            return self.filter_child_by_name(env, rest);
        }
        if let Some(rest) = key.strip_prefix("//") {
            // 后代元素匹配（descendant-or-self::node()/child::X —— 不含自身）
            let (prefix, local, wildcard) = split_qname(rest)?;
            let ns_uri = resolve_prefix(env, prefix.as_deref())?; // Option<String>；None = 无命名空间
            let mut matches = Vec::new();
            for d in self.node().descendants() {
                if !d.is_element() {
                    continue;
                }
                if wildcard || d.tag_name().name() == local {
                    let d_ns = d.tag_name().namespace();
                    if ns_uri.as_deref() == d_ns {
                        matches.push(XmlNode {
                            tree: self.tree.clone(),
                            node_id: d.id(),
                            attr: None,
                        });
                    }
                }
            }
            let models: Vec<TModel> = matches.into_iter().map(|m| m.into_model()).collect();
            if models.len() == 1 {
                return Ok(Some(models.into_iter().next().unwrap()));
            }
            return Ok(Some(TModel::from_sequence(models)));
        }
        if key.starts_with('[') && key.ends_with(']') {
            // `[n]`：第 n 个子元素（1 起始；dom4j 风格扩展 —— 完整 XPath 不支持）
            let idx: usize = match key[1..key.len() - 1].trim().parse() {
                Ok(i) => i,
                Err(_) => {
                    return Err(TemplateError::misc(format!(
                        "Unsupported XPath query: {key}"
                    )))
                }
            };
            if idx == 0 {
                return Err(TemplateError::misc(format!(
                    "Unsupported XPath query: {key}"
                )));
            }
            let kids = self.child_elements();
            if idx <= kids.len() {
                return Ok(Some(kids[idx - 1].clone().into_model()));
            }
            return Ok(Some(TModel::from_sequence(Vec::new())));
        }
        Err(TemplateError::misc(format!(
            "Unsupported XPath query: {key}"
        )))
    }

    // -----------------------------------------------------------------------
    // @@markup 序列化（Java NodeOutputter）
    // -----------------------------------------------------------------------

    /// XML 标记（Java NodeModel.getMarkup → NodeOutputter.outputContent(node)）
    fn markup(&self, env: &mut Environment) -> String {
        let prefixes = env.current_ns_prefixes();
        let default_ns = prefixes.get_default_ns().map(str::to_string);
        let has_default_ns = default_ns.as_deref().is_some_and(|s| !s.is_empty());

        // 前缀查找表（插入序）：null → ""、"" → ""，随后按子树节点 nsURI 填充
        let mut lookup: Vec<(Option<String>, String)> = vec![(None, String::new())];
        let mut next_gen = 1usize;
        build_prefix_lookup(
            &self.node(),
            &prefixes,
            has_default_ns,
            default_ns.as_deref(),
            &mut lookup,
            &mut next_gen,
        );
        if let Some(dns) = &default_ns {
            if !lookup
                .iter()
                .any(|(u, _)| u.as_deref() == Some(dns.as_str()))
            {
                lookup.push((Some(dns.clone()), String::new()));
            }
        }

        // namespaceDecl（Java constructNamespaceDecl）
        let mut ns_decl = String::new();
        for (uri, prefix) in &lookup {
            let Some(uri) = uri else { continue };
            if uri.is_empty() {
                continue;
            }
            ns_decl.push_str(" xmlns");
            if !prefix.is_empty() {
                ns_decl.push(':');
                ns_decl.push_str(prefix);
            }
            ns_decl.push_str("=\"");
            ns_decl.push_str(uri);
            ns_decl.push('"');
        }

        // 序列化（Java outputContent(node)）
        let mut buf = String::new();
        self.output_content(&prefixes, &lookup, &ns_decl, &mut buf);
        buf
    }

    fn output_content(
        &self,
        prefixes: &NsPrefixes,
        lookup: &[(Option<String>, String)],
        ns_decl: &str,
        buf: &mut String,
    ) {
        if self.is_attr() {
            // 属性（Java outputContent(Attr)：` qname="value"`）
            if let Some(v) = self.attribute_value() {
                buf.push(' ');
                buf.push_str(&self.output_qname(prefixes, lookup));
                buf.push_str("=\"");
                buf.push_str(&xml_enc_qattr(&v));
                buf.push('"');
            }
            return;
        }
        let n = self.node();
        match n.node_type() {
            roxmltree::NodeType::Root => {
                for c in self.child_nodes() {
                    c.output_content(prefixes, lookup, ns_decl, buf);
                }
            }
            roxmltree::NodeType::Element => {
                buf.push('<');
                buf.push_str(&self.output_qname(prefixes, lookup));
                // 上下文节点（Java NodeOutputter.contextNode）附 namespaceDecl
                if Some(self.node_id) == self.tree.doc.root_element().id().into() {
                    buf.push_str(ns_decl);
                }
                // 属性（排除 xmlns:*）
                for a in n.attributes() {
                    if a.name().starts_with("xmlns") {
                        continue;
                    }
                    buf.push(' ');
                    buf.push_str(&attr_qualified_name(self.node(), a));
                    buf.push_str("=\"");
                    buf.push_str(&xml_enc_qattr(a.value()));
                    buf.push('"');
                }
                let kids = self.child_nodes();
                if kids.is_empty() {
                    buf.push_str(" />");
                } else {
                    buf.push('>');
                    for c in kids {
                        c.output_content(prefixes, lookup, ns_decl, buf);
                    }
                    buf.push_str("</");
                    buf.push_str(&self.output_qname(prefixes, lookup));
                    buf.push('>');
                }
            }
            roxmltree::NodeType::Text => {
                buf.push_str(&xml_enc_nqg(n.text().unwrap_or("")));
            }
            roxmltree::NodeType::Comment => {
                buf.push_str("<!--");
                buf.push_str(n.text().unwrap_or(""));
                buf.push_str("-->");
            }
            roxmltree::NodeType::PI => {
                if let Some(pi) = n.pi() {
                    buf.push_str("<?");
                    buf.push_str(pi.target);
                    buf.push(' ');
                    buf.push_str(pi.value.unwrap_or(""));
                    buf.push_str("?>");
                }
            }
        }
    }

    /// 限定名输出（Java NodeOutputter.outputQualifiedName：经前缀查找表）
    fn output_qname(&self, _prefixes: &NsPrefixes, lookup: &[(Option<String>, String)]) -> String {
        if self.is_attr() {
            return self.attr.as_deref().unwrap_or_default().to_string();
        }
        let n = self.node();
        let local = n.tag_name().name();
        match n.tag_name().namespace() {
            None | Some("") => local.to_string(),
            Some(uri) => match lookup.iter().find(|(u, _)| u.as_deref() == Some(uri)) {
                Some((_, p)) if !p.is_empty() => format!("{p}:{local}"),
                _ => local.to_string(),
            },
        }
    }

    /// 构造 TModel（node + node_hash + scalar 角色；Java NodeModel 单对象多角色）
    pub(crate) fn into_model(self) -> TModel {
        // Java：element/text/comment/attr/PI 实现 TemplateScalarModel；document 不实现
        let is_document = self.node_type() == "document";
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
        self.hash_get(env, key)
    }
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
fn xml_enc_qattr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// 文本编码（Java StringUtil.XMLEncNQG：< & → 实体；> 仅 `]]>` 序列中转义）
fn xml_enc_nqg(s: &str) -> String {
    let bytes: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in bytes.iter().enumerate() {
        match c {
            '<' => out.push_str("&lt;"),
            '&' => out.push_str("&amp;"),
            '>' if i >= 2 && bytes[i - 2] == ']' && bytes[i - 1] == ']' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Java DomStringUtil.isXMLNameLike：字母/数字/_/-/. + 单个 ":"；首字符非 -/. 数字
fn is_xml_name_like(name: &str) -> bool {
    // XPath/特殊符号开头 → 非元素名（`/`、`//`、`@`、`*`、`[` 等走 XPath 子集）
    if matches!(
        name.chars().next(),
        Some('/') | Some('@') | Some('*') | Some('[') | Some('.')
    ) {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '-' || c == '.' || c.is_ascii_digit() => return false,
        Some(_) => {}
        None => return false,
    }
    let mut colon = false;
    for c in chars {
        if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
            continue;
        }
        if c == ':' {
            if colon {
                return false; // "::" 是 XPath
            }
            colon = true;
            continue;
        }
        return false;
    }
    true
}

/// 拆分 `prefix:local` / `*` / `local`（XPath 名称测试）
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

/// 解析 XPath 前缀（Java XalanXPathSupport.CUSTOM_PREFIX_RESOLVER：D → defaultNS；
/// 其余 → getNamespaceForPrefix；未注册 → 报错）
fn resolve_prefix(env: &mut Environment, prefix: Option<&str>) -> Result<Option<String>> {
    let prefixes = env.current_ns_prefixes();
    match prefix {
        None => Ok(None), // XPath 无前缀名 = 无命名空间
        Some("D") => Ok(prefixes.get_default_ns().map(str::to_string)),
        Some(p) => match prefixes.get_namespace_for_prefix(p) {
            Some(uri) if !uri.is_empty() => Ok(Some(uri.to_string())),
            _ => Err(TemplateError::misc(format!(
                "namespace prefix \"{p}\" has not been declared"
            ))),
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{Configuration, Template};

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
}

#[cfg(test)]
mod sibling_tests {
    use super::*;

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
        };
        let b_node = XmlNode {
            tree: doc.tree.clone(),
            node_id: b.id(),
            attr: None,
        };
        let c_node = XmlNode {
            tree: doc.tree.clone(),
            node_id: c.id(),
            attr: None,
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
