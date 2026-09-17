//! 字面量与块内容扫描 —— 数字/字符串原始文本、标识符、注释与 noparse 内容
//! （对应 JavaCC NUMBER/STRING/RAW_STRING/TERSE_COMMENT/NO_PARSE 各 token 生成动作）。

use super::{is_ident_continue, Lexer, Tok};
use crate::error::Result;

impl Lexer {
    /// 扫描数字字面量原始文本（含 0x 十六进制、指数、L/F/D/B 后缀；扩展见文件头注释）
    pub(crate) fn scan_number_raw(&mut self) -> String {
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

    /// Java TokenMgrError.addEscapes（TokenMgrError.java:73-106）：不可打印字符转义，
    /// `"`/`'`/`\` 转义，其余原样（`\0` 跳过）
    fn add_escapes(s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '\0' => {}
                '\u{08}' => out.push_str("\\b"),
                '\t' => out.push_str("\\t"),
                '\n' => out.push_str("\\n"),
                '\u{0c}' => out.push_str("\\f"),
                '\r' => out.push_str("\\r"),
                '"' => out.push_str("\\\""),
                '\'' => out.push_str("\\'"),
                '\\' => out.push_str("\\\\"),
                c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                    out.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => out.push(c),
            }
        }
        out
    }

    /// 扫描字符串 token 的原始内容（不处理插值；含转义序列原样保留）
    pub(crate) fn scan_string_token(&mut self) -> Result<(Tok, u32)> {
        let (_line, col) = self.line_col();
        let quote = self.bump().unwrap();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    // Java TokenMgrError.LexicalError（EOF 分支）：errorAfter =
                    // 起始引号 + 已匹配内容（addEscapes 转义）；位置 = EOF 处
                    // （jar 实测 parse_unclosed_string 基线）
                    return Err(self.err(
                        self.line,
                        self.col,
                        format!(
                            "Lexical error: encountered <EOF> after \"{}\".",
                            Self::add_escapes(&format!("{quote}{s}"))
                        ),
                    ));
                }
                Some(q) if q == quote => {
                    self.bump();
                    return Ok((Tok::Str(s), col));
                }
                Some('\\') => {
                    s.push('\\');
                    self.bump();
                    match self.peek() {
                        None => {
                            // `\` 后 EOF：errorAfter 含尾部 `\`（Java 同样以 token
                            // 已匹配文本为准）
                            return Err(self.err(
                                self.line,
                                self.col,
                                format!(
                                    "Lexical error: encountered <EOF> after \"{}\".",
                                    Self::add_escapes(&format!("{quote}{s}\\"))
                                ),
                            ));
                        }
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
                None => {
                    // Java UnparsedContent（FTL.jj :4411）：`Unclosed "{start.image}"`，
                    // 位置 = 注释 token 起始（jar 实测 parse_unclosed_comment 基线）。
                    // Rust 注释形态恒为 `<#--`/`[#--`（4 字符，starts_tag 限定）
                    let open = if square { "[#--" } else { "<#--" };
                    return Err(self.err(
                        line,
                        col.saturating_sub(4).max(1),
                        format!("Unclosed \"{open}\""),
                    ));
                }
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
