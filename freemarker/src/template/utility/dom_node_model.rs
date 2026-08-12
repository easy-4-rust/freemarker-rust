//! DOM 节点模型 —— 对应 Java `freemarker.template.utility.DOMNodeModel`
//! （org.w3c.dom 节点 → TemplateNodeModel 适配）
//! v1 差异：XML 支持基于 roxmltree（xml/ 模块），不依赖 JVM DOM——NA

/// DOM 节点模型（对应 DOMNodeModel.java；v1 XML 支持见 xml/ 模块——NA）
pub struct DOMNodeModel;
