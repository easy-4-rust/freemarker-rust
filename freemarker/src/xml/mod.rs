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
//!
//! 模块布局（一文件一 Java 对象，docs/JavaRust结构对照.md）：
//! - `ns_prefixes.rs`：`NsPrefixes` —— 对应 Java `Template` 的 ns_prefixes
//! - `tree.rs`：`XmlTree` —— 树持有者（Rust 特有：Document 借用文本的自引用）
//! - `node.rs`：`XmlNode` + `parse_xml` —— 对应 Java `NodeModel`（含 ElementModel/
//!   TextModel 等子类分支）
//! - `xml_dom_string_util.rs`：`XmlDomStringUtil` 的转义/判定工具
//! - 其余各模型类文件为 Java 类对应锚点（Rust 由 XmlNode 分支承载）

mod attr_value;
mod attribute_node_model;
mod cdata_model;
mod comment_model;
mod document_type_model;
mod element_model;
mod entity_model;
mod misc_node_model;
mod node;
mod node_list_model;
mod node_outputter;
mod ns_prefixes;
mod processing_instruction_model;
mod text_model;
mod tree;
mod xml_dom_string_util;

pub use node::{parse_xml, XmlNode};
pub use ns_prefixes::NsPrefixes;
