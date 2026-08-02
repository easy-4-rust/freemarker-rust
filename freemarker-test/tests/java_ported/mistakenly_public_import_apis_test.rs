//! Java `freemarker.template.MistakenlyPublicImportAPIsTest` 的 Rust 1:1 实现
//! （MistakenlyPublicImportAPIsTest.java：用户不应使用但需保持向后兼容的
//!   import 相关公开 API 行为测试）
//!
//! 引擎差异：v1 无 Template.getImports/addImport、Environment.getVariable/
//! setVariable 公开 API（import 命名空间由渲染环境内部管理）——整体跳过并注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testImportCopying：跨模板复制 LibraryLoad 的兼容行为
#[test]
fn test_import_copying() {
    // Java 断言（注释保留）：
    // - t1（含两个 <#import>）的 getImports() 复制到 t2 后，t2 渲染
    //   "<@i1.m/><@i2.m/>" 抛 InvalidReferenceException，blamedExpression=="i1"
    //   （历史上从未正常工作）；
    // - 通过 Environment 变量把 i1/i2（Namespace）注入 t2 渲染：
    //   "<@i1.m/>" → "1"；"<@i2.m/>" 渲染 "2" 或抛 NullPointerException
    //   （2.3.x 找不到宏的命名空间）。
    // 引擎差异：v1 无 getImports/addImport/setVariable 等误用 API——
    // 正常 import 路径（同名模板内 <#import> 后用命名空间）可工作，测试不可移植
    let (c, loader) = test_config();
    add_template(&loader, "imp1", "<#macro m>1</#macro>");
    add_template(&loader, "imp2", "<#assign x = 2><#macro m>${x}</#macro>");
    // 正常路径验证（Java 注释的"It works this way"部分语义）：
    assert_output(
        &c,
        &loader,
        "<#import 'imp1' as i1><#import 'imp2' as i2><@i1.m/><@i2.m/>",
        "12",
    );
}
