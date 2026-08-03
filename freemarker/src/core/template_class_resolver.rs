//! `?new` 类解析器 —— 对应 Java `freemarker.core.TemplateClassResolver`
//! （接口 + UNRESTRICTED/SAFER/ALLOWS_NOTHING 三个常量实现，TemplateClassResolver.java）
//! 与 `freemarker.core.OptInTemplateClassResolver`（allowed_classes + trusted_templates）。
//!
//! 设置字符串解析对应 Configurable.setSetting 的 NEW_BUILTIN_CLASS_RESOLVER 分支
//! （Configurable.java:2837-2876："unrestricted"/"safer"/"allows_nothing" 精确名；
//! 含 `:` 的值走 SettingStringParser.parseAsSegmentedList 的分段列表——`key: v1, v2`
//! 中带 `:` 的项开启新段，其余项追加当前段）。
//!
//! `resolve` 只做权限判定（Java resolve 的类加载部分由
//! `utility_transforms::new_utility_class` 承接——本引擎无 Java 类加载器；
//! 放行后未知类仍按 ClassNotFoundException 语义报错，与 Java ClassUtil.forName 一致）。

use crate::error::{Result, TemplateError};

/// `?new` 类解析策略 —— 对应 Java `TemplateClassResolver` 接口的三个常量实现 +
/// `OptInTemplateClassResolver`（Rust 特有枚举形态，见 docs/02 §4.1 角色槽位设计）
#[derive(Debug, Clone, PartialEq)]
pub enum NewBuiltinClassResolver {
    /// 不设限 —— Java `TemplateClassResolver.UNRESTRICTED_RESOLVER`
    /// （ClassUtil.forName 直通；Configurable.java:477 默认值）
    Unrestricted,
    /// 仅禁可执行任意代码的类 —— Java `SAFER_RESOLVER`（TemplateClassResolver.java:62-73）：
    /// ObjectConstructor / Execute / JythonRuntime
    Safer,
    /// 全部拒绝 —— Java `ALLOWS_NOTHING_RESOLVER`（TemplateClassResolver.java:75-82）
    AllowsNothing,
    /// 白名单 + 信任模板 —— Java `OptInTemplateClassResolver`
    OptIn(OptInClassResolver),
}

/// 对应 Java `OptInTemplateClassResolver`：allowed_classes（全限定类名集合）+
/// trusted_templates（模板名或 `*` 结尾前缀；前导 "/" 剥离）。
/// 信任模板内走 SAFER 策略，其余模板的类必须 ∈ allowed_classes
/// （OptInTemplateClassResolver.java:69-87）。
#[derive(Debug, Clone, PartialEq)]
pub struct OptInClassResolver {
    /// 允许的类全限定名（Java allowedClasses Set，构造 :44-66）
    allowed_classes: Vec<String>,
    /// 精确模板名（Java trustedTemplateNames Set）
    trusted_template_names: Vec<String>,
    /// 前缀模式（`foo*` 匹配 `foobar`/`foo/bar/baaz` 等；Java trustedTemplatePrefixes）
    trusted_template_prefixes: Vec<String>,
}

impl OptInClassResolver {
    /// 对应 Java OptInTemplateClassResolver 构造（:44-66）：`*` 结尾 → 前缀模式
    /// （`foo*` → 前缀 `foo`）；前导 "/" 剥离；其余为精确名
    pub fn new(allowed_classes: Vec<String>, trusted_templates: Vec<String>) -> Self {
        let mut names = Vec::new();
        let mut prefixes = Vec::new();
        for li in trusted_templates {
            let li = li.strip_prefix('/').unwrap_or(&li).to_string();
            if let Some(p) = li.strip_suffix('*') {
                prefixes.push(p.to_string());
            } else {
                names.push(li);
            }
        }
        OptInClassResolver {
            allowed_classes,
            trusted_template_names: names,
            trusted_template_prefixes: prefixes,
        }
    }

    fn resolve(&self, class_name: &str, template_name: Option<&str>) -> Result<()> {
        // Java OptInTemplateClassResolver.resolve（:69-87）：模板名（?new 词法所在
        // 模板，含 include 链）∈ trusted → SAFER 语义；否则须 ∈ allowed_classes
        if template_name
            .and_then(safe_template_name)
            .is_some_and(|n| self.is_trusted(&n))
        {
            return NewBuiltinClassResolver::Safer.resolve(class_name, None);
        }
        if !self.allowed_classes.iter().any(|c| c == class_name) {
            return Err(instantiating_not_allowed(
                class_name,
                true, // Java OptIn 分支的消息含设置名提示（OptInTemplateClassResolver.java:79-84）
            ));
        }
        Ok(())
    }

    fn is_trusted(&self, name: &str) -> bool {
        if self.trusted_template_names.iter().any(|n| n == name) {
            return true;
        }
        self.trusted_template_prefixes
            .iter()
            .any(|p| name.starts_with(p.as_str()))
    }
}

impl NewBuiltinClassResolver {
    /// 设置字符串解析（Java Configurable.setSetting 的 NEW_BUILTIN_CLASS_RESOLVER
    /// 分支，Configurable.java:2837-2876）。无法识别的值 → Err
    /// （Java invalidSettingValueException 语义）
    pub fn parse(value: &str) -> Result<Self> {
        let v = value.trim();
        match v {
            "unrestricted" => return Ok(NewBuiltinClassResolver::Unrestricted),
            "safer" => return Ok(NewBuiltinClassResolver::Safer),
            "allows_nothing" | "allowsNothing" => {
                return Ok(NewBuiltinClassResolver::AllowsNothing)
            }
            _ => {}
        }
        if v.contains(':') {
            // Java SettingStringParser.parseAsSegmentedList（Configurable.java:3259-3290）：
            // 分段列表 `key: v1, v2, key2: v3` —— 含 `:` 的项开启新段（键），
            // 其余项追加当前段的值列表；键支持 snake_case/camelCase 双写法
            let mut allowed: Vec<String> = Vec::new();
            let mut trusted: Vec<String> = Vec::new();
            // 当前段归属（Java currentSegment 的等价物）
            let mut segment_allowed = true;
            let mut started = false;
            for token in v.split(',') {
                let token = token.trim();
                if token.is_empty() {
                    continue;
                }
                if let Some((key, val)) = token.split_once(':') {
                    let key = key.trim();
                    let val = val.trim();
                    match key {
                        "allowed_classes" | "allowedClasses" => {
                            allowed.push(val.to_string());
                            segment_allowed = true;
                        }
                        "trusted_templates" | "trustedTemplates" => {
                            trusted.push(val.to_string());
                            segment_allowed = false;
                        }
                        other => {
                            return Err(TemplateError::misc(format!(
                                "Unrecognized list segment key: {other:?}. Supported keys are: \
                                 \"allowed_classes\", \"allowedClasses\", \"trusted_templates\", \
                                 \"trustedTemplates\". "
                            )));
                        }
                    }
                    started = true;
                } else if started {
                    if segment_allowed {
                        allowed.push(token.to_string());
                    } else {
                        trusted.push(token.to_string());
                    }
                } else {
                    // Java "The very first list item must be followed by \":\" ..."（:3270-3273）
                    return Err(TemplateError::misc(
                        "The very first list item must be followed by \":\" so it will be the \
                         key for the following sub-list.",
                    ));
                }
            }
            return Ok(NewBuiltinClassResolver::OptIn(OptInClassResolver::new(
                allowed, trusted,
            )));
        }
        Err(TemplateError::misc(format!(
            "Invalid setting value for new_builtin_class_resolver: {v:?}"
        )))
    }

    /// 权限判定（Java `TemplateClassResolver.resolve`）：拒绝 → Err（错误消息对齐
    /// `_MessageUtil.newInstantiatingClassNotAllowedException`，_MessageUtil.java:301-304）；
    /// 放行 → Ok（类加载/实例化由调用方继续）
    pub fn resolve(&self, class_name: &str, template_name: Option<&str>) -> Result<()> {
        match self {
            NewBuiltinClassResolver::Unrestricted => Ok(()),
            NewBuiltinClassResolver::Safer => {
                // Java SAFER_RESOLVER（TemplateClassResolver.java:62-73）：禁
                // ObjectConstructor / Execute / JythonRuntime（可执行任意代码的类）
                if matches!(
                    class_name,
                    "freemarker.template.utility.ObjectConstructor"
                        | "freemarker.template.utility.Execute"
                        | "freemarker.template.utility.JythonRuntime"
                ) {
                    return Err(instantiating_not_allowed(class_name, false));
                }
                Ok(())
            }
            NewBuiltinClassResolver::AllowsNothing => {
                Err(instantiating_not_allowed(class_name, false))
            }
            NewBuiltinClassResolver::OptIn(opt) => opt.resolve(class_name, template_name),
        }
    }
}

/// 拒绝消息 —— 对应 Java `_MessageUtil.newInstantiatingClassNotAllowedException`
/// （_MessageUtil.java:301-304）："Instantiating {className} is not allowed in the
/// template for security reasons."。optin=true 时追加
/// OptInTemplateClassResolver 的设置名提示（:79-84）
fn instantiating_not_allowed(class_name: &str, optin: bool) -> TemplateError {
    let msg = if optin {
        format!(
            "Instantiating {class_name} is not allowed in the template for security reasons. \
             (If you run into this problem when using ?new in a template, you may want to \
             check the \"new_builtin_class_resolver\" setting in the FreeMarker configuration.)"
        )
    } else {
        format!("Instantiating {class_name} is not allowed in the template for security reasons.")
    };
    TemplateError::misc(msg)
}

/// 模板名净化 —— 对应 Java `OptInTemplateClassResolver.safeGetTemplateName`（:88-124）：
/// 前导 "/" 剥离；URL 解码 %2e/%2f/%5c 后做路径段 `..` 检测（命中 → None = 不信任，
/// 防目录穿越绕过白名单）
fn safe_template_name(name: &str) -> Option<String> {
    let decoded = if name.contains('%') {
        name.replace("%2e", ".")
            .replace("%2E", ".")
            .replace("%2f", "/")
            .replace("%2F", "/")
            .replace("%5c", "\\")
            .replace("%5C", "\\")
    } else {
        name.to_string()
    };
    let b = decoded.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'.' && i + 1 < b.len() && b[i + 1] == b'.' {
            let before = if i == 0 { -1 } else { b[i - 1] as i32 };
            let after = if i + 2 >= b.len() {
                -1
            } else {
                b[i + 2] as i32
            };
            if (before == -1 || before == b'/' as i32 || before == b'\\' as i32)
                && (after == -1 || after == b'/' as i32 || after == b'\\' as i32)
            {
                return None;
            }
        }
        i += 1;
    }
    Some(decoded.strip_prefix('/').unwrap_or(&decoded).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_resolvers() {
        assert_eq!(
            NewBuiltinClassResolver::parse("unrestricted").unwrap(),
            NewBuiltinClassResolver::Unrestricted
        );
        assert_eq!(
            NewBuiltinClassResolver::parse("safer").unwrap(),
            NewBuiltinClassResolver::Safer
        );
        assert_eq!(
            NewBuiltinClassResolver::parse("allows_nothing").unwrap(),
            NewBuiltinClassResolver::AllowsNothing
        );
        assert_eq!(
            NewBuiltinClassResolver::parse("allowsNothing").unwrap(),
            NewBuiltinClassResolver::AllowsNothing
        );
        assert!(NewBuiltinClassResolver::parse("bogus").is_err());
    }

    #[test]
    fn parse_optin_segmented_list() {
        // Java 模板套件 new-optin 用例的设置串（manifest 原文，含缩进空白）
        let r = NewBuiltinClassResolver::parse(
            "         allowed_classes: freemarker.test.templatesuite.models.NewTestModel, \
             trusted_templates: subdir/new-optin.ftl, subdir/subsub/*",
        )
        .unwrap();
        let NewBuiltinClassResolver::OptIn(opt) = &r else {
            panic!("expected OptIn");
        };
        assert_eq!(
            opt.allowed_classes,
            vec!["freemarker.test.templatesuite.models.NewTestModel"]
        );
        assert_eq!(opt.trusted_template_names, vec!["subdir/new-optin.ftl"]);
        assert_eq!(opt.trusted_template_prefixes, vec!["subdir/subsub/"]);
    }

    #[test]
    fn safer_rejects_object_constructor() {
        let r = NewBuiltinClassResolver::parse("safer").unwrap();
        assert!(r
            .resolve("freemarker.template.utility.ObjectConstructor", None)
            .is_err());
        assert!(r
            .resolve("freemarker.test.templatesuite.models.NewTestModel", None)
            .is_ok());
        let err = r
            .resolve("freemarker.template.utility.ObjectConstructor", None)
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("is not allowed in the template for security reasons"));
    }

    #[test]
    fn allows_nothing_rejects_all() {
        let r = NewBuiltinClassResolver::parse("allows_nothing").unwrap();
        assert!(r
            .resolve("freemarker.test.templatesuite.models.NewTestModel", None)
            .is_err());
    }

    #[test]
    fn optin_trusted_template_matching() {
        let r = NewBuiltinClassResolver::parse(
            "allowed_classes: a.B, trusted_templates: subdir/x.ftl, subdir/subsub/*",
        )
        .unwrap();
        // 非信任模板：白名单外类拒绝
        assert!(r.resolve("a.B", Some("main.ftl")).is_ok());
        assert!(r.resolve("a.C", Some("main.ftl")).is_err());
        // 精确信任名 → SAFER 语义（非 ObjectConstructor 放行）
        assert!(r.resolve("a.C", Some("subdir/x.ftl")).is_ok());
        // 前缀模式 `subdir/subsub/*`（Java 前缀匹配：startsWith）
        assert!(r.resolve("a.C", Some("subdir/subsub/deep.ftl")).is_ok());
        assert!(r.resolve("a.C", Some("subdir/subsub2/x.ftl")).is_err());
        // 信任模板内 ObjectConstructor 仍被 SAFER 拒绝
        assert!(r
            .resolve(
                "freemarker.template.utility.ObjectConstructor",
                Some("subdir/x.ftl")
            )
            .is_err());
        // 模板名缺失 → 不信任
        assert!(r.resolve("a.C", None).is_err());
    }
}
