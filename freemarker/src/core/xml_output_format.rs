//! XML 输出格式 —— 对应 Java `freemarker.core.XMLOutputFormat`
//! （escape：`& < > " \'`；name = "XML"）

use crate::template::utility::xml_escape;

/// XML 输出格式（对应 XMLOutputFormat.java；OutputFormatKind::Xml 的承载）
#[allow(dead_code)]
pub(crate) struct XmlOutputFormat;

#[allow(dead_code)]
impl XmlOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "XML"
    }

    /// Java `escape(String)` → XMLEncUtil（`& < > " \'` 实体集）
    pub(crate) fn escape(s: &str) -> String {
        xml_escape(s)
    }
}
