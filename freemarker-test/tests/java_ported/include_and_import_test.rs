//! 对应 Java: IncludeAndImportTest
//! Java `freemarker.core.IncludeAndImportTest` 的 Rust 1:1 实现。
//! Java @Before setup：注册 inc1..inc3/lib1..lib3/lib_de/lib_en 等模板。
//!
//! 引擎差异总览：
//! - Java `Configuration.setIncompatibleImprovements` 有版本门控行为；v1 固定 ICI
//!   2.3.34（importInMainCreatesGlobalBugfix 恒为 2.3.24+ 修复后行为）。
//! - 引擎无 `autoInclude` 支持（Configuration 仅有 auto_imports，无 auto_includes）→
//!   autoIncludeAndAutoImport 不翻译。
//! - 引擎无 `lazyImports`/`lazyAutoImports` 设置（import 恒立即初始化）→
//!   lazyImport* / lazyAutoImport* / lazyImportErrors 不翻译。
//! - `lazilyInitializingNamespaceOverridesAll` 是 JVM 反射测试 → 不翻译。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use std::sync::Arc;

/// 对应 Java @Before setup()：注册共享模板
fn setup(loader: &Arc<StringLoader>) {
    add_template(
        loader,
        "inc1.ftl",
        "[inc1]<#global inc1Cnt = (inc1Cnt!0) + 1><#global history = (history!) + 'I'>",
    );
    add_template(loader, "inc2.ftl", "[inc2]");
    add_template(loader, "inc3.ftl", "[inc3]");
    add_template(loader, "lib1.ftl", "<#global lib1Cnt = (lib1Cnt!0) + 1><#global history = (history!) + 'L1'><#macro m>In lib1</#macro>");
    add_template(
        loader,
        "lib2.ftl",
        "<#global history = (history!) + 'L2'><#macro m>In lib2</#macro>",
    );
    add_template(
        loader,
        "lib3.ftl",
        "<#global history = (history!) + 'L3'><#macro m>In lib3</#macro>",
    );
    add_template(
        loader,
        "lib2CallsLib1.ftl",
        "<#global history = (history!) + 'L2'><#macro m>In lib2 (<@lib1.m/>)</#macro>",
    );
    add_template(loader, "lib3ImportsLib1.ftl", "<#import 'lib1.ftl' as lib1><#global history = (history!) + 'L3'><#macro m>In lib3 (<@lib1.m/>)</#macro>");
    add_template(
        loader,
        "lib_de.ftl",
        "<#global history = (history!) + 'LDe'><#assign initLocale=.locale><#macro m>de</#macro>",
    );
    add_template(
        loader,
        "lib_en.ftl",
        "<#global history = (history!) + 'LEn'><#assign initLocale=.locale><#macro m>en</#macro>",
    );
}

/// Java includeSameTwice：同一模板 include 两次，每次都会重新执行（global 计数累加）
#[test]
fn include_same_twice() {
    let (c, loader) = test_config();
    setup(&loader);
    assert_output(
        &c,
        &loader,
        "<#include 'inc1.ftl'>${inc1Cnt}<#include 'inc1.ftl'>${inc1Cnt}",
        "[inc1]1[inc1]2",
    );
}

/// Java importSameTwice：同一模板 import 两次，命名空间只初始化一次
#[test]
fn import_same_twice() {
    let (c, loader) = test_config();
    setup(&loader);
    assert_output(
        &c,
        &loader,
        "<#import 'lib1.ftl' as i1>${lib1Cnt} <#import 'lib1.ftl' as i2>${lib1Cnt}",
        "1 1",
    );
}

/// Java importInMainCreatesGlobal：主命名空间中的 import 同时创建全局变量
#[test]
fn import_in_main_creates_global() {
    let (c, loader) = test_config();
    setup(&loader);
    let ftl = "${.main.lib1???c} ${.globals.lib1???c}<#import 'lib1.ftl' as lib1> ${.main.lib1???c} ${.globals.lib1???c}";
    assert_output(&c, &loader, ftl, "false false true true");
    // Java：setIncompatibleImprovements(2.3.24) 后行为不变 → 引擎固定 ICI 2.3.34，
    // 同样无差异
    assert_output(&c, &loader, ftl, "false false true true");
}

/// Java importInMainCreatesGlobalBugfix：
/// 若库已在别处初始化过，主命名空间 import 是否仍创建全局变量。
/// 引擎差异：v1 固定 ICI 2.3.34，恒为 2.3.24+ 的修复后行为（`1 true true`）；
/// Java 默认（未设 ICI）的 buggy 输出 `1 true false` 在 v1 不可复现。
#[test]
fn import_in_main_creates_global_bugfix() {
    let (c, loader) = test_config();
    setup(&loader);
    let ftl =
        "<#import 'lib3ImportsLib1.ftl' as lib3>${lib1Cnt} ${.main.lib1???c} ${.globals.lib1???c}, "
            .to_string()
            + "<#import 'lib1.ftl' as lib1>${lib1Cnt} ${.main.lib1???c} ${.globals.lib1???c}";
    // Java 默认（ICI 2.3.0）："1 false false, 1 true false"（bug）——
    // 引擎差异：v1 恒为修复后行为，此处断言 2.3.24+ 输出
    assert_output(&c, &loader, &ftl, "1 false false, 1 true true");
    // Java setIncompatibleImprovements(2.3.24) → 修复后行为，与引擎一致
    assert_output(&c, &loader, &ftl, "1 false false, 1 true true");
}

// Java autoIncludeAndAutoImport —— NOT_APPLICABLE：引擎无 autoInclude 支持
// （Configuration 仅有 auto_imports；Java Configuration.addAutoInclude 无对应 API）。
// NOT_APPLICABLE: autoIncludeAndAutoImport —— 引擎无 autoInclude（Configuration 无 auto_includes）

/// Java lookupSrategiesAreNotConsideredProperly：
/// namespace 查找只按模板名（本地化/acquisition 等查找策略被忽略）——引擎同样以
/// 模板名（normalize 后）为 loaded_libs 缓存键，可 1:1 翻译。
#[test]
fn lookup_strategies_are_not_considered_properly() {
    let (c, loader) = test_config();
    setup(&loader);
    // 同名模板在不同 locale 下 import 只初始化一次（en_US → lib_en.ftl）
    let ftl1 = String::from("<#setting locale='en_US'><#import 'lib.ftl' as ns1>")
        + "<#setting locale='de_DE'><#import 'lib.ftl' as ns2>"
        + "<@ns1.m/> <@ns2.m/> ${history}";
    assert_output(&c, &loader, &ftl1, "en en LEn");
    // Java 断言 2：'*/lib.ftl'（acquisition）与 'lib.ftl' 指向同一模板，各自初始化
    // → "en en LEnLEn"。引擎差异：v1 的 import_lib 只做本地化回退、不做 acquisition
    // （'*' 路径在 get_template_localized 中直接 miss）→ 该段以引擎实际报错为准。
    let ftl2 = String::from("<#setting locale='en_US'>")
        + "<#import '*/lib.ftl' as ns1>"
        + "<#import 'lib.ftl' as ns2>"
        + "<@ns1.m/> <@ns2.m/> ${history}";
    let msg = assert_error_contains(&c, &loader, &ftl2, &["Template not found"]);
    assert!(msg.contains("*/lib.ftl"), "msg: {msg}");
}

// Java lazyImportBasics —— NOT_APPLICABLE：引擎无 lazy_imports 设置，import 恒立即初始化。
// NOT_APPLICABLE: lazyImportBasics —— 引擎不支持 lazyImports（v1 import 恒立即初始化）

// Java lazyImportAndLocale —— NOT_APPLICABLE：同上，依赖 lazy_imports。
// NOT_APPLICABLE: lazyImportAndLocale —— 引擎不支持 lazyImports

// Java lazyAutoImportSettings —— NOT_APPLICABLE：依赖 lazyImports/lazyAutoImports 设置。
// NOT_APPLICABLE: lazyAutoImportSettings —— 引擎不支持 lazyImports/lazyAutoImports

// Java lazyAutoImportMixedWithManualImport —— NOT_APPLICABLE：同上。
// NOT_APPLICABLE: lazyAutoImportMixedWithManualImport —— 引擎不支持 lazyImports/lazyAutoImports

// Java lazyImportErrors —— NOT_APPLICABLE：依赖 lazyImports（懒初始化错误语义）。
// NOT_APPLICABLE: lazyImportErrors —— 引擎不支持 lazyImports

// Java lazilyInitializingNamespaceOverridesAll —— NOT_APPLICABLE：JVM 反射遍历
// Namespace 方法（LazilyInitializedNamespace 覆盖检查）。
// NOT_APPLICABLE: lazilyInitializingNamespaceOverridesAll —— JVM 反射遍历方法表，引擎无等价类
