//! 工具函数 —— 对应 Java `freemarker.template.utility.*`

mod string_util;

pub use string_util::{html_escape, java_trim, xml_escape};
