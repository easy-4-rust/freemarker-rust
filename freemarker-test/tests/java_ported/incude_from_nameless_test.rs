//! Java `freemarker.template.IncudeFromNamelessTest` 的 Rust 1:1 实现
//! （IncudeFromNamelessTest.java：无名模板（name==null）中 include/import
//!   相对与绝对路径解析测试）
//!
//! 引擎映射：v1 内联模板解析名 "adhoc"（无目录）——相对引用以根为基准，
//! 与 Java 无名模板（baseName=null → 根相对）语义一致。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java test()：无名模板里 include 相对/绝对路径 + import 命名空间
#[test]
fn test_incude_from_nameless() {
    let (c, loader) = test_config();
    add_template(&loader, "i.ftl", "[i]");
    add_template(&loader, "sub/i.ftl", "[sub/i]");
    add_template(&loader, "import.ftl", "<#assign x = 1>");

    // 引擎差异：Java 用无名 Template（new Template(null, reader, cfg)）——
    // v1 render_ftl 用固定名 "adhoc"（根相对语义相同）
    let ftl = "<#include 'i.ftl'>\n".to_string()
        + "<#include '/i.ftl'>
<#include 'sub/i.ftl'>
<#include '/sub/i.ftl'><#import 'import.ftl' as i>${i.x}";
    assert_output(&c, &loader, &ftl, "[i][i][sub/i][sub/i]1");
}
