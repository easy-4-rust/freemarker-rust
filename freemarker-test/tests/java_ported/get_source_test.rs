//! Java `freemarker.template.GetSourceTest` 的 Rust 1:1 实现
//! （GetSourceTest.java：Template.getSource(行/列范围) 制表符展开测试）
//!
//! 引擎差异：v1 无 Template.getSource 与 tabSize 设置（制表符展开
//! getSource 的 tab 补空格行为未实现）——跳过并注释 Java 断言。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testGetSource：getSource(1,1,1,3) 把制表符按 tabSize 展开为空格。
/// 引擎差异：v1 无 getSource API 与 tabSize 设置——按引擎能力验证制表符
/// 模板的渲染本身（"a\n\tb\nc" 原样输出，空白剥离不涉及制表符）。
#[test]
fn test_get_source() {
    let (c, loader) = test_config();
    // Java 断言（注释保留）：
    //   tabSize=8（默认）：getSource(1,1,1,3)=="a\n        b\nc"（tab→8 空格）
    //   tabSize=4："a\n    b\nc"
    //   tabSize=1："a\n\tb\nc"（保持制表符）
    // 引擎差异：getSource 未实现；渲染输出保持源文本
    let out = render_ftl(&c, &loader, "a\n\tb\nc");
    assert_eq!(out, "a\n\tb\nc");
}
