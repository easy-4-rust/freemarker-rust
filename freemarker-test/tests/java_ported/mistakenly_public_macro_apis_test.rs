//! Java `freemarker.template.MistakenlyPublicMacroAPIsTest` 的 Rust 1:1 实现
//! （MistakenlyPublicMacroAPIsTest.java：用户不应使用但需保持向后兼容的
//!   宏相关公开 API 行为测试）
//!
//! 引擎差异：v1 无 Template.getMacros/addMacro、Environment.getVariable/
//! setVariable 公开 API（宏表内部管理）——整体跳过并注释。
//!
//! NOT_APPLICABLE: testMacroCopyingExploit / testMacroCopyingExploitAndNamespaces /
//!   testMacroCopyingFromFTLVariable —— Java 断言围绕"误公开"API：
//!   Template.getMacros()/addMacro(Macro)（宏复制后 "<@m1/><@m2/><@m3/>..."
//!   渲染 == "123b 1b23b"；复制宏绑定目标模板命名空间 ${x}→"1"）与
//!   Environment.getVariable()/setVariable()（MistakenlyPublicMacroAPIsTest.java:44-81）；
//!   v1 无这些公开 API（宏定义仅解析期注册），断言不可移植——Java 原文保留于
//!   各方法注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testMacroCopyingExploit：把一个模板的宏复制到另一个模板
#[test]
fn test_macro_copying_exploit() {
    // NOT_APPLICABLE: getMacros/addMacro 复制断言——v1 无这些公开 API
    // （Java 原文见 MistakenlyPublicMacroAPIsTest.java:45-58）。
    // Java 断言（注释保留）：
    // - tMacros 定义 m1/m2；t 先加 m1/m2（宏复制）再定义同名 m1 与 m3，
    //   渲染 "<@m1/><@m2/><@m3/> <@m1/><@m2/><@m3/>" == "123b 1b23b"
    //   （addMacro 的宏被本模板定义覆盖；m3 来自本模板）。
    // 引擎差异：v1 无 addMacro 公开 API（宏定义仅在解析期注册）——不可移植
}

/// Java testMacroCopyingExploitAndNamespaces：复制宏使用的变量命名空间
#[test]
fn test_macro_copying_exploit_and_namespaces() {
    // NOT_APPLICABLE: 宏复制 API 缺失（Java 原文见
    // MistakenlyPublicMacroAPIsTest.java:60-67）。
    // Java：tMacros 的 m1 输出 ${x}（tMacros 中 x=0）；复制到 t（x=1）后
    // 渲染 "<@m1/>" == "1"（宏执行时绑定目标模板的命名空间）。
    // 引擎差异：v1 无宏复制 API——不可移植
}

/// Java testMacroCopyingFromFTLVariable：从环境变量取宏再复制
#[test]
fn test_macro_copying_from_ftl_variable() {
    // NOT_APPLICABLE: getVariable/setVariable/addMacro API 缺失（Java 原文见
    // MistakenlyPublicMacroAPIsTest.java:69-81）。
    // Java：env.process() 后 getVariable("m1") 是 Macro 实例；addMacro 后
    // 渲染 "<@m1/>" == "1"。
    // 引擎差异：v1 无 setVariable/addMacro API——不可移植
}
