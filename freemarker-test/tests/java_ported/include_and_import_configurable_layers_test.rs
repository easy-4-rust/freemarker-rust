//! Java `freemarker.core.IncludeAndImportConfigurableLayersTest` 的 Rust 1:1 实现
//! （对应 Java: IncludeAndImportConfigurableLayersTest —— 三层（Configuration /
//!   TemplateConfiguration / Environment）autoImport/autoInclude 的叠加、冲突
//!   覆盖、去重与 lazyImports/lazyAutoImports 惰性语义）。
//!
//! 引擎实现（对照 Java 源码逐层）：
//! - Configuration 层：`Configuration.auto_imports`/`auto_includes`（Java
//!   Configurable.autoImports/autoIncludes 在 cfg 层的自有表）+ add/remove 方法
//!   （addAutoImport/addAutoInclude 的"先移除再追加"语义，Configurable.java:1944-1960/
//!   :2098-2112）；
//! - 模板层：`TemplateConfiguration.auto_imports`/`auto_includes`/lazy 字段
//!   （Java TemplateConfiguration extends Configurable；apply(Template) 时合并进
//!   Template 对象，TemplateConfiguration.java:399-406 —— Rust 侧 Environment::new
//!   从 template.template_configuration 复制为 t 层数据）；
//! - Environment 层：`Environment.addAutoImport`/`addAutoInclude`（Java
//!   Configurable 在 env 层的自有表；test 中 env.addAutoImport("t3", ...)）；
//! - 渲染入口：`Environment.process :322 doAutoImportsAndIncludes` →
//!   `Configuration.doAutoImports/doAutoIncludes`（Configuration.java:3679-3748）：
//!   cfg（父）→ t（主模板）→ env（子）顺序执行，低层同名被高层覆盖；
//! - lazyImports/lazyAutoImports：Settings.lazy_imports/lazy_auto_imports
//!   （Configurable.java:410-412/501；auto imports 用
//!   `getLazyAutoImports() ?? getLazyImports()`，Configuration.java:3690-3692；
//!   `<#import>` 用 getLazyImports()，Environment.java:3168-3170）。
//!
//! 文档化偏差（与 Java 可观测结果一致）：
//! - lazy 命名空间（Java LazilyInitializedNamespace，Environment.java:3501-3590）
//!   的"首次访问才初始化"在 v1 为占位不加载（按需初始化需 env 回引，P 项）；
//!   本测试从不访问 lazy 命名空间变量，可观测语义（是否在渲染开始时处理）一致；
//! - Rust get_template 的 cfg 是加载时克隆（Rc）→ cfg 层 mutate 后需
//!   clear_template_cache() 重载才可见（Java 同一 Configuration 实例直接可见）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::{
    ConditionalTemplateConfigurationFactory, FileNameGlobMatcher, StringLoader,
};
use freemarker::core::{Environment, TemplateConfiguration};
use freemarker::template::{Configuration, TModel};
use std::sync::Arc;

/// Java addCommonTemplates（IncludeAndImportConfigurableLayersTest.java:326-335）
fn common_config() -> (Configuration, Arc<StringLoader>) {
    let (c, loader) = test_config();
    add_template(&loader, "main.ftl", "In main: ${loaded}");
    add_template(&loader, "main2.ftl", "In main2: ${loaded}");
    add_template(&loader, "t1.ftl", "<#global loaded = (loaded!) + 't1;'>T1;");
    add_template(&loader, "t2.ftl", "<#global loaded = (loaded!) + 't2;'>T2;");
    add_template(&loader, "t3.ftl", "<#global loaded = (loaded!) + 't3;'>T3;");
    add_template(
        &loader,
        "t1b.ftl",
        "<#global loaded = (loaded!) + 't1b;'>T1b;",
    );
    add_template(
        &loader,
        "t2b.ftl",
        "<#global loaded = (loaded!) + 't2b;'>T2b;",
    );
    add_template(
        &loader,
        "t3b.ftl",
        "<#global loaded = (loaded!) + 't3b;'>T3b;",
    );
    (c, loader)
}

/// Java `cfg.setTemplateConfigurations(new ConditionalTemplateConfigurationFactory(
/// new FileNameGlobMatcher("main.ftl"), tc))`
fn with_tc(c: &mut Configuration, glob: &str, tc: TemplateConfiguration) {
    c.set_template_configurations(Some(Arc::new(
        ConditionalTemplateConfigurationFactory::with_configuration(
            Box::new(FileNameGlobMatcher::new(glob)),
            Arc::new(tc),
        ),
    )));
}

/// Java `t.createProcessingEnvironment(null, sw)` + setup + `env.process()`：
/// 渲染命名模板，env 构造后可先 addAutoImport/addAutoInclude/setLazy* 再 process
fn render_named_with_env<F: FnOnce(&mut Environment)>(
    c: &Configuration,
    name: &str,
    setup: F,
) -> String {
    let t = c
        .get_template(name)
        .unwrap_or_else(|e| panic!("get_template({name}) failed: {e}"));
    let mut out: Vec<u8> = Vec::new();
    let mut env = Environment::new(&t, TModel::from_hash(indexmap::IndexMap::new()), &mut out);
    setup(&mut env);
    env.process().unwrap();
    String::from_utf8_lossy(&out).into_owned()
}

/// Java test3LayerImportNoClashes（IncludeAndImportConfigurableLayersTest.java:37-86）
#[test]
fn test3_layer_import_no_clashes() {
    let (mut c, _loader) = common_config();
    c.add_auto_import("t1", "t1.ftl");
    let mut tc = TemplateConfiguration::default();
    tc.add_auto_import("t2", "t2.ftl");
    with_tc(&mut c, "main.ftl", tc);

    // env 层 t3 + main.ftl（tc 命中）→ "In main: t1;t2;t3;"
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| env.add_auto_import("t3", "t3.ftl")),
        "In main: t1;t2;t3;"
    );
    // 无 env 层 → "In main: t1;t2;"
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |_| {}),
        "In main: t1;t2;"
    );
    // main2.ftl 不匹配 tc（FileNameGlobMatcher("main.ftl")）→ "In main2: t1;t3;"
    assert_eq!(
        render_named_with_env(&c, "main2.ftl", |env| env.add_auto_import("t3", "t3.ftl")),
        "In main2: t1;t3;"
    );
    c.remove_auto_import("t1");
    // Rust get_template 的 cfg 是加载时克隆 → 清缓存重载（Java 同一实例直接可见）
    c.clear_template_cache();
    // → "In main: t2;t3;"
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| env.add_auto_import("t3", "t3.ftl")),
        "In main: t2;t3;"
    );
}

/// Java test3LayerImportClashes（:89-128）：三层同名 autoImport 的覆盖顺序
/// （env 层覆盖 tc 层覆盖 cfg 层）
#[test]
fn test3_layer_import_clashes() {
    let (mut c, _loader) = common_config();
    c.add_auto_import("t1", "t1.ftl");
    c.add_auto_import("t2", "t2.ftl");
    c.add_auto_import("t3", "t3.ftl");
    let mut tc = TemplateConfiguration::default();
    tc.add_auto_import("t2", "t2b.ftl"); // tc 层覆盖 cfg 层同名
    with_tc(&mut c, "main.ftl", tc);

    // env t3 覆盖 cfg t3 → "In main: t1;t2b;t3b;"
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| env.add_auto_import("t3", "t3b.ftl")),
        "In main: t1;t2b;t3b;"
    );
    // main2.ftl（tc 不匹配）→ "In main2: t1;t2;t3b;"
    assert_eq!(
        render_named_with_env(&c, "main2.ftl", |env| env.add_auto_import("t3", "t3b.ftl")),
        "In main2: t1;t2;t3b;"
    );
    // 无 env 层 → cfg t3 不被覆盖 → "In main: t1;t3;t2b;"
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |_| {}),
        "In main: t1;t3;t2b;"
    );
}

/// Java test3LayerIncludesNoClashes（:131-180）：autoInclude 三层叠加（include
/// 先于主模板执行；T1;/T2;/T3; 为被包含模板的文本输出）
#[test]
fn test3_layer_includes_no_clashes() {
    let (mut c, _loader) = common_config();
    c.add_auto_include("t1.ftl");
    let mut tc = TemplateConfiguration::default();
    tc.add_auto_include("t2.ftl");
    with_tc(&mut c, "main.ftl", tc);

    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| env.add_auto_include("t3.ftl")),
        "T1;T2;T3;In main: t1;t2;t3;"
    );
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |_| {}),
        "T1;T2;In main: t1;t2;"
    );
    assert_eq!(
        render_named_with_env(&c, "main2.ftl", |env| env.add_auto_include("t3.ftl")),
        "T1;T3;In main2: t1;t3;"
    );
    c.remove_auto_include("t1.ftl");
    c.clear_template_cache();
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| env.add_auto_include("t3.ftl")),
        "T2;T3;In main: t2;t3;"
    );
}

/// Java test3LayerIncludeClashes（:183-232）：tc 层与 cfg 层同名 → 低层跳过
/// （cfg t2 因 tc 有 t2 不执行）；env 层同名覆盖低层
#[test]
fn test3_layer_include_clashes() {
    let (mut c, _loader) = common_config();
    c.add_auto_include("t1.ftl");
    c.add_auto_include("t2.ftl");
    c.add_auto_include("t3.ftl");
    let mut tc = TemplateConfiguration::default();
    tc.add_auto_include("t2.ftl");
    with_tc(&mut c, "main.ftl", tc);

    // main.ftl + env t3 → cfg t2 跳过（tc 层有）→ T1;T2(tc);T3(env)
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| env.add_auto_include("t3.ftl")),
        "T1;T2;T3;In main: t1;t2;t3;"
    );
    // main2.ftl（tc 不匹配）→ cfg 全执行 + env t3
    assert_eq!(
        render_named_with_env(&c, "main2.ftl", |env| env.add_auto_include("t3.ftl")),
        "T1;T2;T3;In main2: t1;t2;t3;"
    );
    // 无 env 层 → cfg t2 跳过（tc 有）→ T1;T3;T2;
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |_| {}),
        "T1;T3;T2;In main: t1;t3;t2;"
    );
    // env.addAutoInclude("t1.ftl") → cfg t1 跳过（env 有）→ T3;T2;T1;
    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| env.add_auto_include("t1.ftl")),
        "T3;T2;T1;In main: t3;t2;t1;"
    );
}

/// Java test3LayerIncludesClashes2（:235-258）：同层重复 addAutoInclude → 去重
/// （先移除再追加，Configurable.java:2098-2112）
#[test]
fn test3_layer_includes_clashes2() {
    let (mut c, _loader) = common_config();
    c.add_auto_include("t1.ftl");
    c.add_auto_include("t1.ftl"); // 同层重复 → 去重
    let mut tc = TemplateConfiguration::default();
    tc.add_auto_include("t2.ftl");
    tc.add_auto_include("t2.ftl");
    with_tc(&mut c, "main.ftl", tc);

    assert_eq!(
        render_named_with_env(&c, "main.ftl", |env| {
            env.add_auto_include("t3.ftl");
            env.add_auto_include("t3.ftl");
            env.add_auto_include("t1.ftl");
            env.add_auto_include("t1.ftl");
        }),
        "T2;T3;T1;In main: t2;t3;t1;"
    );
}

/// 层枚举（Java test3LayerLazyness 的
/// `new Class<?>[] { Configuration.class, Template.class, Environment.class }`；
/// Template 层经 TemplateConfiguration 承载——Java Template extends Configurable，
/// Rust 侧 tc 应用等价）
#[derive(Clone, Copy, PartialEq, Debug)]
enum Layer {
    Configuration,
    Template,
    Environment,
}

/// Java test3LayerLazyness 的单组（:280-316）：模板
/// `<#import 't2.ftl' as t2>${loaded!}` + cfg.addAutoImport("t1", "t1.ftl")；
/// 按 layer 在 Configuration/Template(经 tc)/Environment 上设置
/// lazyImports/lazyAutoImports 后 process。
fn test3_layer_lazyness(
    c: &mut Configuration,
    layer: Layer,
    lazy_imports: Option<bool>,
    lazy_auto_imports: Option<bool>,
    set_lazy_auto_imports: bool,
    expected: &str,
) {
    c.add_auto_import("t1", "t1.ftl");
    match layer {
        Layer::Configuration => {
            // Java setLazynessOfConfigurable(cfg, ...)
            if let Some(l) = lazy_imports {
                c.set_lazy_imports(l);
            }
            if set_lazy_auto_imports {
                c.set_lazy_auto_imports(lazy_auto_imports);
            }
        }
        Layer::Template => {
            let mut tc = TemplateConfiguration::default();
            if let Some(l) = lazy_imports {
                tc.set_lazy_imports(l);
            }
            if set_lazy_auto_imports {
                tc.set_lazy_auto_imports(lazy_auto_imports);
            }
            with_tc(c, "lazy.ftl", tc);
        }
        Layer::Environment => {}
    }
    // Java 每次 new Template（不缓存）；Rust get_template 缓存 + cfg 克隆
    // → 清缓存重载等价
    c.clear_template_cache();
    let t = c
        .get_template("lazy.ftl")
        .unwrap_or_else(|e| panic!("get_template(lazy.ftl) failed: {e}"));
    let mut out: Vec<u8> = Vec::new();
    let mut env = Environment::new(&t, TModel::from_hash(indexmap::IndexMap::new()), &mut out);
    if layer == Layer::Environment {
        if let Some(l) = lazy_imports {
            env.set_lazy_imports(l);
        }
        if set_lazy_auto_imports {
            env.set_lazy_auto_imports(lazy_auto_imports);
        }
    }
    env.process().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out),
        expected,
        "layer={layer:?} lazyImports={lazy_imports:?} lazyAutoImports={lazy_auto_imports:?} setLazyAuto={set_lazy_auto_imports}"
    );
}

/// Java test3LayerLazyness（:261-278）：三层 × 12 组惰性矩阵
#[test]
fn test3_layer_lazyness_matrix() {
    for layer in [Layer::Configuration, Layer::Template, Layer::Environment] {
        // Java 每层 12 组（lazyImports, lazyAutoImports, setLazyAutoImports, expected）
        let combos: [(&str, Option<bool>, Option<bool>, bool); 12] = [
            ("t1;t2;", None, None, false),
            ("t1;t2;", None, None, true),
            ("t1;t2;", None, Some(false), true),
            ("t2;", None, Some(true), true),
            ("t1;t2;", Some(false), None, false),
            ("t1;t2;", Some(false), None, true),
            ("t1;t2;", Some(false), Some(false), true),
            ("t2;", Some(false), Some(true), true),
            ("", Some(true), None, false),
            ("", Some(true), None, true),
            ("t1;", Some(true), Some(false), true),
            ("", Some(true), Some(true), true),
        ];
        for (_i, (expected, lazy_imports, lazy_auto_imports, set_lazy_auto)) in
            combos.iter().enumerate()
        {
            // Java dropConfiguration() + getConfiguration()：每组全新 Configuration
            let (mut c, loader) = test_config();
            add_template(&loader, "lazy.ftl", "<#import 't2.ftl' as t2>${loaded!}");
            add_template(&loader, "t1.ftl", "<#global loaded = (loaded!) + 't1;'>T1;");
            add_template(&loader, "t2.ftl", "<#global loaded = (loaded!) + 't2;'>T2;");
            test3_layer_lazyness(
                &mut c,
                layer,
                *lazy_imports,
                *lazy_auto_imports,
                *set_lazy_auto,
                expected,
            );
        }
    }
}
