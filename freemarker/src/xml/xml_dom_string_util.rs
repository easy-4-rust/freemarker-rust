//! XML 字符串工具 —— 对应 Java `freemarker.ext.dom.XmlDomStringUtil`
//! （XML 属性值/节点名转义与合法性判定；v1 供 node.rs 的
//! NodeHashModel.get/序列化路径使用）

pub(crate) fn xml_enc_qattr(s: &str) -> String {
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
pub(crate) fn xml_enc_nqg(s: &str) -> String {
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
pub(crate) fn is_xml_name_like(name: &str) -> bool {
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
