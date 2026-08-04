//! 模板加载与缓存 —— 对应 Java `freemarker.cache.*`
//! （全量语义见 docs/07 §4；一文件一 Java 对象）

mod and_matcher;
mod byte_array_template_loader;
mod cache_storage;
mod cache_storage_with_get_size;
mod class_template_loader;
mod concurrent_cache_storage;
mod conditional_template_configuration_factory;
mod file_extension_matcher;
mod file_name_glob_matcher;
mod file_template_loader;
mod first_match_template_configuration_factory;
mod merging_template_configuration_factory;
mod mru_cache_storage;
mod multi_template_loader;
mod not_matcher;
mod null_cache_storage;
mod or_matcher;
mod path_glob_matcher;
mod path_regex_matcher;
mod soft_cache_storage;
mod stateful_template_loader;
mod string_template_loader;
mod strong_cache_storage;
mod template_cache;
mod template_configuration_factory;
mod template_configuration_factory_exception;
mod template_loader;
mod template_loader_utils;
mod template_lookup_context;
mod template_lookup_result;
mod template_lookup_strategy;
mod template_name_format;
mod template_source;
mod template_source_matcher;
mod url_template_loader;
mod url_template_source;

pub use and_matcher::AndMatcher;
pub use byte_array_template_loader::{ByteArrayTemplateLoader, ByteArrayTemplateSource};
pub use cache_storage::CacheStorage;
pub use cache_storage_with_get_size::CacheStorageWithGetSize;
pub use class_template_loader::{ClassTemplateLoader, ClassTemplateSource};
pub use concurrent_cache_storage::ConcurrentCacheStorage;
pub use conditional_template_configuration_factory::ConditionalTemplateConfigurationFactory;
pub use file_extension_matcher::FileExtensionMatcher;
pub use file_name_glob_matcher::FileNameGlobMatcher;
pub use file_template_loader::{FileLoader, FileSource};
pub use first_match_template_configuration_factory::FirstMatchTemplateConfigurationFactory;
pub use merging_template_configuration_factory::MergingTemplateConfigurationFactory;
pub use mru_cache_storage::MruCacheStorage;
pub use multi_template_loader::{MultiLoader, MultiSource};
pub use not_matcher::NotMatcher;
pub use null_cache_storage::NullCacheStorage;
pub use or_matcher::OrMatcher;
pub use path_glob_matcher::PathGlobMatcher;
pub use path_regex_matcher::PathRegexMatcher;
pub use soft_cache_storage::SoftCacheStorage;
pub use stateful_template_loader::StatefulTemplateLoader;
pub use string_template_loader::{StringLoader, StringSource};
pub use strong_cache_storage::StrongCacheStorage;
pub use template_cache::TemplateCache;
pub use template_configuration_factory::TemplateConfigurationFactory;
pub use template_configuration_factory_exception::TemplateConfigurationFactoryException;
pub use template_loader::TemplateLoader;
pub use template_loader_utils::get_class_name_for_to_string;
pub use template_lookup_context::FindFn;
pub use template_lookup_result::LookupResult;
pub use template_lookup_strategy::{
    Default020300 as LookupStrategyDefault020300, LookupStrategyKind, TemplateLookupStrategy,
};
pub use template_name_format::{
    Default020300 as NameFormatDefault020300, Default020400 as NameFormatDefault020400,
    TemplateNameFormat,
};
pub use template_source::TemplateSource;
pub use template_source_matcher::TemplateSourceMatcher;
pub use url_template_loader::URLTemplateLoader;
pub use url_template_source::URLTemplateSource;
