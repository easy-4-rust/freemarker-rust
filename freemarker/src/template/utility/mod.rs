//! 工具函数 —— 对应 Java `freemarker.template.utility.*`

mod fnv;
mod string_util;

pub use fnv::{FnvBuildHasher, FnvHasher};
pub use string_util::{glob_to_regex, html_enc_legacy, html_escape, java_trim, xml_escape};
