//! 模板加载与缓存 —— 对应 Java `freemarker.cache.*`
//! （全量语义见 docs/07 §4）

mod and_matcher;
mod byte_array_template_loader;
mod class_template_loader;
mod conditional_template_configuration_factory;
mod file_extension_matcher;
mod file_name_glob_matcher;
mod file_template_loader;
mod first_match_template_configuration_factory;
mod merging_template_configuration_factory;
mod multi_template_loader;
mod not_matcher;
mod or_matcher;
mod path_glob_matcher;
mod path_regex_matcher;
mod string_template_loader;
mod template_cache;
mod template_configuration_factory;
mod template_configuration_factory_exception;
mod template_loader;
mod template_lookup_strategy;
mod template_name_format;
mod template_source;
mod template_source_matcher;
mod url_template_loader;

pub use and_matcher::AndMatcher;
pub use byte_array_template_loader::{ByteArrayTemplateLoader, ByteArrayTemplateSource};
pub use class_template_loader::{ClassTemplateLoader, ClassTemplateSource};
pub use conditional_template_configuration_factory::ConditionalTemplateConfigurationFactory;
pub use file_extension_matcher::FileExtensionMatcher;
pub use file_name_glob_matcher::FileNameGlobMatcher;
pub use file_template_loader::{FileLoader, FileSource};
pub use first_match_template_configuration_factory::FirstMatchTemplateConfigurationFactory;
pub use merging_template_configuration_factory::MergingTemplateConfigurationFactory;
pub use multi_template_loader::{MultiLoader, MultiSource};
pub use not_matcher::NotMatcher;
pub use or_matcher::OrMatcher;
pub use path_glob_matcher::PathGlobMatcher;
pub use path_regex_matcher::PathRegexMatcher;
pub use string_template_loader::{StringLoader, StringSource};
pub use template_cache::TemplateCache;
pub use template_configuration_factory::TemplateConfigurationFactory;
pub use template_configuration_factory_exception::TemplateConfigurationFactoryException;
pub use template_loader::TemplateLoader;
pub use template_lookup_strategy::{
    Default020300 as LookupStrategyDefault020300, LookupResult, LookupStrategyKind,
    TemplateLookupStrategy,
};
pub use template_name_format::{
    Default020300 as NameFormatDefault020300, Default020400 as NameFormatDefault020400,
    TemplateNameFormat,
};
pub use template_source::TemplateSource;
pub use template_source_matcher::TemplateSourceMatcher;
pub use url_template_loader::URLTemplateLoader;
