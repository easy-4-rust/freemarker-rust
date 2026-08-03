//! 模板加载与缓存 —— 对应 Java `freemarker.cache.*`
//! （全量语义见 docs/07 §4）

mod byte_array_template_loader;
mod class_template_loader;
mod file_template_loader;
mod multi_template_loader;
mod string_template_loader;
mod template_cache;
mod template_loader;
mod template_lookup_strategy;
mod template_name_format;
mod url_template_loader;

pub use byte_array_template_loader::{ByteArrayTemplateLoader, ByteArrayTemplateSource};
pub use class_template_loader::{ClassTemplateLoader, ClassTemplateSource};
pub use file_template_loader::{FileLoader, FileSource};
pub use multi_template_loader::{MultiLoader, MultiSource};
pub use string_template_loader::{StringLoader, StringSource};
pub use template_cache::TemplateCache;
pub use template_loader::{TemplateLoader, TemplateSource};
pub use template_lookup_strategy::{
    Default020300 as LookupStrategyDefault020300, LookupResult, LookupStrategyKind,
    TemplateLookupStrategy,
};
pub use template_name_format::{
    Default020300 as NameFormatDefault020300, Default020400 as NameFormatDefault020400,
    TemplateNameFormat,
};
pub use url_template_loader::URLTemplateLoader;
