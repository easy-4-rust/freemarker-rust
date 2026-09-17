//! 节点查询与序列化 —— hash 导航（元素/属性/@@ 特殊键/XPath 子集）与
//! @@markup 输出（对应 Java ElementModel/DocumentModel.get 与 NodeOutputter）。

use super::XmlNode;
use crate::core::Environment;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

use crate::xml::ns_prefixes::NsPrefixes;
use crate::xml::xml_dom_string_util::{is_xml_name_like, xml_enc_nqg, xml_enc_qattr};
impl XmlNode {
    pub(crate) fn hash_get(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>> {
        if key.starts_with("@@") {
            return Ok(Some(self.atat_key(env, key)?));
        }
        if self.doctype {
            // Java DocumentTypeModel.get：访问 DTD 属性不支持
            return Err(TemplateError::misc(
                "accessing properties of a DTD is not currently supported",
            ));
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
                                doctype: false,
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
            if super::attr_qualified_name(n, a) == qname {
                return Some(XmlNode {
                    tree: self.tree.clone(),
                    node_id: self.node_id,
                    attr: Some(a.name().to_string()),
                    doctype: false,
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
                            doctype: false,
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
                            doctype: false,
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
                    buf.push_str(&super::attr_qualified_name(self.node(), a));
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
                        doctype: false,
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
            // @@nodeName：节点名（元素标签名/文本 "@text"/文档 "@document" 等，
            // 与 ?node_name 内建一致 —— Java getNodeName 语义；2.3.34 的 AtAtKey
            // 无此键，本实现按任务清单作为扩展键提供，见 tests/xml_coverage.rs）
            "@@nodeName" => Ok(TModel::from_scalar(self.node_name().unwrap_or_default())),
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
            // 后代元素匹配（descendant-or-self::node()/child::X —— 不含自身）。
            // 名称匹配用 ElementModel.matchesName（DomStringUtil.matchesName，
            // DomStringUtil.java:73-90）：无前缀名在元素命名空间 == 模板默认命名
            // 空间（ns_prefixes 声明的 D）时匹配——Java Jaxen 的 customNamespaceContext
            // 把无前缀名解析为 getNamespaceForPrefix("") = 默认 NS
            // （JaxenXPathSupport.java customNamespaceContext + Template.java
            // getNamespaceForPrefix("")）。`//*` 通配匹配全部元素（XPath 1.0 的
            // `*` 名称测试与命名空间无关）。
            let (_, _, wildcard) = super::split_qname(rest)?;
            let mut matches = Vec::new();
            for d in self.node().descendants() {
                if !d.is_element() {
                    continue;
                }
                let d_node = XmlNode {
                    tree: self.tree.clone(),
                    node_id: d.id(),
                    attr: None,
                    doctype: false,
                };
                if wildcard || d_node.matches_name(env, rest) {
                    matches.push(d_node);
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
    pub(crate) fn markup(&self, env: &mut Environment) -> String {
        let prefixes = env.current_ns_prefixes();
        let default_ns = prefixes.get_default_ns().map(str::to_string);
        let has_default_ns = default_ns.as_deref().is_some_and(|s| !s.is_empty());

        // 前缀查找表（插入序）：null → ""、"" → ""，随后按子树节点 nsURI 填充
        let mut lookup: Vec<(Option<String>, String)> = vec![(None, String::new())];
        let mut next_gen = 1usize;
        super::build_prefix_lookup(
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
                    buf.push_str(&super::attr_qualified_name(self.node(), a));
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
}
