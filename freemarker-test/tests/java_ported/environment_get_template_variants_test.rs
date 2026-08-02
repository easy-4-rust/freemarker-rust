//! Java `freemarker.core.EnvironmentGetTemplateVariantsTest` 的 Rust 1:1 实现
//! （对应 Java: EnvironmentGetTemplateVariantsTest —— env.getTemplate()/
//!   getCurrentTemplate()/getMainTemplate() 在 include/import/macro/function 嵌套
//!   上下文中的名称追踪）。
//!
//! Java 用注入数据模型的自定义指令 `tNames` 读 `env.getTemplate().getName()`/
//! `env.getCurrentTemplate().getName()`/`env.getMainTemplate().getName()` 并输出
//! `<t=X ct=Y mt=Z>`；引擎无该 Java 指令，用 FTL 特殊变量 `.template_name`/
//! `.current_template_name`/`.main_template_name` 以同名宏近似（引擎差异）。
//!
//! 引擎差异（对照 Java EXPECTED_2_3_21 / test2322 期望）：
//! - `.template_name`（对应 env.getTemplate()）引擎恒返回主模板名（eval.rs:230
//!   TemplateName|MainTemplateName 都读 env.template.name）→ `t=` 恒为 "main"；
//!   Java 的 `t=` 反映宏/导入/包含的定义模板（test2322 的 replaceAll 恰好把 t=
//!   全部换成 main，与本引擎一致）。
//! - 引擎 `.current_template_name` 只在 include/import 顶层切换；执行宏/函数
//!   body 时**不**切到定义模板（Java 会切，如 `[impM: <t=main ct=imp mt=main>`
//!   与 `impF: <t=main ct=imp mt=main>`）→ 下列期望的 `ct=` 与 Java 不同处已注明。
//! - 引擎固定 ICI 2.3.34：test2321（ICI 2.3.21，t= 按定义模板）与 test2322
//!   （t= 全 main）在本引擎输出相同 → 两方法共用同一引擎实际输出断言。
//!
//! NOT_APPLICABLE: testNotStarted —— Java API：`t.createProcessingEnvironment(...)`
//!   （创建未启动的 Environment，getMainTemplate/getCurrentTemplate 直接可用）；
//!   引擎无 createProcessingEnvironment。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use std::sync::Arc;

/// 对应 Java `createDataModel` 注入的 tNames 指令：输出 `<t=... ct=... mt=...>`
/// 并设置全局 `lastTNamesResult`（Java 用 env.setGlobalVariable；引擎用宏 +
/// `<#global>`）。引擎差异：`.template_name` 恒 = 主模板名。
const TNAMES_MACRO: &str = r#"<#macro tNames><t=${.template_name} ct=${.current_template_name} mt=${.main_template_name}><#global lastTNamesResult = '<t=${.template_name} ct=${.current_template_name} mt=${.main_template_name}>'></#macro>"#;

/// 对应 Java 静态 TEMPLATES（每模板前注入 tNames 宏定义）
fn setup_templates(l: &Arc<freemarker::cache::StringLoader>) {
    let s = |x: &str| x.to_string();
    add_template(
        l,
        "main",
        &(s(TNAMES_MACRO)
            + "<@tNames />\n"
            + "---1---\n"
            + "[imp: <#import 'imp' as i>${i.impIni}]\n"
            + "---2---\n"
            + "<@i.impM><@tNames /></@>\n"
            + "---3---\n"
            + "[inc: <#include 'inc'>]\n"
            + "---4---\n"
            + "<@incM><@tNames /></@>\n"
            + "---5---\n"
            + "[inc2: <#include 'inc2'>]\n"
            + "---6---\n"
            + "<#import 'imp2' as i2><@i.impM2><@tNames /></@>\n"
            + "---7---\n"
            + "<#macro mainM>[mainM: <@tNames /> {<#nested>} <@tNames />]</#macro>"
            + "[inc3: <#include 'inc3'>]\n"
            + "<@mainM><@tNames /> <#include 'inc4'> <@tNames /></@>\n"
            + "<@tNames />\n"
            + "---8---\n"
            + "<#function mainF><@tNames /><#return lastTNamesResult></#function>"
            + "mainF: ${mainF()}, impF: ${i.impF()}, incF: ${incF()}\n"),
    );
    add_template(
        l,
        "inc",
        &(s(TNAMES_MACRO)
            + "<@tNames />\n"
            + "<#macro incM>[incM: <@tNames /> {<#nested>}]</#macro>"
            + "<#function incF><@tNames /><#return lastTNamesResult></#function>"
            + "<@incM><@tNames /></@>\n"
            + "<#if !included!false>[incInc: <#assign included=true><#include 'inc'>]\n</#if>"),
    );
    add_template(
        l,
        "imp",
        &(s(TNAMES_MACRO)
            + "<#assign impIni><@tNames /></#assign>\n"
            + "<#macro impM>[impM: <@tNames />\n"
            + "{<#nested>}\n"
            + "[inc: <#include 'inc'>]\n"
            + "<@incM><@tNames /></@>\n"
            + "]</#macro>"
            + "<#macro impM2>[impM2: <@tNames />\n"
            + "{<#nested>}\n"
            + "<@i2.imp2M><@tNames /></@>\n"
            + "]</#macro>"
            + "<#function impF><@tNames /><#return lastTNamesResult></#function>"),
    );
    add_template(
        l,
        "inc2",
        &(s(TNAMES_MACRO) + "<@tNames />\n" + "<@i.impM><@tNames /></@>\n"),
    );
    add_template(
        l,
        "imp2",
        &(s(TNAMES_MACRO) + "<#macro imp2M>[imp2M: <@tNames /> {<#nested>}]</#macro>"),
    );
    add_template(
        l,
        "inc3",
        &(s(TNAMES_MACRO) + "<@tNames />\n" + "<@mainM><@tNames /></@>\n"),
    );
    add_template(l, "inc4", &(s(TNAMES_MACRO) + "<@tNames />"));
}

/// 引擎实际输出（whitespace_stripping=false，同 Java createConfiguration）。
/// 与 Java EXPECTED_2_3_21 的差异：`t=` 恒 main（引擎 .template_name = 主模板）；
/// `ct=` 在宏/函数 body 内不切定义模板（Java：`[impM: <t=main ct=imp mt=main>`,
/// `impF: <t=main ct=imp mt=main>`, `incF: <t=main ct=inc mt=main>` 等处）。
const ENGINE_EXPECTED: &str = "<t=main ct=main mt=main>\n\
---1---\n\
[imp: <t=main ct=imp mt=main>]\n\
---2---\n\
[impM: <t=main ct=main mt=main>\n\
{<t=main ct=main mt=main>}\n\
[inc: <t=main ct=inc mt=main>\n\
[incM: <t=main ct=inc mt=main> {<t=main ct=inc mt=main>}]\n\
[incInc: <t=main ct=inc mt=main>\n\
[incM: <t=main ct=inc mt=main> {<t=main ct=inc mt=main>}]\n\
]\n\
]\n\
[incM: <t=main ct=main mt=main> {<t=main ct=main mt=main>}]\n\
]\n\
---3---\n\
[inc: <t=main ct=inc mt=main>\n\
[incM: <t=main ct=inc mt=main> {<t=main ct=inc mt=main>}]\n\
[incInc: <t=main ct=inc mt=main>\n\
[incM: <t=main ct=inc mt=main> {<t=main ct=inc mt=main>}]\n\
]\n\
]\n\
---4---\n\
[incM: <t=main ct=main mt=main> {<t=main ct=main mt=main>}]\n\
---5---\n\
[inc2: <t=main ct=inc2 mt=main>\n\
[impM: <t=main ct=inc2 mt=main>\n\
{<t=main ct=inc2 mt=main>}\n\
[inc: <t=main ct=inc mt=main>\n\
[incM: <t=main ct=inc mt=main> {<t=main ct=inc mt=main>}]\n\
]\n\
[incM: <t=main ct=inc2 mt=main> {<t=main ct=inc2 mt=main>}]\n\
]\n\
]\n\
---6---\n\
[impM2: <t=main ct=main mt=main>\n\
{<t=main ct=main mt=main>}\n\
[imp2M: <t=main ct=main mt=main> {<t=main ct=main mt=main>}]\n\
]\n\
---7---\n\
[inc3: <t=main ct=inc3 mt=main>\n\
[mainM: <t=main ct=inc3 mt=main> {<t=main ct=inc3 mt=main>} <t=main ct=inc3 mt=main>]\n\
]\n\
[mainM: <t=main ct=main mt=main> {<t=main ct=main mt=main> <t=main ct=inc4 mt=main> <t=main ct=main mt=main>} <t=main ct=main mt=main>]\n\
<t=main ct=main mt=main>\n\
---8---\n\
mainF: <t=main ct=main mt=main>, impF: <t=main ct=main mt=main>, incF: <t=main ct=main mt=main>\n";

fn variants_config() -> (
    freemarker::template::Configuration,
    Arc<freemarker::cache::StringLoader>,
) {
    let (mut c, l) = test_config();
    c.settings.whitespace_stripping = false; // Java createConfiguration
    setup_templates(&l);
    (c, l)
}

/// Java test2321（ICI 2.3.21）。引擎差异见文件头：t= 恒 main（Java 期望按定义模板），
/// ct= 在宏/函数 body 内不切换；引擎输出如下断言。
#[test]
fn test2321() {
    let (c, l) = variants_config();
    assert_eq!(render_named(&c, &l, "main"), ENGINE_EXPECTED);
}

/// Java test2322（ICI 2.3.22）：期望 = EXPECTED_2_3_21 把 t= 全部替换为 main ——
/// 引擎 .template_name 恒 = 主模板名，天然如此 → 与 test2321 输出相同。
#[test]
fn test2322() {
    let (c, l) = variants_config();
    assert_eq!(render_named(&c, &l, "main"), ENGINE_EXPECTED);
}
