//! XML 树持有者 —— 对应 Java `freemarker.ext.dom.NodeModel` 内部 wrap 的 DOM 树。

use crate::error::{Result, TemplateError};
use roxmltree::Document;
use std::rc::Rc;

/// DOCTYPE 声明信息（roxmltree 0.21.1 无 doctype API，自扫源文本提取）
#[derive(Debug, Clone)]
pub(crate) struct DoctypeInfo {
    /// DOCTYPE 名称（如 "html"、"note"）
    pub(crate) name: String,
    /// 原始声明串（如 `<!DOCTYPE html>`）
    pub(crate) raw: String,
}

/// XML 树持有者：文本 + 解析后的 Document 同住（Rc 自引用，见 parse 的安全注释）
pub(crate) struct XmlTree {
    /// 注意字段声明顺序：doc 先析构（其借用目标 _text 后释放）
    pub(crate) doc: Document<'static>,
    _text: Rc<str>,
    /// DOCTYPE 声明（roxmltree 无 doctype API，自扫源文本提取）
    pub(crate) doctype: Option<DoctypeInfo>,
}

impl XmlTree {
    /// 解析 XML 文本。安全性说明：roxmltree 的 `Document` 借用输入文本，无法直接
    /// 自引用持有；这里把文本与 Document 同放入 Rc<XmlTree>（doc 字段声明在
    /// _text 之前 → 析构时 doc 先释放，借用先于借用目标失效），并把 Document 的
    /// 借用生命周期延长为 'static。由 `XmlNode` 持有 `Rc<XmlTree>` 保证树在节点
    /// 存活期间不释放，因此该借用恒有效（标准 Rc 自引用模式）。
    pub(crate) fn parse(text: &str) -> Result<Rc<XmlTree>> {
        // 先扫描 DOCTYPE 声明（roxmltree 0.21.1 无 doctype API，且会拒绝 DTD）
        let doctype = scan_doctype(text);
        // 若有 DOCTYPE，剥离后再传给 roxmltree 解析
        let parse_text = if let Some(ref dt) = doctype {
            text.replace(&dt.raw, "")
        } else {
            text.to_string()
        };
        let text: Rc<str> = Rc::from(parse_text);
        let doc = Document::parse(&text)
            .map_err(|e| TemplateError::misc(format!("XML parsing failed: {e}")))?;
        // 安全：见上方注释 —— doc 与 text 同住 Rc<XmlTree>，且 doc 先于 text 析构
        let doc: Document<'static> = unsafe { std::mem::transmute(doc) };
        Ok(Rc::new(XmlTree {
            doc,
            _text: text,
            doctype,
        }))
    }
}

/// 扫描源文本中的 DOCTYPE 声明（roxmltree 0.21.1 无 doctype API）
///
/// 查找 `<!DOCTYPE name ...>` 模式，提取 name 和原始声明串。
/// 只扫描文档头部（在第一个 `<` 元素之前）。
fn scan_doctype(text: &str) -> Option<DoctypeInfo> {
    // 查找 <!DOCTYPE 开头（不区分大小写，但 XML 规范要求大写）
    let upper = text.to_uppercase();
    let start = upper.find("<!DOCTYPE")?;
    // 从 start 开始找到匹配的 >
    let rest = &text[start..];
    let end = rest.find('>')?;
    let raw = rest[..=end].to_string();
    // 提取 DOCTYPE 名称：<!DOCTYPE 之后的第一个非空白 token
    let after_keyword = &raw["<!DOCTYPE".len()..];
    let after_keyword = after_keyword.trim_start();
    let name = after_keyword
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches('>')
        .to_string();
    if name.is_empty() {
        return None;
    }
    Some(DoctypeInfo { name, raw })
}
