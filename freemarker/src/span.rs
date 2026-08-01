//! 源码位置标记（对应 Java `TemplateElement.getBeginLine()/getBeginColumn()`）。
//!
//! 错误消息 `[in template "x" at line N, column M]` 依赖此数据。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub const fn new(line: u32, col: u32) -> Self {
        Span { line, col }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at line {}, column {}", self.line, self.col)
    }
}
