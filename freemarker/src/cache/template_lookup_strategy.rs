//! 模板查找策略 —— 对应 Java `freemarker.cache.TemplateLookupStrategy`
//! （Default020300.lookup :101-113 → TemplateCache.lookupWithLocalizedThenAcquisitionStrategy
//!   :914-942 与 lookupTemplateWithAcquisitionStrategy :740-781）
//! LookupResult / FindFn 分别见 template_lookup_result.rs / template_lookup_context.rs
//! （一文件一 Java 对象）

use crate::cache::template_lookup_context::FindFn;
use crate::cache::template_lookup_result::LookupResult;
use crate::error::Result;

/// 查找策略种类（v1 用 enum 表达策略选择；对应 Configuration 的
///   templateLookupStrategy 设置，见 docs/07 §2 :63）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LookupStrategyKind {
    /// 对应 `TemplateLookupStrategy.DEFAULT_2_3_0`（Java:80）：本地化回退 + acquisition
    #[default]
    Default020300,
}

/// 查找策略 trait（对应 TemplateLookupStrategy 抽象类；`lookup` 对应 Java:99）
pub trait TemplateLookupStrategy: Send + Sync {
    /// 对应 `lookup(TemplateLookupContext)`（Java:99）：
    /// 按策略尝试候选名，返回首个命中的源；全部 miss → Ok(None)。
    /// - `locale`：Some 表示启用局部化回退（Java: TemplateCache.java:897-899，
    ///   localizedLookup 关闭时传 None）
    /// - `find`：每次调用对应一次 TemplateLoader.findTemplateSource
    ///   （acquisition 会多次调用；Java 经 TemplateLookupContext 提供）
    fn lookup(
        &self,
        name: &str,
        locale: Option<&str>,
        find: FindFn<'_>,
    ) -> Result<Option<LookupResult>>;
}

/// Default020300 策略（对应 `TemplateLookupStrategy.DEFAULT_2_3_0`，Java:101-113）
pub struct Default020300;

impl TemplateLookupStrategy for Default020300 {
    /// 对应 Java:104-106：`lookupWithLocalizedThenAcquisitionStrategy(name, locale)`
    fn lookup(
        &self,
        name: &str,
        locale: Option<&str>,
        find: FindFn<'_>,
    ) -> Result<Option<LookupResult>> {
        match locale {
            // 局部化关闭（Java:916-919）
            None => lookup_with_acquisition(name, find),
            Some(loc) => {
                // Java:927-928 —— 拆前缀与扩展名（最后一个 "." 之后为扩展名）
                let last_dot = name.rfind('.');
                let (prefix, suffix) = match last_dot {
                    Some(i) => (&name[..i], &name[i..]),
                    None => (name, ""),
                };
                // Java:929-931 —— locale 变体："_" + locale.toString()，从尾部逐级去掉变体
                let mut locale_name = format!("_{}", loc);
                loop {
                    let candidate = format!("{}{}{}", prefix, locale_name, suffix);
                    if let Some(r) = lookup_with_acquisition(&candidate, find)? {
                        return Ok(Some(r));
                    }
                    if locale_name.is_empty() {
                        break;
                    }
                    match locale_name.rfind('_') {
                        // Java:939-943 —— 去掉尾部变体（"_en_US"→"_en"）；仅剩前导 "_"
                        // 时清空 → 尝试基础名（"foo.ftl"），与 Java 每次截到 substring(0, i) 一致
                        Some(i) if i > 0 => {
                            locale_name.truncate(i);
                        }
                        _ => locale_name.clear(),
                    }
                }
                Ok(None)
            }
        }
    }
}

/// 对应 `lookupTemplateWithAcquisitionStrategy`（TemplateCache.java:740-781）：
/// 处理 "*" 步骤（acquisition）；无 "*" 则直接查找
fn lookup_with_acquisition(path: &str, find: FindFn<'_>) -> Result<Option<LookupResult>> {
    // Java:770-772 —— 无 "*" 快捷路径
    if !path.contains('*') {
        return find_source(path, find);
    }
    // Java:773-785 —— 分词；连续 "*" 步骤折叠为一个（保留最后一个）
    let mut tokens: Vec<&str> = path.split('/').collect();
    let mut last_asterisk: Option<usize> = None;
    let mut i = 0usize;
    while i < tokens.len() {
        if tokens[i] == "*" {
            if let Some(prev) = last_asterisk {
                tokens.remove(prev); // Java:781-782 —— 移除前一个 "*"
                i -= 1;
            }
            last_asterisk = Some(i);
        }
        i += 1;
    }
    let Some(la) = last_asterisk else {
        // Java:786-788 —— 无真正的 "*" 步骤（如 "*.ftl"）
        return find_source(path, find);
    };
    // Java:790-792 —— basePath = "*" 之前的步骤（含尾部 "/"），resourcePath 为之后的部分
    let base_path = if la == 0 {
        String::new()
    } else {
        format!("{}/", tokens[..la].join("/"))
    };
    let mut resource_path = tokens[la + 1..].join("/");
    // Java:793-795 —— 去掉末尾 "/"
    if resource_path.ends_with('/') {
        resource_path.pop();
    }
    // Java:796-800 —— 逐级回退：先试完整 basePath+resourcePath，再逐级去掉 basePath 尾部步骤
    let mut attempt = base_path.clone();
    let mut l = base_path.len();
    loop {
        let full = format!("{}{}", attempt, resource_path);
        if let Some(r) = find_source(&full, find)? {
            return Ok(Some(r));
        }
        if l == 0 {
            return Ok(None);
        }
        // Java:799 —— l = basePath.lastIndexOf('/', l - 2) + 1
        l = base_path.as_bytes()[..l.saturating_sub(1)]
            .iter()
            .rposition(|&b| b == b'/')
            .map_or(0, |i| i + 1);
        attempt.truncate(l);
    }
}

/// 单次查找 + 结果包装（Java:771-772/787-788 的 findTemplateSource 调用点）
fn find_source(name: &str, find: FindFn<'_>) -> Result<Option<LookupResult>> {
    Ok(find(name)?.map(|source| LookupResult {
        source_name: name.to_string(),
        source,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::cache::TemplateLoader;
    use crate::cache::TemplateSource;

    fn strategy_find(
        loader: &StringLoader,
    ) -> impl FnMut(&str) -> Result<Option<Box<dyn TemplateSource>>> + '_ {
        |name: &str| loader.find(name)
    }

    #[test]
    fn localized_fallback_order() {
        let loader = StringLoader::default();
        loader.put("foo.ftl", "base");
        loader.put("foo_en.ftl", "english");
        loader.put("bar.ftl", "bar");
        let s = Default020300;

        // 最具体的本地化变体优先（Java 文档示例 :65-71）
        let mut find = strategy_find(&loader);
        let r = s
            .lookup("foo.ftl", Some("en_US"), &mut find)
            .unwrap()
            .expect("本地化变体命中");
        assert_eq!(r.source_name, "foo_en.ftl");
        assert_eq!(loader.read(&*r.source).unwrap(), "english");

        // 无对应变体 → 回退基础名（变体链 "_de" → ""）
        let mut find = strategy_find(&loader);
        let r = s
            .lookup("bar.ftl", Some("de"), &mut find)
            .unwrap()
            .expect("回退基础名");
        assert_eq!(r.source_name, "bar.ftl");

        // 全部 miss → None
        let mut find = strategy_find(&loader);
        assert!(s
            .lookup("nope.ftl", Some("de"), &mut find)
            .unwrap()
            .is_none());

        // 局部化关闭（locale=None）→ 直接查基础名
        let mut find = strategy_find(&loader);
        let r = s
            .lookup("foo.ftl", None, &mut find)
            .unwrap()
            .expect("直接命中");
        assert_eq!(r.source_name, "foo.ftl");
        assert_eq!(loader.read(&*r.source).unwrap(), "base");
    }

    #[test]
    fn acquisition_steps() {
        let loader = StringLoader::default();
        loader.put("a/b/c.ftl", "nested");
        loader.put("c.ftl", "root");
        let s = Default020300;

        // "a/*/c.ftl"：先试 "a/c.ftl"（无）→ 回退到 "c.ftl"（Java:796-800）
        let mut find = strategy_find(&loader);
        let r = s
            .lookup("a/*/c.ftl", None, &mut find)
            .unwrap()
            .expect("acquisition 命中");
        assert_eq!(r.source_name, "c.ftl");
        assert_eq!(loader.read(&*r.source).unwrap(), "root");

        // 较浅目录存在时优先（"a/c.ftl" 在 "c.ftl" 之前被尝试）
        loader.put("a/c.ftl", "shallow");
        let mut find = strategy_find(&loader);
        let r = s
            .lookup("a/*/c.ftl", None, &mut find)
            .unwrap()
            .expect("较浅目录命中");
        assert_eq!(r.source_name, "a/c.ftl");
        assert_eq!(loader.read(&*r.source).unwrap(), "shallow");

        // 无 "*" → 直接查找
        let mut find = strategy_find(&loader);
        let r = s
            .lookup("a/b/c.ftl", None, &mut find)
            .unwrap()
            .expect("直接命中");
        assert_eq!(r.source_name, "a/b/c.ftl");

        // 完全 miss → None
        let mut find = strategy_find(&loader);
        assert!(s.lookup("x/*/y.ftl", None, &mut find).unwrap().is_none());
    }

    #[test]
    fn acquisition_with_localized_combined() {
        let loader = StringLoader::default();
        loader.put("msg_de.ftl", "de");
        let s = Default020300;
        // 本地化变体命中后不再尝试 acquisition 之外的名字
        let mut find = strategy_find(&loader);
        let r = s
            .lookup("msg.ftl", Some("de_DE"), &mut find)
            .unwrap()
            .expect("变体 _de 命中");
        assert_eq!(r.source_name, "msg_de.ftl");
    }
}
