//! 工具函数 —— 对应 Java `freemarker.template.utility.*`
//! （一文件一 Java 对象；fnv.rs 为 Rust 特有）

mod capture_output;
mod date_util;
mod execute;
mod fnv;
mod html_escape;
mod normalize_newlines;
mod number_util;
mod object_constructor;
mod standard_compress;
mod string_util;
mod xml_escape;

pub use capture_output::CaptureOutputTransform;
pub use date_util::DateUtil;
pub use execute::Execute;
pub use fnv::{FnvBuildHasher, FnvHasher};
pub use html_escape::{html_escape_entity, HtmlEscapeTransform};
pub use normalize_newlines::{normalize_newlines_text, NormalizeNewlinesTransform};
pub use number_util::NumberUtil;
pub use object_constructor::ObjectConstructorFn;
pub use standard_compress::{standard_compress_text, StandardCompressTransform};
pub use string_util::{glob_to_regex, html_enc_legacy, html_escape, java_trim, xml_escape};
pub use xml_escape::XmlEscapeTransform;
