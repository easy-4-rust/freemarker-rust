//! XML 树持有者 —— 对应 Java `freemarker.ext.dom.NodeModel` 内部 wrap 的 DOM 树。

use crate::error::{Result, TemplateError};
use roxmltree::Document;
use std::rc::Rc;

/// XML 树持有者：文本 + 解析后的 Document 同住（Rc 自引用，见 parse 的安全注释）
pub(crate) struct XmlTree {
    /// 注意字段声明顺序：doc 先析构（其借用目标 _text 后释放）
    pub(crate) doc: Document<'static>,
    _text: Rc<str>,
}

impl XmlTree {
    /// 解析 XML 文本。安全性说明：roxmltree 的 `Document` 借用输入文本，无法直接
    /// 自引用持有；这里把文本与 Document 同放入 Rc<XmlTree>（doc 字段声明在
    /// _text 之前 → 析构时 doc 先释放，借用先于借用目标失效），并把 Document 的
    /// 借用生命周期延长为 'static。由 `XmlNode` 持有 `Rc<XmlTree>` 保证树在节点
    /// 存活期间不释放，因此该借用恒有效（标准 Rc 自引用模式）。
    pub(crate) fn parse(text: &str) -> Result<Rc<XmlTree>> {
        let text: Rc<str> = Rc::from(text);
        let doc = Document::parse(&text)
            .map_err(|e| TemplateError::misc(format!("XML parsing failed: {e}")))?;
        // 安全：见上方注释 —— doc 与 text 同住 Rc<XmlTree>，且 doc 先于 text 析构
        let doc: Document<'static> = unsafe { std::mem::transmute(doc) };
        Ok(Rc::new(XmlTree { doc, _text: text }))
    }
}
