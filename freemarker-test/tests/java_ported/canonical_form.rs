//! Java `freemarker.core.CanonicalFormTest` —— 跳过（空 mod）
//! （对应 Java: CanonicalFormTest —— AST 的 canonical 形式（ASTPrinter 快照））
//!
//! 不可移植原因：Java 测试断言 `template.getCanonicalForm()`（ASTPrinter 输出
//! 的规范化源码，如 `[=1 + "[=2]"]`）；v1 引擎无 ASTPrinter/canonical form
//! 等价物 —— 无对应测试可翻译。
