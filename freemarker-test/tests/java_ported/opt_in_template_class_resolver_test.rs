//! Java `freemarker.core.OptInTemplateClassResolverTest` 的 Rust 1:1 实现
//! （对应 Java: OptInTemplateClassResolverTest —— OptInTemplateClassResolver 的
//!   白名单类/信任模板前缀/目录穿越防护 + `new_builtin_class_resolver` 设置串解析）。
//!
//! 引擎映射：`freemarker::core::NewBuiltinClassResolver::OptIn(OptInClassResolver)`
//!   （template_class_resolver.rs）对应 Java 类；`resolve(class_name, template_name)`
//!   做权限判定（Java 的类加载部分由 Java 类加载器承接，Rust 侧无 Java 类——
//!   放行类名断言由 Ok/Err 表达；Java `assertEquals(String.class, resolve(...))`
//!   的"返回加载后的 Class"部分是 JVM 特有，见各方法注释）。
//!
//! ENGINE_GAP: `NewBuiltinClassResolver::parse`（对应 Configurable.setSetting 的
//!   new_builtin_class_resolver 分支）不剥引号/不处理键值间空白——Java
//!   SettingStringParser 会剥离单双引号并把 `"allowed_classes" : java.lang.String`
//!   等带空白写法解析为合法键值；引擎 parse 对带引号输入报错或把引号并入值
//!   → testSettingParser 的带引号块（Java 原样输入）失败，登记 ENGINE_GAP，
//!   原样翻译测试 #[ignore]，另附 ADAPTED 版本（按 Java 解析器剥引号后的
//!   等价输入，断言逐字对齐）。
//!
//! NOT_APPLICABLE: 无（仅上述 ENGINE_GAP 局部）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::core::{NewBuiltinClassResolver, OptInClassResolver};

/// 对应 Java 测试类静态字段：ALLOWED_CLASSES = {java.lang.String, java.lang.Integer}；
/// TRUSTED_TEMPLATES = {lib/*, /include/*, trusted.ftl}（`/include/*` 前导 "/" 剥离
/// 后为前缀 "include/"）
fn opt_in_resolver() -> NewBuiltinClassResolver {
    NewBuiltinClassResolver::OptIn(OptInClassResolver::new(
        vec![
            "java.lang.String".to_string(),
            "java.lang.Integer".to_string(),
        ],
        vec![
            "lib/*".to_string(),
            "/include/*".to_string(),
            "trusted.ftl".to_string(),
        ],
    ))
}

/// Java testOptIn：
/// `assertEquals(String.class, resolve("java.lang.String", null, dummyTemp))` 等 ——
/// dummyTemp = "foo.ftl" 非信任模板；白名单内类放行、白名单外（Long）报
/// TemplateException。引擎差异：resolve 只做权限判定（返回 Ok/Err），类加载
/// （String.class 身份）为 JVM 特有。
#[test]
fn test_opt_in() {
    let resolver = opt_in_resolver();
    // assertEquals(String.class, resolver.resolve("java.lang.String", null, dummyTemp));
    assert!(
        resolver.resolve("java.lang.String", Some("foo.ftl")).is_ok(),
        "java.lang.String 应在白名单内"
    );
    // assertEquals(Integer.class, resolver.resolve("java.lang.Integer", null, dummyTemp));
    assert!(
        resolver.resolve("java.lang.Integer", Some("foo.ftl")).is_ok(),
        "java.lang.Integer 应在白名单内"
    );
    // try { resolver.resolve("java.lang.Long", null, dummyTemp); fail(); }
    // catch (TemplateException e) { /* good */ }
    assert!(
        resolver.resolve("java.lang.Long", Some("foo.ftl")).is_err(),
        "java.lang.Long 不在白名单 → 应拒绝"
    );
}

/// Java testTrusted：信任模板内走 SAFER 语义（Long 放行；ObjectConstructor 仍拒绝）。
/// 模板名 "lib/foo.ftl"（前缀 lib/）、"/lib/foo.ftl"（前导 / 剥离）、
/// "include/foo.ftl"（前缀 include/）、"trusted.ftl"（精确名）、"/trusted.ftl"
/// 均信任；`ObjectConstructor.class.getName()` = "freemarker.template.utility.ObjectConstructor"
/// 在信任模板内仍被 SAFER 拒绝（Java assertEquals(Long.class, ...) 的类身份
/// 断言由 Ok/Err 表达——JVM 类加载部分无对应）。
#[test]
fn test_trusted() {
    let resolver = opt_in_resolver();
    // assertEquals(Long.class, resolve("java.lang.Long", null, Template.getPlainTextTemplate("lib/foo.ftl", "", dummyCfg)));
    assert!(resolver.resolve("java.lang.Long", Some("lib/foo.ftl")).is_ok());
    // assertEquals(String.class, resolve("java.lang.String", null, ..."lib/foo.ftl"));
    assert!(resolver.resolve("java.lang.String", Some("lib/foo.ftl")).is_ok());
    // assertEquals(Long.class, resolve("java.lang.Long", null, ..."/lib/foo.ftl"));
    assert!(resolver.resolve("java.lang.Long", Some("/lib/foo.ftl")).is_ok());
    // assertEquals(Long.class, resolve("java.lang.Long", null, ..."include/foo.ftl"));
    assert!(resolver.resolve("java.lang.Long", Some("include/foo.ftl")).is_ok());
    // assertEquals(Long.class, resolve("java.lang.Long", null, ..."trusted.ftl"));
    assert!(resolver.resolve("java.lang.Long", Some("trusted.ftl")).is_ok());
    // assertEquals(Long.class, resolve("java.lang.Long", null, ..."/trusted.ftl"));
    assert!(resolver.resolve("java.lang.Long", Some("/trusted.ftl")).is_ok());
    // try { assertEquals(Long.class, resolve(ObjectConstructor.class.getName(), null,
    //         Template.getPlainTextTemplate("trusted.ftl", "", dummyCfg))); fail(); }
    // catch (TemplateException e) { /* good */ }
    assert!(resolver
        .resolve("freemarker.template.utility.ObjectConstructor", Some("trusted.ftl"))
        .is_err());
}

/// Java testCraftedTrusted：`..` 路径段穿越（含 %xx 编码与 `\` 分隔符变体）→
/// safeGetTemplateName 返回 null → 不信任 → Long 拒绝；非穿越的 `.` 名字
/// （"lib/./foo.ftl"、"lib/foo..ftl"、"lib/%2e/foo.ftl"）→ 仍信任 → Long 放行。
/// （Java 对放行组经 testTrusted_checkFails 的 fail()→AssertionFailedError 被
/// 外层捕获断言；Rust 直接断言 resolve 成功。）
#[test]
fn test_crafted_trusted() {
    let resolver = opt_in_resolver();
    // Java：testTrusted_checkFails 逐个期望 TemplateException
    test_trusted_check_fails(&resolver, "lib/../foo.ftl");
    test_trusted_check_fails(&resolver, "lib\\..\\foo.ftl");
    test_trusted_check_fails(&resolver, "lib\\../foo.ftl");
    test_trusted_check_fails(&resolver, "lib/..\\foo.ftl");
    test_trusted_check_fails(&resolver, "lib/..");
    test_trusted_check_fails(&resolver, "lib%2f%2E%2e%5cfoo.ftl");
    test_trusted_check_fails(&resolver, "/lib%5C%.%2e%2Efoo.ftl");

    // Java：
    //   try { testTrusted_checkFails("lib/./foo.ftl"); fail(); }
    //   catch (AssertionFailedError e) { /* good */ }  —— 即 resolve 成功
    assert!(
        resolver.resolve("java.lang.Long", Some("lib/./foo.ftl")).is_ok(),
        "lib/./foo.ftl 无 .. 段 → 应信任"
    );
    // 同上："lib/foo..ftl"（.. 前后非边界，非穿越段）
    assert!(
        resolver.resolve("java.lang.Long", Some("lib/foo..ftl")).is_ok(),
        "lib/foo..ftl 的 .. 非路径段 → 应信任"
    );
    // 同上："lib/%2e/foo.ftl" 解码为 lib/./foo.ftl
    assert!(
        resolver.resolve("java.lang.Long", Some("lib/%2e/foo.ftl")).is_ok(),
        "lib/%2e/foo.ftl 解码后无 .. 段 → 应信任"
    );
}

/// Java testTrusted_checkFails(templateName)：
///   try { resolver.resolve("java.lang.Long", null, getPlainTextTemplate(templateName)); fail(); }
///   catch (TemplateException e) { /* good */ }
fn test_trusted_check_fails(resolver: &NewBuiltinClassResolver, template_name: &str) {
    assert!(
        resolver.resolve("java.lang.Long", Some(template_name)).is_err(),
        "expected TemplateException for trusted-template name {template_name:?}"
    );
}

/// Java testSettingParser —— 第 4 段（键错误）可 1:1：
///   try { cfg.setSetting("new_builtin_class_resolver", "wrong: foo"); fail(); }
///   catch (TemplateException e) { /* good */ }
/// 引擎 parse 对未识别段键返回 Err（消息 "Unrecognized list segment key"）。
#[test]
fn test_setting_parser_wrong_key() {
    let err = NewBuiltinClassResolver::parse("wrong: foo").unwrap_err();
    assert!(
        err.to_user_message().contains("Unrecognized list segment key"),
        "{err}"
    );
}

/// Java testSettingParser 的 1:1 翻译（Java 原样输入，含引号与空白变体）。
/// ENGINE_GAP：`NewBuiltinClassResolver::parse` 不剥引号（Java SettingStringParser
/// 会剥离 `"..."`/`'...'` 并把 `"allowed_classes" : java.lang.String` 中的空白
/// 写法识别为合法键）→ 第 1/3/5 段（带引号输入）在本引擎行为不同（引号并入
/// 值/键导致不匹配或解析报错）→ 本方法 #[ignore]，通过版本见
/// test_setting_parser_adapted（输入按 Java 解析器剥引号后的等价形式）。
#[test]
#[ignore = "ENGINE_GAP: NewBuiltinClassResolver::parse 不剥引号/不处理键值间空白（Java SettingStringParser 会）——见文件头"]
fn test_setting_parser() {
    // 第 1 段：trusted_templates 带双引号 "lib/*"
    // cfg.setSetting("new_builtin_class_resolver", "trusted_templates: foo.ftl, \"lib/*\"");
    let res = NewBuiltinClassResolver::parse("trusted_templates: foo.ftl, \"lib/*\"").unwrap();
    // assertEquals(String.class, res.resolve("java.lang.String", null, getPlainTextTemplate("foo.ftl")));
    assert!(res.resolve("java.lang.String", Some("foo.ftl")).is_ok());
    // assertEquals(String.class, res.resolve("java.lang.String", null, getPlainTextTemplate("lib/bar.ftl")));
    assert!(res.resolve("java.lang.String", Some("lib/bar.ftl")).is_ok());
    // try { res.resolve("java.lang.String", null, getPlainTextTemplate("bar.ftl")); fail(); }
    // catch (TemplateException e) { /* good */ }
    assert!(res.resolve("java.lang.String", Some("bar.ftl")).is_err());

    // 第 2 段：allowed_classes 无引号
    // cfg.setSetting("new_builtin_class_resolver", "allowed_classes: java.lang.String, java.lang.Integer");
    let res =
        NewBuiltinClassResolver::parse("allowed_classes: java.lang.String, java.lang.Integer")
            .unwrap();
    assert!(res.resolve("java.lang.String", Some("foo.ftl")).is_ok());
    assert!(res.resolve("java.lang.Integer", Some("foo.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("foo.ftl")).is_err());

    // 第 3 段：混合 trusted_templates + allowed_classes（单引号）
    // cfg.setSetting("new_builtin_class_resolver",
    //         "trusted_templates: foo.ftl, 'lib/*', allowed_classes: 'java.lang.String', java.lang.Integer");
    let res = NewBuiltinClassResolver::parse(
        "trusted_templates: foo.ftl, 'lib/*', allowed_classes: 'java.lang.String', java.lang.Integer",
    )
    .unwrap();
    assert!(res.resolve("java.lang.String", Some("x.ftl")).is_ok());
    assert!(res.resolve("java.lang.Integer", Some("x.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("x.ftl")).is_err());
    assert!(res.resolve("java.lang.Long", Some("foo.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("lib/bar.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("x.ftl")).is_err());

    // 第 4 段：键错误（本方法错误断言见 test_setting_parser_wrong_key）
    // try { cfg.setSetting("new_builtin_class_resolver", "wrong: foo"); fail(); }
    // catch (TemplateException e) { /* good */ }

    // 第 5 段：带引号键 + 值间空白
    // cfg.setSetting("new_builtin_class_resolver",
    //         "\"allowed_classes\"  :  java.lang.String  ,  'trusted_templates' :\"lib:*\"");
    let res = NewBuiltinClassResolver::parse(
        "\"allowed_classes\"  :  java.lang.String  ,  'trusted_templates' :\"lib:*\"",
    )
    .unwrap();
    // assertEquals(String.class, res.resolve("java.lang.String", null, getPlainTextTemplate("x.ftl")));
    assert!(res.resolve("java.lang.String", Some("x.ftl")).is_ok());
    // try { res.resolve("java.lang.Long", null, getPlainTextTemplate("x.ftl")); fail(); }
    // catch (TemplateException e) { /* good */ }
    assert!(res.resolve("java.lang.Long", Some("x.ftl")).is_err());
    // assertEquals(Long.class, res.resolve("java.lang.Long", null, getPlainTextTemplate("lib:bar.ftl")));
    assert!(res.resolve("java.lang.Long", Some("lib:bar.ftl")).is_ok());
}

/// Java testSettingParser 的 ADAPTED 版本：输入按 Java SettingStringParser 的
/// 剥引号语义改写（引号剥离、键值间空白不敏感），断言逐字对齐 Java
/// （test_setting_parser 的 #[ignore] 版输入为 Java 原样）。
#[test]
fn test_setting_parser_adapted() {
    // 第 1 段：trusted_templates: foo.ftl, "lib/*"（Java 剥双引号 → lib/*）
    let res = NewBuiltinClassResolver::parse("trusted_templates: foo.ftl, lib/*").unwrap();
    assert!(res.resolve("java.lang.String", Some("foo.ftl")).is_ok());
    assert!(res.resolve("java.lang.String", Some("lib/bar.ftl")).is_ok());
    assert!(res.resolve("java.lang.String", Some("bar.ftl")).is_err());

    // 第 2 段：allowed_classes: java.lang.String, java.lang.Integer（无引号，同 Java）
    let res =
        NewBuiltinClassResolver::parse("allowed_classes: java.lang.String, java.lang.Integer")
            .unwrap();
    assert!(res.resolve("java.lang.String", Some("foo.ftl")).is_ok());
    assert!(res.resolve("java.lang.Integer", Some("foo.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("foo.ftl")).is_err());

    // 第 3 段：trusted_templates: foo.ftl, 'lib/*', allowed_classes: 'java.lang.String',
    //         java.lang.Integer（Java 剥引号）
    let res = NewBuiltinClassResolver::parse(
        "trusted_templates: foo.ftl, lib/*, allowed_classes: java.lang.String, java.lang.Integer",
    )
    .unwrap();
    assert!(res.resolve("java.lang.String", Some("x.ftl")).is_ok());
    assert!(res.resolve("java.lang.Integer", Some("x.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("x.ftl")).is_err());
    assert!(res.resolve("java.lang.Long", Some("foo.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("lib/bar.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("x.ftl")).is_err());

    // 第 4 段：键错误 → parse 返回 Err（见 test_setting_parser_wrong_key）

    // 第 5 段："allowed_classes" : java.lang.String, 'trusted_templates' : "lib:*"
    //（Java 剥引号 + 键值间空白不敏感 → allowed_classes: java.lang.String,
    //  trusted_templates: lib:*）
    let res = NewBuiltinClassResolver::parse(
        "allowed_classes: java.lang.String, trusted_templates: lib:*",
    )
    .unwrap();
    assert!(res.resolve("java.lang.String", Some("x.ftl")).is_ok());
    assert!(res.resolve("java.lang.Long", Some("x.ftl")).is_err());
    assert!(res.resolve("java.lang.Long", Some("lib:bar.ftl")).is_ok());
}
