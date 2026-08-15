//! 表达式辅助方法：参数列表、lambda、字符串解码与插值、callee 表达式。

use super::grammar_helpers::{find_matching_brace, tok_desc};
use super::Parser;
use crate::core::{Expr, ExprKind, StrPart};
use crate::error::Result;
use crate::parser::lexer::{ExprCtx, Tok};
use crate::span::Span;

impl<'a> Parser<'a> {
    pub(crate) fn positional_args(&mut self, allow_lambda: bool) -> Result<Vec<Expr>> {
        let mut args: Vec<Expr> = Vec::new();
        if self.at_expr_start(allow_lambda)? {
            args.push(self.parse_arg(allow_lambda)?);
            loop {
                if self.peek_tok()?.0 == Tok::Comma {
                    self.next_tok()?;
                }
                if !self.at_expr_start(allow_lambda)? {
                    break;
                }
                args.push(self.parse_arg(allow_lambda)?);
            }
        }
        Ok(args)
    }

    pub(crate) fn parse_arg(&mut self, allow_lambda: bool) -> Result<Expr> {
        if allow_lambda && self.at_lambda_start()? {
            self.lambda()
        } else {
            self.expression()
        }
    }

    /// 当前 token 是否可开始一个表达式（DefaultTo/参数列表的前瞻；
    /// 含一元 +/-/! —— Java UnaryPlusMinusExpression/NotExpression 可作参数首 token，
    /// 如 `?then(1, -x)`、`[1, -1]`、`join(1..-1, ...)`）
    pub(crate) fn at_expr_start(&mut self, allow_lambda: bool) -> Result<bool> {
        if allow_lambda && self.at_lambda_start()? {
            return Ok(true);
        }
        let (t, _, _) = self.peek_tok()?;
        Ok(matches!(
            t,
            Tok::Ident(_)
                | Tok::Number(_)
                | Tok::Str(_)
                | Tok::RawStr(_)
                | Tok::True
                | Tok::False
                | Tok::OpenParen
                | Tok::OpenBracket
                | Tok::OpenCurly
                | Tok::Dot
                | Tok::Minus
                | Tok::Plus
                | Tok::Exclam
        ))
    }

    /// lambda 前瞻：`x ->`、`(x) ->` 或 `(a, b) ->`（对应 LambdaExpressionParameterList）。
    /// 前瞻开始前取出全部前瞻缓冲并保存词法位置，前瞻期间全部重新词法，
    /// 结束后缓冲与词法位置一并恢复（对调用方完全无副作用）。
    pub(crate) fn at_lambda_start(&mut self) -> Result<bool> {
        let save = self.lexer.save();
        let saved_buf: Vec<(Tok, u32, u32, u32, u32)> = self.buf.drain(..).collect();
        let t = self.next_tok()?.0;
        let ok = match t {
            Tok::Ident(_) => self.peek_tok()?.0 == Tok::LambdaArrow,
            Tok::OpenParen => {
                let mut found = false;
                if matches!(self.next_tok()?.0, Tok::Ident(_)) {
                    if self.peek_tok()?.0 == Tok::Comma {
                        // (a, b, ...) -> ：全 Ident + 逗号序列 + `) ->`
                        self.next_tok()?;
                        loop {
                            if !matches!(self.next_tok()?.0, Tok::Ident(_)) {
                                break;
                            }
                            if self.peek_tok()?.0 == Tok::Comma {
                                self.next_tok()?;
                                continue;
                            }
                            break;
                        }
                        found = self.peek_tok()?.0 == Tok::CloseParen
                            && self.peek_tok2()?.0 == Tok::LambdaArrow;
                    } else {
                        found = self.next_tok()?.0 == Tok::CloseParen
                            && self.peek_tok()?.0 == Tok::LambdaArrow;
                    }
                }
                found
            }
            _ => false,
        };
        self.lexer.restore(&save);
        self.buf = saved_buf;
        Ok(ok)
    }

    /// LocalLambdaExpression：`x -> expr` / `(x, y) -> expr`（对应 Java
    /// LambdaExpressionParameterList + LocalLambdaExpression，FTL.jj 2326-2355）
    pub(crate) fn lambda(&mut self) -> Result<Expr> {
        let (t, l, c) = self.next_tok()?;
        let mut params: Vec<String> = Vec::new();
        match t {
            Tok::OpenParen => {
                loop {
                    let (Tok::Ident(p), _, _) = self.next_tok()? else {
                        return Err(self.err(l, c, "Expected a lambda parameter name."));
                    };
                    params.push(p);
                    if self.peek_tok()?.0 == Tok::Comma {
                        self.next_tok()?;
                        continue;
                    }
                    break;
                }
                self.expect_tok(Tok::CloseParen, "\")\" to close the lambda parameter list")?;
            }
            Tok::Ident(p) => params.push(p),
            other => {
                return Err(self.err(
                    l,
                    c,
                    format!(
                        "Expected a lambda parameter name, but found {}.",
                        tok_desc(&other)
                    ),
                ))
            }
        }
        self.expect_tok(Tok::LambdaArrow, "\"->\" after the lambda parameter")?;
        let body = self.expression()?;
        Ok(Expr::new(
            ExprKind::Lambda {
                params,
                body: Box::new(body),
            },
            Span::new(l, c),
        ))
    }

    /// IdentifierOrStringLiteral：`<#assign>`/`<#macro>` 的名称（Ident 或 Str）
    pub(crate) fn ident_or_string_literal(&mut self) -> Result<(String, Span)> {
        let (t, l, c) = self.next_tok()?;
        match t {
            Tok::Ident(n) => Ok((n, Span::new(l, c))),
            Tok::Str(raw) => {
                let decoded = self.decode_string(&raw, l, c)?;
                Ok((decoded, Span::new(l, c)))
            }
            other => Err(self.err(
                l,
                c,
                format!(
                    "Expected a name (identifier or string literal), but found {}.",
                    tok_desc(&other)
                ),
            )),
        }
    }

    /// 字符串字面量解码（对应 StringUtil.FTLStringLiteralDec + `\uXXXX` 扩展）
    pub(crate) fn decode_string(&self, raw: &str, line: u32, col: u32) -> Result<String> {
        let mut out = String::new();
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            let esc = chars.next().ok_or_else(|| {
                self.err(
                    line,
                    col,
                    "The last character of string literal is backslash",
                )
            })?;
            match esc {
                '"' => out.push('"'),
                '\'' => out.push('\''),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'f' => out.push('\u{000c}'),
                'b' => out.push('\u{0008}'),
                'g' => out.push('>'),
                'l' => out.push('<'),
                'a' => out.push('&'),
                '{' => out.push('{'),
                '=' => out.push('='),
                'x' => {
                    // \xHHHH（1-4 位十六进制；Java StringUtil.FTLStringLiteralDec :625-648
                    // 的 `z = idx+3` 上限 —— `\x010C` = U+010C）
                    let mut v: u32 = 0;
                    let mut n = 0;
                    for _ in 0..4 {
                        match chars.clone().next() {
                            Some(h) if h.is_ascii_hexdigit() => {
                                v = v * 16 + h.to_digit(16).unwrap();
                                chars.next();
                                n += 1;
                            }
                            _ => break,
                        }
                    }
                    if n == 0 {
                        return Err(self.err(line, col, "Invalid \\x escape in a string literal"));
                    }
                    out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
                }
                'u' => {
                    // \uXXXX（4 位十六进制；扩展，Java FTLStringLiteralDec 不支持）
                    let mut v: u32 = 0;
                    let mut n = 0;
                    for _ in 0..4 {
                        match chars.next() {
                            Some(h) if h.is_ascii_hexdigit() => {
                                v = v * 16 + h.to_digit(16).unwrap();
                                n += 1;
                            }
                            Some(_) | None => break,
                        }
                    }
                    if n != 4 {
                        return Err(self.err(
                            line,
                            col,
                            "Invalid \\u escape in a string literal (expected 4 hex digits)",
                        ));
                    }
                    out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
                }
                other => {
                    return Err(self.err(
                        line,
                        col,
                        format!("Invalid escape sequence (\\{other}) in a string literal"),
                    ))
                }
            }
        }
        Ok(out)
    }

    /// 字符串插值：解码后内容中找 `${...}`（StringLiteral(interpolate=true) 语义；
    /// 与 Java 相同：先解码再找 `${`，`\${` 转义后的歧义与 Java 一致）
    pub(crate) fn interpolate_string(
        &mut self,
        decoded: String,
        line: u32,
        col: u32,
    ) -> Result<ExprKind> {
        let mut parts: Vec<StrPart> = Vec::new();
        let mut rest: &str = &decoded;
        let mut text = String::new();
        loop {
            match rest.find("${") {
                None => {
                    text.push_str(rest);
                    break;
                }
                Some(i) => {
                    text.push_str(&rest[..i]);
                    let after = &rest[i + 2..];
                    let (inner, tail) = match find_matching_brace(after) {
                        Some(pair) => pair,
                        None => {
                            return Err(self.err(
                                line,
                                col,
                                "Unclosed \"${\" interpolation in a string literal.",
                            ))
                        }
                    };
                    let inner_expr = self.parse_sub_expression(inner)?;
                    if !text.is_empty() {
                        parts.push(StrPart::Text(std::mem::take(&mut text)));
                    }
                    parts.push(StrPart::Interp(Box::new(inner_expr)));
                    rest = tail;
                }
            }
        }
        if parts.is_empty() {
            Ok(ExprKind::Str(decoded))
        } else {
            if !text.is_empty() {
                parts.push(StrPart::Text(text));
            }
            Ok(ExprKind::InterpStr(parts))
        }
    }

    /// 在子文本上解析插值内的表达式（Java StringLiteral.parseValue 的 sub-parser 语义）
    pub(crate) fn parse_sub_expression(&self, inner: &str) -> Result<Expr> {
        let mut sub = Parser::new(self.cfg, &self.name, inner);
        sub.lexer.strict_syntax = self.lexer.strict_syntax;
        sub.ctx = ExprCtx::Interp;
        let e = sub.expression()?;
        let (t, l, c) = sub.peek_tok()?;
        if t == Tok::Eof || t == Tok::InterpEnd {
            Ok(e)
        } else {
            Err(sub.err(
                l,
                c,
                format!("Unexpected {} in a string interpolation.", tok_desc(&t)),
            ))
        }
    }
}
