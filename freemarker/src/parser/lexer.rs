//! 词法器 —— 对应 Java `freemarker.core.FMParserTokenManager`（FTL.jj TOKEN 块）
//!
//! JavaCC 的 5 个词法状态（docs/03 §2.1）在本实现中由调用上下文隐式表达：
//! - `DEFAULT`：模板文本扫描（`scan_text_chunk`，识别 `<#`/`<@`/`${`/`<#--` 等）；
//! - `FM_EXPRESSION` / `NO_SPACE_EXPRESSION` / `NAMED_PARAMETER_EXPRESSION`：
//!   `next_expr_token(ExprCtx)` 按表达式上下文出 token（三个状态的差异在本实现中
//!   表现为 `ExprCtx` 与括号深度，语义等价，见 docs/03 §2.3 规则 1/3/6）；
//! - `IN_PAREN`：括号深度 > 0 时 `>`/`>=` 为比较运算符而非标签结束（`parenthesisNesting`）；
//! - `NO_PARSE`：`<#-- -->` 注释、`<#comment>`、`<#noparse>` 内容（`scan_comment`/`scan_unparsed`）。
//!
//! 词法特殊规则（docs/03 §2.3）：
//! 1. `<` 歧义：`<` 仅在后跟 `#`/`@`/`/`（严格语法）或已知指令名（非严格语法）时为标签开头，
//!    否则为文本（`a < b` 原样输出）；
//! 2. `$${` 转义为字面 `$` + 插值（`scan_text_chunk` 中把前一个 `$` 并入文本）；
//! 3. `[#` 为方括号指令语法，`[` 后非 `#` 为文本（表达式状态中 `[` 是列表字面量）；
//! 4. 注释 `<#-- -->` 在文本与表达式状态均可出现；
//! 5. 字符串插值 `"a${x}b"` 的原始扫描与 `${`/`#{` 起始检测（解码与插值在 grammar.rs）；
//! 6. 数字后缀 `L/F/D/B`、`0x` 十六进制、指数（超出 JavaCC 的 INTEGER/DECIMAL 的扩展，
//!    对应 value.rs 的 TNumber 映射契约）；
//! 7. 指令名大小写不敏感（`<#IF>` 合法，FTL.jj 中大小写敏感，此处按项目规范放宽）；
//! 8. 空白剥离标记在 grammar.rs 解析期完成；
//! 9. `[#ftl]` 头部仅在模板首行有效（grammar.rs 的 Root 产生式处理）。

use crate::error::{Result, TemplateError};

/// 标签语法（对应 `Configuration.TAG_SYNTAX`，首个标签自动检测）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagSyntax {
    /// `<>` 尖括号语法（默认）
    Angle,
    /// `[]` 方括号语法
    Square,
}

/// 表达式词法上下文（对应 JavaCC 词法状态：标签参数区 vs 插值内部）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExprCtx {
    /// 指令标签参数区：括号深度 0 时 `>`（或方括号语法下 `]`）结束标签；
    /// 对应 FM_EXPRESSION / NO_SPACE_EXPRESSION / NAMED_PARAMETER_EXPRESSION / IN_PAREN
    Tag { square: bool },
    /// `${...}` / `#{...}` 插值内部：`}` 结束插值，`>` 为普通比较符；
    /// 对应 FM_EXPRESSION 的 postInterpolationLexState 语义
    Interp,
}

/// 表达式 token（对应 FTL.jj `<FM_EXPRESSION, ...> TOKEN` 块）
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    /// 标识符（含非 ASCII 与 `\` 转义字符，如 `a\-b` → "a-b"）
    Ident(String),
    /// 数字字面量原始文本（含 0x/指数/后缀，未转换；转换在 grammar.rs）
    Number(String),
    /// 字符串字面量原始内容（含转义序列，未解码；不含引号）
    Str(String),
    /// 原始字符串 `r"..."` 内容（无转义、无插值）
    RawStr(String),
    True,
    False,
    In,
    As,
    Using,
    /// `lt` / `\lt` / `<` / `&lt;`（LESS_THAN）
    Lt,
    /// `lte` / `\lte` / `<=` / `&lt;=`
    Lte,
    /// `gt` / `\gt` / `&gt;`（NATURAL_GT 仅括号内/插值内；标签内 `>` 是结束符）
    Gt,
    /// `gte` / `\gte` / `&gt;=`
    Gte,
    Plus,
    Minus,
    Times,
    /// `**`（DOUBLE_STAR）
    DoubleStar,
    Divide,
    Percent,
    PlusEq,
    MinusEq,
    TimesEq,
    DivEq,
    ModEq,
    PlusPlus,
    MinusMinus,
    /// `=` 或 `==`（DOUBLE_EQUALS 与 EQUALS 语义合一）
    Eq,
    NotEq,
    /// `!`（EXCLAM，默认值/逻辑非共用）
    Exclam,
    /// `??`（EXISTS）
    Exists,
    /// `?`（BUILT_IN）
    Builtin,
    /// `&` / `&&` / `&amp;&amp;` / `\and`
    And,
    /// `|` / `||`
    Or,
    /// `->` / `-&gt;`
    LambdaArrow,
    Dot,
    DotDot,
    /// `..<` 或 `..!`（排端范围）
    DotDotLess,
    /// `..*`（无界范围）
    DotDotStar,
    /// `...`（宏 catch-all 参数）
    Ellipsis,
    Comma,
    Semicolon,
    Colon,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    OpenCurly,
    CloseCurly,
    /// 标签结束 `>` / `]`（括号深度 0；对应 DIRECTIVE_END）
    TagEnd,
    /// 标签结束 `/>` / `/]`（对应 EMPTY_DIRECTIVE_END）
    EmptyTagEnd,
    /// 插值结束 `}`（对应 CLOSING_CURLY_BRACKET 的 endInterpolation 分支）
    InterpEnd,
    Eof,
}

/// 文本扫描的停止原因（DEFAULT 状态出口）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextStop {
    /// 文本结束（模板文件尾）
    Eof,
    /// 遇到标签开头（`<#`/`<@`/`</#`/`<#--` 等，或非严格语法的 `<name`）
    Tag,
    /// 遇到 `${` / `#{`（插值开始；`$${` 时前一个 `$` 已并入文本）
    Interp,
}

/// 标签开头（`read_tag_open` 消费后的分类）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagOpen {
    /// `<#name` / `[#name` 指令开始
    Dir { square: bool },
    /// `<@name` / `[@name` 用户指令（宏）调用开始
    Call { square: bool },
    /// `</#name` / `[/#name` 指令结束标签
    EndDir { square: bool },
    /// `</@name` / `[/@name` 调用结束标签
    EndCall { square: bool },
    /// `<#--` / `[#--` 简洁注释（TERSE_COMMENT）
    TerseComment { square: bool },
}

/// 可保存/恢复的词法位置（用于前瞻：named-args 判定、非严格标签判定等）
#[derive(Debug, Clone, Copy)]
pub(crate) struct LexerPos {
    pos: usize,
    line: u32,
    col: u32,
    paren_depth: u32,
    bracket_depth: u32,
    curly_depth: u32,
}

/// 词法器状态（对应 FMParserTokenManager 的字段）
pub(crate) struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    /// 括号嵌套深度（对应 parenthesisNesting；> 0 时 `>` 是 NATURAL_GT）
    pub(crate) paren_depth: u32,
    /// 方括号嵌套深度（对应 bracketNesting；> 0 时 `]` 关闭列表字面量）
    pub(crate) bracket_depth: u32,
    /// 花括号嵌套深度（对应 curlyBracketNesting；> 0 时 `}` 关闭哈希字面量）
    pub(crate) curly_depth: u32,
    /// 严格语法模式（对应 token_source.strictSyntaxMode；`<#ftl strict_syntax=false>` 可关闭；
    /// 初始值取 Configuration.settings.strict_syntax）
    pub(crate) strict_syntax: bool,
    /// 已确立的标签语法（对应 squBracTagSyntax + autodetectTagSyntax；首标签决定）
    pub(crate) tag_syntax: Option<TagSyntax>,
    /// 模板名（错误消息用）
    name: String,
}

impl Lexer {
    pub(crate) fn new(name: &str, text: &str, strict_syntax: bool) -> Self {
        Lexer {
            chars: text.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            paren_depth: 0,
            bracket_depth: 0,
            curly_depth: 0,
            strict_syntax,
            tag_syntax: None,
            name: name.to_string(),
        }
    }

    pub(crate) fn save(&self) -> LexerPos {
        LexerPos {
            pos: self.pos,
            line: self.line,
            col: self.col,
            paren_depth: self.paren_depth,
            bracket_depth: self.bracket_depth,
            curly_depth: self.curly_depth,
        }
    }

    pub(crate) fn restore(&mut self, p: &LexerPos) {
        self.pos = p.pos;
        self.line = p.line;
        self.col = p.col;
        self.paren_depth = p.paren_depth;
        self.bracket_depth = p.bracket_depth;
        self.curly_depth = p.curly_depth;
    }

    /// 当前字符（不消费）
    pub(crate) fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// 当前字符后第 offset 个字符（不消费）
    pub(crate) fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    /// 消费一个字符并维护行列（`\n`、`\r`、`\r\n` 计为换行）
    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        match c {
            '\n' => {
                self.line += 1;
                self.col = 1;
            }
            '\r' => {
                // \r\n 视为单个换行：\r 自身不计数，紧随的 \n 由 \n 分支计数（共 +1 行）
                self.col = 1;
            }
            _ => {
                self.col += 1;
            }
        }
        Some(c)
    }

    /// 当前行列（1-based）
    pub(crate) fn line_col(&self) -> (u32, u32) {
        (self.line, self.col)
    }

    /// 解析错误：`Parsing error in template "{name}" at line L, column C. {details}`
    pub(crate) fn err(&self, line: u32, col: u32, details: impl Into<String>) -> TemplateError {
        TemplateError::Parse {
            template: self.name.clone(),
            message: format!(
                "\"{}\" at line {}, column {}. {}",
                self.name,
                line,
                col,
                details.into()
            ),
        }
    }

    /// 跳过空白（表达式状态 SKIP：空格/制表/换行/回车）
    pub(crate) fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// 当前字符是否可能开始一个标签（DEFAULT 状态的 `<`/`[` 歧义判定，docs/03 §2.3 规则 1/3）
    pub(crate) fn starts_tag(&mut self) -> bool {
        let save = self.save();
        let r = self.starts_tag_inner();
        self.restore(&save);
        r
    }

    fn starts_tag_inner(&mut self) -> bool {
        match self.peek() {
            Some('<') => match self.peek_at(1) {
                // `<#name` / `<#--`：`<#` 后必须是字母/下划线或 `--`（注释），否则是文本
                Some('#') => match self.peek_at(2) {
                    Some(c) if c.is_ascii_alphabetic() || c == '_' => true,
                    Some('-') => self.peek_at(3) == Some('-'),
                    _ => false,
                },
                // `<@name`
                Some('@') => true,
                // `</#` / `</@` / 非严格语法的 `</name`
                Some('/') => match self.peek_at(2) {
                    Some('#') | Some('@') => true,
                    _ => {
                        if !self.strict_syntax {
                            self.bump();
                            self.bump();
                            return self.non_strict_tag_name(true);
                        }
                        false
                    }
                },
                // 非严格语法：`<name`（已知指令名 + 合法形状）
                Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                    if self.strict_syntax {
                        false
                    } else {
                        self.bump();
                        self.non_strict_tag_name(false)
                    }
                }
                _ => false,
            },
            Some('[') => match self.peek_at(1) {
                // `[#name` / `[#--`（方括号语法；`[` 后非 `#`/`@` 为文本）
                Some('#') => match self.peek_at(2) {
                    Some(c) if c.is_ascii_alphabetic() || c == '_' => true,
                    Some('-') => self.peek_at(3) == Some('-'),
                    _ => false,
                },
                Some('@') => true,
                Some('/') => matches!(self.peek_at(2), Some('#') | Some('@')),
                _ => false,
            },
            _ => false,
        }
    }

    /// 非严格语法：`<`（或 `</`）已消费，当前位于指令名首字符。
    /// 要求：已知指令名（大小写不敏感）且形状合法 —— 需要参数的指令后跟空白；
    /// 简单指令与结束标签后跟空白*闭合符。返回是否应视为标签
    /// （JavaCC 最长匹配失败则回落为文本）。
    fn non_strict_tag_name(&mut self, is_end: bool) -> bool {
        let name = self.read_name();
        let Some(name) = name else { return false };
        if !DIRECTIVE_NAMES.contains(&name.as_str()) || name == "ftl" {
            // `<ftl>` 不是 FTL_HEADER（FTL.jj 仅 `<#ftl`/`[#ftl`），按 Java 语义为文本
            return false;
        }
        if is_end {
            // `</if>` 等结束标签：空白*闭合符（END_xxx token 用 CLOSE_TAG1）
            self.skip_ws();
            return matches!(self.peek(), Some('>') | Some(']'))
                || (self.peek() == Some('/') && matches!(self.peek_at(1), Some('>') | Some(']')));
        }
        let param_required = PARAM_DIRECTIVES.contains(&name.as_str());
        if param_required {
            // `<if x>` 合法；`<if>`/`<ifx>` 为文本（IF token 需要 BLANK）
            matches!(self.peek(), Some(c) if c == ' ' || c == '\t' || c == '\n' || c == '\r')
        } else {
            // `<else>`/`<break>` 等：空白*（`/`）?（`>`|`]`）
            self.skip_ws();
            match self.peek() {
                Some('>') | Some(']') => true,
                Some('/') => matches!(self.peek_at(1), Some('>') | Some(']')),
                _ => false,
            }
        }
    }

    /// 读取标签/指令名（字母与下划线；对应 UNKNOWN_DIRECTIVE 的 `[A-Za-z_]+`）
    pub(crate) fn read_name(&mut self) -> Option<String> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() || c == '_' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    /// 消费 `<`/`[` 标签开头并分类（调用前必须 starts_tag() == true）
    pub(crate) fn read_tag_open(&mut self) -> TagOpen {
        let square = self.peek() == Some('[');
        self.bump(); // < 或 [
        match self.peek() {
            Some('#') => {
                self.bump();
                if self.peek() == Some('-') && self.peek_at(1) == Some('-') {
                    self.bump();
                    self.bump();
                    TagOpen::TerseComment { square }
                } else {
                    TagOpen::Dir { square }
                }
            }
            Some('@') => {
                self.bump();
                TagOpen::Call { square }
            }
            Some('/') => {
                self.bump();
                match self.peek() {
                    Some('@') => {
                        self.bump();
                        TagOpen::EndCall { square }
                    }
                    // `</#name` 或非严格语法 `</name`（`#` 可选）
                    Some('#') => {
                        self.bump();
                        TagOpen::EndDir { square }
                    }
                    _ => TagOpen::EndDir { square },
                }
            }
            // 非严格语法 `<name`：START_TAG "<" 后直接跟指令名（starts_tag 已保证合法）
            Some(c) if c.is_ascii_alphabetic() || c == '_' => TagOpen::Dir { square },
            _ => unreachable!("starts_tag 已保证标签开头"),
        }
    }

    /// 消费标签/指令结束符 `>`、`/>`、`]`、`/]`（允许任意组合，宽于 JavaCC 的
    /// CLOSE_TAG1/CLOSE_TAG2 区分；文档化偏差）。返回 false 表示不匹配。
    pub(crate) fn try_read_tag_end(&mut self) -> Option<bool> {
        // 返回 Some(true) = `/>`（自闭合），Some(false) = `>`
        match self.peek() {
            Some('>') => {
                self.bump();
                Some(false)
            }
            Some(']') => {
                self.bump();
                Some(false)
            }
            Some('/') => match self.peek_at(1) {
                Some('>') | Some(']') => {
                    self.bump();
                    self.bump();
                    Some(true)
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// 扫描一段模板文本（DEFAULT 状态），直到标签/插值开头或文件尾。
    /// `$${` 时前一个 `$` 并入文本（docs/03 §2.3 规则 2）。
    pub(crate) fn scan_text_chunk(&mut self) -> Result<(String, TextStop)> {
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Ok((text, TextStop::Eof)),
                Some('<') | Some('[') => {
                    if self.starts_tag() {
                        return Ok((text, TextStop::Tag));
                    }
                    text.push(self.bump().unwrap());
                }
                Some('$') => {
                    if self.peek_at(1) == Some('{') {
                        return Ok((text, TextStop::Interp));
                    }
                    if self.peek_at(1) == Some('$') && self.peek_at(2) == Some('{') {
                        // `$${`：字面 `$` + 插值开始
                        text.push(self.bump().unwrap());
                        continue;
                    }
                    text.push(self.bump().unwrap());
                }
                Some('#') => {
                    if self.peek_at(1) == Some('{') {
                        // 传统数值插值 `#{expr}`（legacy NumericalOutput，契约坍缩为 Interpolation）
                        return Ok((text, TextStop::Interp));
                    }
                    text.push(self.bump().unwrap());
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
    }

    /// 表达式 token 扫描（FM_EXPRESSION 家族状态）。
    /// `ctx` 决定 `>`/`]`/`}` 在深度 0 时的语义（标签结束/插值结束/错误）。
    /// 返回 (token, 起始行, 起始列, 结束行, 结束列) —— 结束位置供解析器做
    /// token 相邻性判定（`<@callee>` 的 NO_SPACE_EXPRESSION 语义，见 grammar.rs）。
    pub(crate) fn next_expr_token(&mut self, ctx: ExprCtx) -> Result<(Tok, u32, u32, u32, u32)> {
        loop {
            self.skip_ws();
            let (line, col) = self.line_col();
            let c = match self.peek() {
                None => return Ok((Tok::Eof, line, col, line, col)),
                Some(c) => c,
            };
            // 表达式内注释 `<#-- -->` / `<!-- -->` / `[#-- --]` / `[!-- --]`（EXPRESSION_COMMENT）
            if c == '<' || c == '[' {
                let n1 = self.peek_at(1);
                let n2 = self.peek_at(2);
                if matches!(n1, Some('#') | Some('!'))
                    && n2 == Some('-')
                    && self.peek_at(3) == Some('-')
                {
                    self.bump();
                    self.bump();
                    self.bump();
                    self.bump();
                    self.skip_expr_comment()?;
                    continue;
                }
            }
            let (tok, _, _) = self.scan_expr_token_after_ws(ctx, line, col)?;
            let (el, ec) = self.line_col();
            return Ok((tok, line, col, el, ec));
        }
    }

    /// 表达式注释正文（EXPRESSION_COMMENT SKIP：直到 `-->` 或 `--]`）
    fn skip_expr_comment(&mut self) -> Result<()> {
        loop {
            match self.peek() {
                None => {
                    return Err(self.err(self.line, self.col, "Unclosed comment in expression."))
                }
                Some('-') if self.peek_at(1) == Some('-') => match self.peek_at(2) {
                    Some('>') | Some(']') => {
                        self.bump();
                        self.bump();
                        self.bump();
                        return Ok(());
                    }
                    _ => {
                        self.bump();
                    }
                },
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// 跳过空白后的 token 分发（核心词法表）
    fn scan_expr_token_after_ws(
        &mut self,
        ctx: ExprCtx,
        line: u32,
        col: u32,
    ) -> Result<(Tok, u32, u32)> {
        let c = self.peek().unwrap();
        let tok = match c {
            '<' => {
                // LESS_THAN / LESS_THAN_EQUALS（表达式内 `<` 恒为比较符；
                // 标签结束只可能是 `>`，不存在歧义）
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Tok::Lte
                } else {
                    Tok::Lt
                }
            }
            '>' => {
                if self.paren_depth > 0 {
                    // IN_PAREN：NATURAL_GT / NATURAL_GTE
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        Tok::Gte
                    } else {
                        Tok::Gt
                    }
                } else {
                    match ctx {
                        // DIRECTIVE_END：`>` 结束标签（`>=` 不可能 —— `>` 已结束标签）
                        ExprCtx::Tag { .. } => {
                            self.bump();
                            Tok::TagEnd
                        }
                        // 插值内 `>` 是 NATURAL_GT（DIRECTIVE_END 动作的 postInterpolation 分支）
                        ExprCtx::Interp => {
                            self.bump();
                            if self.peek() == Some('=') {
                                self.bump();
                                Tok::Gte
                            } else {
                                Tok::Gt
                            }
                        }
                    }
                }
            }
            ']' => {
                if self.bracket_depth > 0 {
                    self.bump();
                    self.bracket_depth -= 1;
                    Tok::CloseBracket
                } else {
                    match ctx {
                        ExprCtx::Tag { square: true } => {
                            // 方括号语法下 `]` 在深度 0 结束标签（CLOSE_BRACKET 的 DIRECTIVE_END 分支）
                            self.bump();
                            Tok::TagEnd
                        }
                        _ => {
                            // 2.3.28+：角度语法下无配对 `]` → 报错（newUnexpectedClosingTokenException）
                            return Err(self.err(
                                line,
                                col,
                                "You can't have a \"]\" here, as there's nothing open that it could close.",
                            ));
                        }
                    }
                }
            }
            '}' => {
                if self.curly_depth > 0 {
                    self.bump();
                    self.curly_depth -= 1;
                    Tok::CloseCurly
                } else {
                    match ctx {
                        ExprCtx::Interp => {
                            self.bump();
                            Tok::InterpEnd
                        }
                        ExprCtx::Tag { .. } => {
                            return Err(self.err(
                                line,
                                col,
                                "You can't have a \"}\" here, as there's nothing open that it could close.",
                            ));
                        }
                    }
                }
            }
            ')' => {
                self.bump();
                self.paren_depth = self.paren_depth.saturating_sub(1);
                Tok::CloseParen
            }
            '(' => {
                self.bump();
                self.paren_depth += 1;
                Tok::OpenParen
            }
            '[' => {
                self.bump();
                self.bracket_depth += 1;
                Tok::OpenBracket
            }
            '{' => {
                self.bump();
                self.curly_depth += 1;
                Tok::OpenCurly
            }
            '=' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                }
                Tok::Eq
            }
            '!' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Tok::NotEq
                } else {
                    Tok::Exclam
                }
            }
            '?' => {
                self.bump();
                if self.peek() == Some('?') {
                    self.bump();
                    Tok::Exists
                } else {
                    Tok::Builtin
                }
            }
            '+' => {
                self.bump();
                match self.peek() {
                    Some('+') => {
                        self.bump();
                        Tok::PlusPlus
                    }
                    Some('=') => {
                        self.bump();
                        Tok::PlusEq
                    }
                    _ => Tok::Plus,
                }
            }
            '-' => {
                self.bump();
                match self.peek() {
                    Some('-') => {
                        self.bump();
                        Tok::MinusMinus
                    }
                    Some('=') => {
                        self.bump();
                        Tok::MinusEq
                    }
                    Some('>') => {
                        self.bump();
                        Tok::LambdaArrow
                    }
                    Some('&')
                        if self.peek_at(1) == Some('g')
                            && self.peek_at(2) == Some('t')
                            && self.peek_at(3) == Some(';') =>
                    {
                        // `-&gt;` 也是 lambda 箭头
                        self.bump();
                        self.bump();
                        self.bump();
                        self.bump();
                        Tok::LambdaArrow
                    }
                    _ => Tok::Minus,
                }
            }
            '*' => {
                self.bump();
                match self.peek() {
                    Some('*') => {
                        self.bump();
                        Tok::DoubleStar
                    }
                    Some('=') => {
                        self.bump();
                        Tok::TimesEq
                    }
                    _ => Tok::Times,
                }
            }
            '/' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Tok::DivEq
                } else if matches!(ctx, ExprCtx::Tag { .. })
                    && matches!(self.peek(), Some('>') | Some(']'))
                {
                    // `/>` / `/]`：自闭合标签结束（EMPTY_DIRECTIVE_END，最长匹配优先于 DIVIDE）
                    self.bump();
                    Tok::EmptyTagEnd
                } else {
                    Tok::Divide
                }
            }
            '%' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Tok::ModEq
                } else {
                    Tok::Percent
                }
            }
            '&' => {
                // `&lt;` / `&lt;=` / `&gt;` / `&gt;=` / `&amp;&amp;` / `&` / `&&`
                if self.peek_at(1) == Some('l')
                    && self.peek_at(2) == Some('t')
                    && self.peek_at(3) == Some(';')
                {
                    self.bump();
                    self.bump();
                    self.bump();
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        Tok::Lte
                    } else {
                        Tok::Lt
                    }
                } else if self.peek_at(1) == Some('g')
                    && self.peek_at(2) == Some('t')
                    && self.peek_at(3) == Some(';')
                {
                    self.bump();
                    self.bump();
                    self.bump();
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        Tok::Gte
                    } else {
                        Tok::Gt
                    }
                } else if self.peek_at(1) == Some('a')
                    && self.peek_at(2) == Some('m')
                    && self.peek_at(3) == Some('p')
                    && self.peek_at(4) == Some(';')
                    && self.peek_at(5) == Some('&')
                    && self.peek_at(6) == Some('a')
                    && self.peek_at(7) == Some('m')
                    && self.peek_at(8) == Some('p')
                    && self.peek_at(9) == Some(';')
                {
                    // `&amp;&amp;`
                    for _ in 0..10 {
                        self.bump();
                    }
                    Tok::And
                } else {
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                    }
                    Tok::And
                }
            }
            '|' => {
                self.bump();
                if self.peek() == Some('|') {
                    self.bump();
                }
                Tok::Or
            }
            ',' => {
                self.bump();
                Tok::Comma
            }
            ';' => {
                self.bump();
                Tok::Semicolon
            }
            ':' => {
                self.bump();
                Tok::Colon
            }
            '.' => {
                // `...` / `..<` / `..!` / `..*` / `..` / `.`
                self.bump();
                if self.peek() == Some('.') {
                    self.bump();
                    if self.peek() == Some('.') {
                        self.bump();
                        Tok::Ellipsis
                    } else if matches!(self.peek(), Some('<') | Some('!')) {
                        // `..<`（排端范围，FTL.jj DOT_DOT_LESS）与 `..!`（兼容分支）同 token
                        self.bump();
                        Tok::DotDotLess
                    } else if self.peek() == Some('*') {
                        self.bump();
                        Tok::DotDotStar
                    } else {
                        Tok::DotDot
                    }
                } else {
                    Tok::Dot
                }
            }
            '\\' => {
                // 转义标识符起始字符 `\-` `\.` `\:` `\#`（ESCAPED_ID_CHAR）
                if matches!(
                    self.peek_at(1),
                    Some('-') | Some('.') | Some(':') | Some('#')
                ) {
                    Tok::Ident(self.scan_ident())
                } else {
                    // 转义运算符：`\and` / `\lt` / `\lte` / `\gt` / `\gte`
                    let n1 = self.peek_at(1);
                    match n1 {
                        Some('a')
                            if self.peek_at(2) == Some('n') && self.peek_at(3) == Some('d') =>
                        {
                            for _ in 0..4 {
                                self.bump();
                            }
                            Tok::And
                        }
                        Some('l') => {
                            if self.peek_at(2) == Some('t') {
                                self.bump();
                                self.bump();
                                if self.peek() == Some('e') {
                                    self.bump();
                                    Tok::Lte
                                } else {
                                    Tok::Lt
                                }
                            } else {
                                return Err(self.err(
                                    line,
                                    col,
                                    format!("Unexpected character \"\\\\{n1:?}\"."),
                                ));
                            }
                        }
                        Some('g') if self.peek_at(2) == Some('t') => {
                            self.bump();
                            self.bump();
                            if self.peek() == Some('e') {
                                self.bump();
                                Tok::Gte
                            } else {
                                Tok::Gt
                            }
                        }
                        _ => {
                            return Err(self.err(
                                line,
                                col,
                                format!("Unexpected character \"\\\\{n1:?}\"."),
                            ));
                        }
                    }
                }
            }
            '"' | '\'' => {
                let (tok, _) = self.scan_string_token()?;
                tok
            }
            'r' if matches!(self.peek_at(1), Some('"') | Some('\'')) => {
                // RAW_STRING：`r"..."` / `r'...'`
                self.bump(); // r
                let quote = self.peek().unwrap(); // 引号（已确认存在）
                self.bump();
                let mut s = String::new();
                loop {
                    match self.peek() {
                        None => {
                            return Err(self.err(line, col, "Unclosed raw string literal."));
                        }
                        Some(q) if q == quote => {
                            self.bump();
                            break;
                        }
                        Some(q) => {
                            s.push(q);
                            self.bump();
                        }
                    }
                }
                Tok::RawStr(s)
            }
            c if c.is_ascii_digit() => {
                let raw = self.scan_number_raw();
                Tok::Number(raw)
            }
            c if is_ident_start(c) => {
                let name = self.scan_ident();
                match name.as_str() {
                    "true" => Tok::True,
                    "false" => Tok::False,
                    "in" => Tok::In,
                    "as" => Tok::As,
                    "using" => Tok::Using,
                    "lt" => Tok::Lt,
                    "lte" => Tok::Lte,
                    "gt" => Tok::Gt,
                    "gte" => Tok::Gte,
                    _ => Tok::Ident(name),
                }
            }
            c => {
                return Err(self.err(line, col, format!("Unexpected character \"{c}\".")));
            }
        };
        Ok((tok, line, col))
    }

    /// 扫描数字字面量原始文本（含 0x 十六进制、指数、L/F/D/B 后缀；扩展见文件头注释）
    fn scan_number_raw(&mut self) -> String {
        let mut s = String::new();
        // 十进制整数部分
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            s.push(self.bump().unwrap());
        }
        // 0x 十六进制（仅当整数部分恰为 "0"）
        if s == "0" && matches!(self.peek(), Some('x') | Some('X')) {
            self.bump();
            s.push('x');
            while matches!(self.peek(), Some(c) if c.is_ascii_hexdigit()) {
                s.push(self.bump().unwrap());
            }
        } else {
            // 小数部分（`.` 后必须是数字，否则 `.` 留给 DOT）
            if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit())
            {
                s.push(self.bump().unwrap());
                while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    s.push(self.bump().unwrap());
                }
            }
            // 指数 `e[+-]digits`
            if matches!(self.peek(), Some('e') | Some('E')) {
                let sign_ok = matches!(self.peek_at(1), Some('+') | Some('-'))
                    && matches!(self.peek_at(2), Some(c) if c.is_ascii_digit());
                let digit_ok = matches!(self.peek_at(1), Some(c) if c.is_ascii_digit());
                if sign_ok || digit_ok {
                    s.push(self.bump().unwrap());
                    if matches!(self.peek(), Some('+') | Some('-')) {
                        s.push(self.bump().unwrap());
                    }
                    while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                        s.push(self.bump().unwrap());
                    }
                }
            }
        }
        // 类型后缀 L/D/F/B
        if matches!(
            self.peek(),
            Some('l')
                | Some('L')
                | Some('f')
                | Some('F')
                | Some('d')
                | Some('D')
                | Some('b')
                | Some('B')
        ) {
            s.push(self.bump().unwrap());
        }
        s
    }

    /// 扫描字符串 token 的原始内容（不处理插值；含转义序列原样保留）
    fn scan_string_token(&mut self) -> Result<(Tok, u32)> {
        let (line, col) = self.line_col();
        let quote = self.bump().unwrap();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(self.err(line, col, "Unclosed string literal."));
                }
                Some(q) if q == quote => {
                    self.bump();
                    return Ok((Tok::Str(s), col));
                }
                Some('\\') => {
                    s.push('\\');
                    self.bump();
                    match self.peek() {
                        None => return Err(self.err(line, col, "Unclosed string literal.")),
                        Some(c) => {
                            s.push(c);
                            self.bump();
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
    }

    /// 扫描标识符（ID_START_CHAR 与 `\` 转义字符 `\-\.\:\#`；非 ASCII 字母支持）
    pub(crate) fn scan_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                s.push(c);
                self.bump();
            } else if c == '\\'
                && matches!(
                    self.peek_at(1),
                    Some('-') | Some('.') | Some(':') | Some('#')
                )
            {
                // ESCAPED_ID_CHAR：`\-` → `-` 等（反斜杠去除）
                self.bump();
                s.push(self.bump().unwrap());
            } else {
                break;
            }
        }
        s
    }

    /// 扫描简洁注释 `<#-- ... -->` / `[#-- ... --]` 内容。
    /// 角度注释以 `-->` 结束，方括号注释以 `--]` 结束（TERSE_COMMENT_END 语义，
    /// 交叉结束符视为内容，与 JavaCC 的 noparseTag 判定一致）。
    pub(crate) fn scan_comment(&mut self, square: bool) -> Result<(String, u32, u32)> {
        let (line, col) = self.line_col();
        let term: [char; 3] = if square {
            ['-', '-', ']']
        } else {
            ['-', '-', '>']
        };
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err(line, col, "Unclosed comment.")),
                Some('-') if self.peek_at(1) == Some('-') && self.peek_at(2) == Some(term[2]) => {
                    self.bump();
                    self.bump();
                    self.bump();
                    return Ok((s, line, col));
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
    }

    /// 扫描 `<#comment>` / `<#noparse>` 的未解析内容（NO_PARSE 状态），直到匹配的结束标签。
    /// 结束标签：`</#name` / `</name`（也接受 `[/#` / `[` 形式）+ 空白* + `>`/`]`。
    pub(crate) fn scan_unparsed(&mut self, end_name: &str) -> Result<(String, u32, u32)> {
        let (line, col) = self.line_col();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(self.err(
                        line,
                        col,
                        format!("Unclosed \"<#{end_name}>\" (missing \"</#{end_name}>\" or \"</{end_name}>\")."),
                    ));
                }
                Some('<') | Some('[') => {
                    let save = self.save();
                    let n1 = self.peek_at(1);
                    if n1 == Some('/') {
                        self.bump();
                        self.bump();
                        if self.peek() == Some('#') {
                            self.bump();
                        }
                        if let Some(name) = self.read_name() {
                            if name.eq_ignore_ascii_case(end_name) {
                                // 空白* + `>`/`]`
                                let mut ok = false;
                                self.skip_ws();
                                match self.peek() {
                                    Some('>') | Some(']') => {
                                        self.bump();
                                        ok = true;
                                    }
                                    _ => {}
                                }
                                if ok {
                                    return Ok((s, line, col));
                                }
                            }
                        }
                        // 不匹配：回退为普通内容
                        self.restore(&save);
                        s.push(self.bump().unwrap());
                    } else {
                        s.push(self.bump().unwrap());
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
    }
}

/// 标识符起始字符（ID_START_CHAR 近似：字母（含非 ASCII）、`$`、`_`、`@`；
/// Java isLegacyFTLIdStartChar 的 `@`..`Z` 区间 = `@` + A-Z，identifierChars
/// 生成器见 FTL.jj:1427；`\` 转义字符在 scan_ident 内处理）
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '$' || c == '_' || c == '@'
}

/// 标识符续字符（ID_START_CHAR 或 ASCII 数字）
fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

/// 全部内置指令名（大小写不敏感匹配；对应 _CoreAPI.ALL_BUILT_IN_DIRECTIVE_NAMES 的
/// 本实现子集 —— 仅包含本解析器支持的指令）
pub(crate) const DIRECTIVE_NAMES: &[&str] = &[
    "attempt",
    "recover",
    "if",
    "elseif",
    "else",
    "list",
    "items",
    "sep",
    "switch",
    "case",
    "default",
    "assign",
    "global",
    "local",
    "include",
    "import",
    "macro",
    "function",
    "stop",
    "return",
    "break",
    "continue",
    "nested",
    "flush",
    "t",
    "lt",
    "rt",
    "nt",
    "compress",
    "comment",
    "noparse",
    "escape",
    "noescape",
    "autoesc",
    "noautoesc",
    "outputformat",
    "setting",
    "call",
    "foreach",
    "transform",
    "visit",
    "recurse",
    "on",
    "fallback",
    "ftl",
];

/// 需要参数（后跟空白）的指令 —— 非严格语法下 `<name>` 判定用（IF/LIST 等 token 需 BLANK）
pub(crate) const PARAM_DIRECTIVES: &[&str] = &[
    "if",
    "elseif",
    "list",
    "items",
    "switch",
    "case",
    "assign",
    "global",
    "local",
    "include",
    "import",
    "macro",
    "function",
    "stop",
    "return",
    "nested",
    "escape",
    "setting",
    "call",
    "foreach",
    "transform",
    "visit",
    "recurse",
    "on",
    "outputformat",
    "ftl",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(name: &str, text: &str, strict: bool) -> Lexer {
        Lexer::new(name, text, strict)
    }

    fn tokens(l: &mut Lexer, ctx: ExprCtx) -> Vec<Tok> {
        let mut out = Vec::new();
        loop {
            let (t, _, _, _, _) = l.next_expr_token(ctx).unwrap();
            let done = t == Tok::Eof;
            out.push(t);
            if done {
                break;
            }
        }
        out
    }

    #[test]
    fn expr_tokens_basic() {
        let mut l = lex("t", "a + b*2 != (x??) ?name", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Ident("a".into()),
                Tok::Plus,
                Tok::Ident("b".into()),
                Tok::Times,
                Tok::Number("2".into()),
                Tok::NotEq,
                Tok::OpenParen,
                Tok::Ident("x".into()),
                Tok::Exists,
                Tok::CloseParen,
                Tok::Builtin,
                Tok::Ident("name".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn word_operators_and_keywords() {
        let mut l = lex("t", "a lt b gt c and d or e", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        // `and`/`or` 不是 token（仅 `&&`/`\and`/`||`）；lt/gt 是运算符
        assert!(ts.contains(&Tok::Lt));
        assert!(ts.contains(&Tok::Gt));
        assert!(!ts.contains(&Tok::And));
        assert!(!ts.contains(&Tok::Or));
        let mut l = lex("t", "a \\and b || c && d &amp;&amp; e", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts.iter().filter(|t| **t == Tok::And).count(),
            3,
            "\\and、&&、&amp;&amp; 是 And（|| 是 Or）"
        );
        assert_eq!(ts.iter().filter(|t| **t == Tok::Or).count(), 1);
    }

    #[test]
    fn number_forms() {
        let mut l = lex("t", "1 1L 1F 1D 1.5 1e3 0x1A 1..5", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Number("1".into()),
                Tok::Number("1L".into()),
                Tok::Number("1F".into()),
                Tok::Number("1D".into()),
                Tok::Number("1.5".into()),
                Tok::Number("1e3".into()),
                Tok::Number("0x1A".into()),
                Tok::Number("1".into()),
                Tok::DotDot,
                Tok::Number("5".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn string_and_raw_string() {
        let mut l = lex("t", r#""a\n\t\"\\" 'x' r"raw\ny" "#, true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Str("a\\n\\t\\\"\\\\".into()),
                Tok::Str("x".into()),
                Tok::RawStr("raw\\ny".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn gt_ends_tag_outside_parens() {
        let mut l = lex("t", "a > b", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        // `a` 后 `>` 直接结束标签（TagEnd），`b` 不再是表达式 token
        assert_eq!(
            ts,
            vec![
                Tok::Ident("a".into()),
                Tok::TagEnd,
                Tok::Ident("b".into()),
                Tok::Eof
            ]
        );
        let mut l = lex("t", "(a > b)", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert!(ts.contains(&Tok::Gt));
        let mut l = lex("t", "a >= b", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        // 标签内 `>=` 不是 GTE（`>` 结束标签，`=` 留作文本）—— 与 Java 一致
        assert!(!ts.contains(&Tok::Gte));
        assert_eq!(ts[1], Tok::TagEnd);
    }

    #[test]
    fn interp_ctx_closes_with_brace() {
        let mut l = lex("t", "a + }", true);
        let ts = tokens(&mut l, ExprCtx::Interp);
        assert_eq!(
            ts,
            vec![Tok::Ident("a".into()), Tok::Plus, Tok::InterpEnd, Tok::Eof]
        );
    }

    #[test]
    fn curly_bracket_hash_vs_interp_end() {
        let mut l = lex("t", r#"{"a": 1} x"#, true);
        let ts = tokens(&mut l, ExprCtx::Interp);
        assert_eq!(
            ts,
            vec![
                Tok::OpenCurly,
                Tok::Str("a".into()),
                Tok::Colon,
                Tok::Number("1".into()),
                Tok::CloseCurly,
                Tok::Ident("x".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn bracket_list_depth() {
        let mut l = lex("t", "[1, 2] x", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: true });
        assert_eq!(
            ts,
            vec![
                Tok::OpenBracket,
                Tok::Number("1".into()),
                Tok::Comma,
                Tok::Number("2".into()),
                Tok::CloseBracket,
                Tok::Ident("x".into()),
                Tok::Eof,
            ]
        );
        // 方括号语法：深度 0 的 `]` 结束标签
        let mut l = lex("t", "[1, 2]]", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: true });
        assert_eq!(
            ts,
            vec![
                Tok::OpenBracket,
                Tok::Number("1".into()),
                Tok::Comma,
                Tok::Number("2".into()),
                Tok::CloseBracket,
                Tok::TagEnd,
                Tok::Eof,
            ]
        );
        // 角度语法下深度 0 的 `]` 报错（`[1, 2]` 的 `]` 关闭列表字面量，
        // 第二个 `]` 在深度 0 → 报错）
        let mut l = lex("t", "[1, 2]]", true);
        for _ in 0..5 {
            l.next_expr_token(ExprCtx::Tag { square: false }).unwrap();
        }
        let r = l.next_expr_token(ExprCtx::Tag { square: false });
        assert!(r.is_err());
    }

    #[test]
    fn text_scanning_rules() {
        // `a < b` 是文本（严格语法）
        let mut l = lex("t", "a < b", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "a < b");
        assert_eq!(s, TextStop::Eof);
        // 非严格语法 `<if x>` 是标签
        let mut l = lex("t", "x <if y>", false);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x ");
        assert_eq!(s, TextStop::Tag);
        // `<b>`（非指令名）是文本
        let mut l = lex("t", "a <b> c", false);
        let (t, _s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "a <b> c");
        // `$${` → 文本 `$` + 插值
        let mut l = lex("t", "$${x}", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "$");
        assert_eq!(s, TextStop::Interp);
        // `${` 插值开始
        let mut l = lex("t", "ab${x}", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "ab");
        assert_eq!(s, TextStop::Interp);
        // `#{` 传统插值
        let mut l = lex("t", "ab#{x}", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "ab");
        assert_eq!(s, TextStop::Interp);
        // `<#--` 注释标签开头
        let mut l = lex("t", "ab<#-- c -->", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "ab");
        assert_eq!(s, TextStop::Tag);
    }

    #[test]
    fn comment_scanning() {
        let mut l = lex("t", "hello --] world -->", true);
        let (s, _, _) = l.scan_comment(false).unwrap();
        assert_eq!(s, "hello --] world ");
        let mut l = lex("t", "hello --> world --]", true);
        let (s, _, _) = l.scan_comment(true).unwrap();
        assert_eq!(s, "hello --> world ");
        let mut l = lex("t", "unclosed", true);
        assert!(l.scan_comment(false).is_err());
    }

    #[test]
    fn unparsed_scanning() {
        let mut l = lex("t", "a</#noparse>", true);
        let (s, _, _) = l.scan_unparsed("noparse").unwrap();
        assert_eq!(s, "a");
        let mut l = lex("t", "a</noparse>", true);
        let (s, _, _) = l.scan_unparsed("noparse").unwrap();
        assert_eq!(s, "a");
        let mut l = lex("t", "a</#comment>", true);
        let (s, _, _) = l.scan_unparsed("comment").unwrap();
        assert_eq!(s, "a");
        // 不匹配的结束标签视为内容
        let mut l = lex("t", "a</#noparse x>rest</#noparse>", true);
        let (s, _, _) = l.scan_unparsed("noparse").unwrap();
        assert_eq!(s, "a</#noparse x>rest");
    }

    #[test]
    fn non_strict_tag_detection() {
        // 非严格：`<if x>` 标签、`<if>` 文本（IF 需 BLANK）、`<else>` 标签、`<foo>` 文本
        let mut l = lex("t", "x <if y>", false);
        let (_, stop) = l.scan_text_chunk().unwrap();
        assert_eq!(stop, TextStop::Tag);
        assert!(l.starts_tag());
        let mut l = lex("t", "x <if>", false);
        let (t, _) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x <if>");
        let mut l = lex("t", "x <else>", false);
        let (t, _s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x ");
        let mut l = lex("t", "x <else y>", false);
        let (t, _) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x <else y>");
        // 严格：`<if x>` 是文本
        let mut l = lex("t", "x <if y>", true);
        let (t, _) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x <if y>");
    }

    #[test]
    fn escaped_identifiers() {
        let mut l = lex("t", r"a\-b\.c\:d\#e", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(ts[0], Tok::Ident("a-b.c:d#e".into()));
    }

    #[test]
    fn ident_special_chars() {
        // `$`、非 ASCII 都是标识符字符
        let mut l = lex("t", "$foo _bar français x2", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Ident("$foo".into()),
                Tok::Ident("_bar".into()),
                Tok::Ident("français".into()),
                Tok::Ident("x2".into()),
                Tok::Eof,
            ]
        );
    }
}
