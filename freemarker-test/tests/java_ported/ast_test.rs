//! Java `freemarker.core.ASTTest` —— 跳过（空 mod）
//! （对应 Java: ASTTest —— AST 节点形态断言（ASTPrinter 快照 + 父链/子链遍历））
//!
//! 不可移植原因：Java 测试遍历 TemplateElement 父子链并断言 ASTPrinter 输出；
//! v1 引擎的 AST（Element/Expr）与 Java 结构不同且无 ASTPrinter ——
//! 无对应测试可翻译。
