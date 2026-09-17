//! 表达式词法 —— next_expr_token 主循环与核心分发表（FM_EXPRESSION 家族状态）。
//!
//! `ctx` 决定 `>`/`]`/`}` 在深度 0 时的语义（标签结束/插值结束/错误）；
//! `>` 语义随方括号/角度语法区分（ExprCtx::Tag.square），详见主文件头注释。

use super::{is_ident_start, ExprCtx, Lexer, Tok};
use crate::error::Result;

impl Lexer {
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
        // Java OPEN_MISPLACED_INTERPOLATION（FTL.jj :1409-1421）：表达式模式中
        // `${`/`#{`/`[=` 是词法错误（TokenMgrError LEXICAL_ERROR，位置=起始符列；
        // jar 实测 parse_needless_interpolation 基线）。注意 `$` 本身是合法标识符
        // 起始字符（`$foo` 变量名），仅 `${` 组合报错
        if (c == '$' || c == '#') && self.peek_at(1) == Some('{')
            || c == '[' && self.peek_at(1) == Some('=')
        {
            let img = if c == '[' {
                "[="
            } else if c == '$' {
                "${"
            } else {
                "#{"
            };
            let closer = if c == '[' { "]" } else { "}" };
            self.bump();
            self.bump();
            return Err(self.err(
                line,
                col,
                format!(
                    "You can't use {img}...{closer} (an interpolation) here as you are \
                     already in FreeMarker-expression-mode. Thus, instead of {img}myExpression{closer}, \
                     just write myExpression. ({img}...{closer} is only used where otherwise static \
                     text is expected, i.e., outside FreeMarker tags and interpolations, or inside \
                     string literals.)"
                ),
            ));
        }
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
                        // 角度语法 DIRECTIVE_END：`>` 结束标签（`>=` 不可能 —— `>` 已结束标签）
                        ExprCtx::Tag { square: false } => {
                            self.bump();
                            Tok::TagEnd
                        }
                        // 方括号语法：标签仅由 `]` 结束，故 `>`/`>=` 为 NATURAL_GT/GTE。
                        // 插值内 `>` 同样是 NATURAL_GT（DIRECTIVE_END 动作的 postInterpolation 分支）。
                        ExprCtx::Tag { square: true } | ExprCtx::Interp => {
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
}
