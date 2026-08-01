//! 解析器 —— 对应 FTL.jj → FMParser/FMParserTokenManager（docs/03）
//!
//! 模块布局：本文件只做模块声明与重导出（项目规范：mod.rs 禁止定义类型）。
//! - `lexer`：词法器（5 词法状态压缩为 `ExprCtx` + 括号深度，见 lexer.rs 文件头）；
//! - `grammar`：递归下降语法分析器（24 表达式产生式 + 13 指令产生式 + 空白剥离标记）。
//!
//! 契约入口：`parse(cfg, name, text) -> Result<Template>`（AST 类型见 `crate::core`）。

mod grammar;
mod lexer;

pub use grammar::{parse, parse_expression};
