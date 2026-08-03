//! 命名空间前缀映射 —— 对应 Java `Template` 的 ns_prefixes（`<#ftl ns_prefixes=...>`）。

use std::collections::HashMap;

/// 命名空间前缀映射 —— 对应 Java `Template` 的 ns_prefixes（`<#ftl ns_prefixes=...>`）。
/// 特殊前缀："D"（`Template.DEFAULT_NAMESPACE_PREFIX`）注册为默认命名空间；
/// "N"（`Template.NO_NS_PREFIX`）保留、不可注册。
#[derive(Debug, Default)]
pub struct NsPrefixes {
    /// prefix → URI（不含 "D"——它进入 default_ns）
    prefix_to_uri: HashMap<String, String>,
    /// URI → prefix（getPrefixForNamespace 反查）
    uri_to_prefix: HashMap<String, String>,
    /// 默认命名空间（`D` 前缀）
    default_ns: Option<String>,
}

impl NsPrefixes {
    pub fn new(map: HashMap<String, String>) -> Self {
        let mut p = NsPrefixes {
            prefix_to_uri: HashMap::new(),
            uri_to_prefix: HashMap::new(),
            default_ns: None,
        };
        for (prefix, uri) in map {
            // Java Template.addNsPrefix（Template.java:920-951）："N" 保留非法；
            // "D" 注册为 defaultNS；其余入 prefixToNamespaceURILookup + 反查表
            // （同 URI 只能映射一个前缀，重复即非法——解析期已校验）
            if prefix == "N" {
                continue;
            }
            if prefix == "D" {
                p.default_ns = Some(uri);
            } else {
                p.uri_to_prefix.insert(uri.clone(), prefix.clone());
                p.prefix_to_uri.insert(prefix, uri);
            }
        }
        p
    }

    /// 默认命名空间 URI（Java Template.getDefaultNS）
    pub fn get_default_ns(&self) -> Option<&str> {
        self.default_ns.as_deref()
    }

    /// prefix → URI（Java Template.getNamespaceForPrefix："" → defaultNS 或 ""）
    pub fn get_namespace_for_prefix(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() {
            return Some(self.default_ns.as_deref().unwrap_or(""));
        }
        self.prefix_to_uri.get(prefix).map(|s| s.as_str())
    }

    /// URI → prefix（Java Template.getPrefixForNamespace：null → null；
    /// "" → defaultNS 为 null ? "" : "N"；== defaultNS → ""；否则反查表）
    pub fn get_prefix_for_namespace(&self, ns_uri: &str) -> Option<&str> {
        if ns_uri.is_empty() {
            return Some(if self.default_ns.is_none() { "" } else { "N" });
        }
        if self.default_ns.as_deref() == Some(ns_uri) {
            return Some("");
        }
        self.uri_to_prefix.get(ns_uri).map(|s| s.as_str())
    }
}
