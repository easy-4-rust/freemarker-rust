//! 语法分析器 —— 对应 Java `freemarker.core.FMParser`（FTL.jj 全部产生式）
//!
//! 递归下降实现 docs/03 §3 的 24 个表达式产生式与 §4 的 13 个指令产生式；
//! 词法由 lexer.rs 提供（JavaCC 的 5 词法状态在本实现中压缩为 `ExprCtx` +
//! 括号深度，见 lexer.rs 文件头说明）。
//!
//! 优先级层次（低 → 高，对照 FTL.jj）：or < and < equality < relational <
//! range < additive < multiplicative < unary(not/±) < primary(postfix) < atomic。
//!
//! 解析错误消息：`Parsing error in template "{name}" at line L, column C. {details}`
//! （契约：错误消息必须含模板名 + 行列）。
//!
//! 实现文件按职责拆分（`#[path]` 聚合，参照 core/expression.rs 模式）：
//! - grammar_block.rs —— 块解析（parse_block_impl 主循环）
//! - grammar_expression.rs —— 表达式产生式（优先级链 + atomic）
//! - grammar_expression_args.rs —— 参数列表、lambda、字符串解码、callee
//! - grammar_directive.rs —— 指令分派 + if/list/foreach/items/sep/call
//! - grammar_assign.rs —— assign/macro/parse_call/nested
//! - grammar_control.rs —— switch/attempt/return/stop/include/import
//! - grammar_stripping.rs —— 空白剥离标记（mark_block）
//! - grammar_element_helpers.rs —— 元素树遍历与终结节点检查
//! - grammar_helpers.rs —— 辅助类型与函数（字面量校验、内建名等）

#[path = "grammar_assign.rs"]
mod grammar_assign;
#[path = "grammar_block.rs"]
mod grammar_block;
#[path = "grammar_control.rs"]
mod grammar_control;
#[path = "grammar_directive.rs"]
mod grammar_directive;
#[cfg(test)]
#[path = "grammar_directive_tests.rs"]
mod grammar_directive_tests;
#[path = "grammar_element_helpers.rs"]
mod grammar_element_helpers;
#[path = "grammar_expression.rs"]
mod grammar_expression;
#[path = "grammar_expression_args.rs"]
mod grammar_expression_args;
#[path = "grammar_helpers.rs"]
mod grammar_helpers;
#[cfg(test)]
#[path = "grammar_misc_tests.rs"]
mod grammar_misc_tests;
#[path = "grammar_stripping.rs"]
mod grammar_stripping;
#[cfg(test)]
#[path = "grammar_tests.rs"]
mod grammar_tests;

use crate::core::{Element, ElementKind, ExprKind, MacroDef};
use crate::error::{Result, TemplateError};
use crate::parser::lexer::{ExprCtx, Lexer, TagOpen, TagSyntax, TextStop, Tok};
use crate::template::{Configuration, Template};
use std::collections::HashMap;
use std::rc::Rc;

use grammar_element_helpers::{is_non_outputting, is_ws, sync_macro_defs};
use grammar_helpers::{header_bool, tok_desc, BlockStop, IterCtx};

/// 解析入口 —— 对应 `new FMParser(Configuration, Reader, ParserConfiguration)`。
/// 构造即解析：词法 + 语法全流程，产物为 `Template`（根元素树 + 宏表）。
pub fn parse(cfg: &Rc<Configuration>, name: &str, text: &str) -> Result<Template> {
    let mut parser = Parser::new(cfg, name, text);
    parser.parse_template()
}

/// 解析独立表达式（Java `?eval` 的 FM_EXPRESSION 模式，BuiltInsForStringsMisc.java:70-90：
/// 源码包为 `(...)` 后按表达式词法解析；本实现包为 `${{src}}` 插值解析后提取表达式）。
pub fn parse_expression(cfg: &Rc<Configuration>, src: &str) -> Result<crate::core::Expr> {
    let wrapped = format!("${{{src}}}");
    let t = parse(cfg, "eval", &wrapped)?;
    match t.root.first().map(|e| &e.kind) {
        Some(ElementKind::Interpolation { expr, .. }) => Ok(expr.clone()),
        _ => Err(crate::error::TemplateError::misc(format!(
            "Failed to parse the expression: {src:?}"
        ))),
    }
}

/// 有结束标签形态（END_xxx token）的指令名——Java FTL.jj 的 `<END_* : <END_TAG>...`
/// token 全量（:1038-1100）。无 END 形态的指令（else/elseif/case/default/recover 外
/// 的 visit/on/fallback 等）出现 `</#name>` 时词法层即报畸形标签。
/// 注意：该集合比 getEndTokenDescIfIsEndToken 的 desc 映射（end_tag_descs，仅供
/// 错误消息）大——END_AUTOESC/END_NOAUTOESC/END_OUTPUTFORMAT/END_RECOVER 存在但
/// 无 desc
const END_TAG_NAMES: &[&str] = &[
    "foreach",
    "list",
    "sep",
    "items",
    "switch",
    "if",
    "compress",
    "macro",
    "function",
    "transform",
    "escape",
    "noescape",
    "assign",
    "global",
    "local",
    "attempt",
    "recover",
    "outputformat",
    "autoesc",
    "noautoesc",
    // "trim"：Rust 契约扩展（Java 无 `<#trim>` 块指令）——结束标签随扩展保留
    "trim",
];

/// Java PropertySetting.SETTING_NAMES（PropertySetting.java:43-68）：模板内可设置
/// 的 12 项设置（snake_case 规范键；camelCase 由 canonical_setting_key 归一）。
/// 顺序与 Java 错误消息的 allowed 列表一致（jar 实测 parse_setting_unknown 基线）
const SETTING_NAMES: &[&str] = &[
    "boolean_format",
    "c_format",
    "classic_compatible",
    "date_format",
    "datetime_format",
    "locale",
    "number_format",
    "output_encoding",
    "sql_date_and_time_time_zone",
    "time_format",
    "time_zone",
    "url_escaping_charset",
];

/// 表达式起始 token 集合（FTL.jj 表达式产生式起始 LOOKAHEAD 的 tokenImage）——
/// JavaCC "was expecting one of these patterns" 列表（jar 实测 parse_invalid_char
/// 基线，顺序与 Java 一致）
const EXPRESSION_START_PATTERNS: &[&str] = &[
    "<STRING_LITERAL>",
    "<RAW_STRING>",
    "\"false\"",
    "\"true\"",
    "<INTEGER>",
    "<DECIMAL>",
    "\".\"",
    "\"+\"",
    "\"-\"",
    "\"!\"",
    "\"[\"",
    "\"(\"",
    "\"{\"",
    "<ID>",
];

/// MixedContentElements 的完整 LOOKAHEAD（JavaCC 生成 ParseException 的
/// expectedTokenSequences tokenImage；FTL.jj MixedContentElements 产生式，
/// 顺序与 jar 实测 parse_nested_comment 基线一致，53 项）
const MIXED_CONTENT_PATTERNS: &[&str] = &[
    "<ATTEMPT>",
    "<IF>",
    "<ELSE_IF>",
    "<LIST>",
    "<ITEMS>",
    "<SEP>",
    "<FOREACH>",
    "<SWITCH>",
    "<ASSIGN>",
    "<GLOBALASSIGN>",
    "<LOCALASSIGN>",
    "<_INCLUDE>",
    "<IMPORT>",
    "<FUNCTION>",
    "<MACRO>",
    "<TRANSFORM>",
    "<VISIT>",
    "<STOP>",
    "<RETURN>",
    "<CALL>",
    "<SETTING>",
    "<OUTPUTFORMAT>",
    "<AUTOESC>",
    "<NOAUTOESC>",
    "<COMPRESS>",
    "<COMMENT>",
    "<TERSE_COMMENT>",
    "<NOPARSE>",
    "<END_IF>",
    "<ELSE>",
    "<BREAK>",
    "<CONTINUE>",
    "<SIMPLE_RETURN>",
    "<HALT>",
    "<FLUSH>",
    "<TRIM>",
    "<LTRIM>",
    "<RTRIM>",
    "<NOTRIM>",
    "<SIMPLE_NESTED>",
    "<NESTED>",
    "<SIMPLE_RECURSE>",
    "<RECURSE>",
    "<FALLBACK>",
    "<ESCAPE>",
    "<NOESCAPE>",
    "<UNIFIED_CALL>",
    "<STATIC_TEXT_WS>",
    "<STATIC_TEXT_NON_WS>",
    "<STATIC_TEXT_FALSE_ALARM>",
    "\"${\"",
    "\"#{\"",
    "\"[=\"",
];

/// 根级 MixedContentElements 的 expected 列表（Java 语义 LOOKAHEAD：无打开的块
/// → 无 END_IF/ELSE_IF/ELSE，`<EOF>` 置首；jar 实测 parse_double_close 基线）
const ROOT_MIXED_PATTERNS: &[&str] = &[
    "<EOF>",
    "<ATTEMPT>",
    "<IF>",
    "<LIST>",
    "<ITEMS>",
    "<SEP>",
    "<FOREACH>",
    "<SWITCH>",
    "<ASSIGN>",
    "<GLOBALASSIGN>",
    "<LOCALASSIGN>",
    "<_INCLUDE>",
    "<IMPORT>",
    "<FUNCTION>",
    "<MACRO>",
    "<TRANSFORM>",
    "<VISIT>",
    "<STOP>",
    "<RETURN>",
    "<CALL>",
    "<SETTING>",
    "<OUTPUTFORMAT>",
    "<AUTOESC>",
    "<NOAUTOESC>",
    "<COMPRESS>",
    "<COMMENT>",
    "<TERSE_COMMENT>",
    "<NOPARSE>",
    "<BREAK>",
    "<CONTINUE>",
    "<SIMPLE_RETURN>",
    "<HALT>",
    "<FLUSH>",
    "<TRIM>",
    "<LTRIM>",
    "<RTRIM>",
    "<NOTRIM>",
    "<SIMPLE_NESTED>",
    "<NESTED>",
    "<SIMPLE_RECURSE>",
    "<RECURSE>",
    "<FALLBACK>",
    "<ESCAPE>",
    "<NOESCAPE>",
    "<UNIFIED_CALL>",
    "<STATIC_TEXT_WS>",
    "<STATIC_TEXT_NON_WS>",
    "<STATIC_TEXT_FALSE_ALARM>",
    "\"${\"",
    "\"#{\"",
    "\"[=\"",
];

/// 无参数指令（Java CLOSE_TAG1/CLOSE_TAG2 家族 + 双 token 的 SIMPLE_* 无参版）：
/// `<#name>`（空白* + `>`/`]` 直接闭合）合法。BLANK 家族（PARAM_DIRECTIVES）
/// 之外的指令均属此类
const NOPARAM_DIRECTIVES: &[&str] = &[
    "attempt",
    "recover",
    "sep",
    "compress",
    "comment",
    "default",
    "trim",
    "autoesc",
    "noautoesc",
    "noescape",
    "noparse",
    "else",
    "break",
    "continue",
    "flush",
    "t",
    "lt",
    "rt",
    "nt",
    "fallback",
    "nested",
    "recurse",
    "return",
    "stop",
    "ftl",
];

/// 允许自闭合（`<#name/>`）的指令——Java CLOSE_TAG2 家族 + SIMPLE_* 无参版
/// （CLOSE_TAG1 不含 `/`：`<#compress/>` 报畸形）
const SELF_CLOSE_DIRECTIVES: &[&str] = &[
    "else", "break", "continue", "flush", "t", "lt", "rt", "nt", "fallback", "nested", "recurse",
    "return", "stop",
];

/// 递归下降解析器（对应 Java FMParser 的字段 + 产生式方法）
struct Parser<'a> {
    lexer: Lexer,
    cfg: &'a Rc<Configuration>,
    name: String,
    /// 宏表（`template.addMacro` 语义；key 为宏名）
    macros: HashMap<String, MacroDef>,
    /// `[#ftl encoding=...]` 设置的编码（写入 Template.encoding）
    encoding: Option<String>,
    /// `[#ftl ns_prefixes=...]` 的命名空间前缀映射（prefix → URI；写入 Template）
    ns_prefixes: HashMap<String, String>,
    /// whitespace_stripping（`[#ftl strip_whitespace=false]` 可覆盖；docs/08 §5.2）
    strip_ws: bool,
    /// inMacro / inFunction 嵌套计数（Macro 语义校验；互斥）
    in_macro: u32,
    in_function: u32,
    /// breakable/continuable 嵌套计数（#list with as / #items / #switch / #foreach）
    loop_nesting: u32,
    /// continuable（#continue 合法）嵌套计数 —— Java FTL.jj 的
    /// `continuableDirectiveNesting`：#switch 只算 breakable、不算 continuable，
    /// 因此 #continue 在 #switch 内非法（breakableDirectiveNesting 除外）
    continue_nesting: u32,
    /// 迭代块上下文栈（Java ParserIteratorBlockContext；#items/#sep 的嵌套校验）
    iter_stack: Vec<IterCtx>,
    /// 当前表达式词法上下文（标签参数区 vs 插值内部）
    ctx: ExprCtx,
    /// 前瞻 token 缓冲（最多 2 个；`next_expr_token` 有消耗性）。
    /// 元组 = (token, 起始行, 起始列, 结束行, 结束列) —— 结束位置供
    /// `<@callee>` 的 NO_SPACE_EXPRESSION 相邻性判定（FTL.jj 1614 TERMINATING_WHITESPACE）
    buf: Vec<(Tok, u32, u32, u32, u32)>,
    /// 最近一个被消费 token 的结束位置（`next_tok` 维护；callee 相邻性判定用）
    last_tok_end: (u32, u32),
    /// 最近一个标签的起始位置（指令 span 用）
    tag_pos: (u32, u32),
    /// 命名参数值表达式嵌套深度（NAMED_PARAMETER_EXPRESSION 语义：`!`+空白终止）
    named_arg_depth: u32,
    /// 就地元素（`<#sep>`）自动收尾时上抛的外层块停止信号（Java Sep() 的
    /// MixedContentElements：END_SEP 可选，父块结束标签/终止指令由外层块接手）
    pending_stop: Option<BlockStop>,
}

impl<'a> Parser<'a> {
    fn new(cfg: &'a Rc<Configuration>, name: &str, text: &str) -> Self {
        Parser {
            lexer: Lexer::new(name, text, cfg.settings.strict_syntax),
            cfg,
            name: name.to_string(),
            macros: HashMap::new(),
            encoding: None,
            ns_prefixes: HashMap::new(),
            strip_ws: cfg.settings.whitespace_stripping,
            in_macro: 0,
            in_function: 0,
            loop_nesting: 0,
            continue_nesting: 0,
            iter_stack: Vec::new(),
            ctx: ExprCtx::Tag { square: false },
            buf: Vec::new(),
            last_tok_end: (1, 1),
            tag_pos: (1, 1),
            named_arg_depth: 0,
            pending_stop: None,
        }
    }

    /// 解析错误：Syntax error in template "<name>" in line L, column C: <details>
    /// （Java ParseException.getMessage 格式，jar 实测）
    fn err(&self, line: u32, col: u32, details: impl Into<String>) -> TemplateError {
        TemplateError::Parse {
            template: self.name.clone(),
            line,
            col,
            message: details.into(),
        }
    }

    /// 已确立的标签语法（首个标签决定；docs/03 §2.3 规则 3）
    fn tag_square(&self) -> bool {
        self.lexer.tag_syntax == Some(TagSyntax::Square)
    }

    /// 首个标签确立标签语法（`<`→Angle，`[`→Square）
    fn establish_tag_syntax(&mut self, square: bool) {
        if self.lexer.tag_syntax.is_none() {
            self.lexer.tag_syntax = Some(if square {
                TagSyntax::Square
            } else {
                TagSyntax::Angle
            });
        }
    }

    /// 标签打开后：确立语法 + 切换到标签参数词法上下文
    fn enter_tag(&mut self, square: bool) {
        self.establish_tag_syntax(square);
        self.ctx = ExprCtx::Tag {
            square: self.tag_square(),
        };
        debug_assert!(self.buf.is_empty());
    }

    // -----------------------------------------------------------------------
    // token 流辅助
    // -----------------------------------------------------------------------

    fn peek_tok(&mut self) -> Result<(Tok, u32, u32)> {
        if self.buf.is_empty() {
            let (t, l, c, el, ec) = self.lexer.next_expr_token(self.ctx)?;
            self.buf.push((t, l, c, el, ec));
        }
        Ok((self.buf[0].0.clone(), self.buf[0].1, self.buf[0].2))
    }

    fn peek_tok2(&mut self) -> Result<(Tok, u32, u32)> {
        self.peek_tok()?;
        while self.buf.len() < 2 {
            let (t, l, c, el, ec) = self.lexer.next_expr_token(self.ctx)?;
            self.buf.push((t, l, c, el, ec));
        }
        Ok((self.buf[1].0.clone(), self.buf[1].1, self.buf[1].2))
    }

    fn next_tok(&mut self) -> Result<(Tok, u32, u32)> {
        let entry = if !self.buf.is_empty() {
            Some(self.buf.remove(0))
        } else {
            let (t, l, c, el, ec) = self.lexer.next_expr_token(self.ctx)?;
            Some((t, l, c, el, ec))
        };
        if let Some((t, l, c, el, ec)) = entry {
            self.last_tok_end = (el, ec);
            Ok((t, l, c))
        } else {
            unreachable!()
        }
    }

    /// 期望指定 token；失败报错（含位置与期望内容）
    fn expect_tok(&mut self, tok: Tok, what: &str) -> Result<(u32, u32)> {
        let (t, l, c) = self.next_tok()?;
        if t == tok {
            Ok((l, c))
        } else {
            Err(self.err(
                l,
                c,
                format!("Expected {what}, but found {}.", tok_desc(&t)),
            ))
        }
    }

    /// 标签结束（token 形式）：`>` 结束标签
    fn expect_tag_end(&mut self) -> Result<(u32, u32)> {
        let (t, l, c) = self.next_tok()?;
        match t {
            Tok::TagEnd => Ok((l, c)),
            Tok::EmptyTagEnd => Err(self.err(
                l,
                c,
                "The tag can't be self-closing (\" />\"): this directive needs a body.",
            )),
            other => Err(self.err(
                l,
                c,
                format!(
                    "Expected \">\" to close the tag, but found {}.",
                    tok_desc(&other)
                ),
            )),
        }
    }

    /// 宽松标签结束：`>` 或 `/>`（对应 LooseDirectiveEnd）
    fn loose_end(&mut self) -> Result<(u32, u32)> {
        let (t, l, c) = self.next_tok()?;
        match t {
            Tok::TagEnd | Tok::EmptyTagEnd => Ok((l, c)),
            other => Err(self.err(
                l,
                c,
                format!(
                    "Expected \">\" or \"/>\" to close the tag, but found {}.",
                    tok_desc(&other)
                ),
            )),
        }
    }

    /// 无参数指令的标签结束（直接读原始字符；调用前不得 lex 过 token）
    fn expect_tag_end_raw(&mut self) -> Result<()> {
        match self.lexer.try_read_tag_end() {
            Some(_) => Ok(()),
            None => {
                let (l, c) = self.lexer.line_col();
                Err(self.err(l, c, "Expected \">\" or \"/>\" to close the tag."))
            }
        }
    }

    // -----------------------------------------------------------------------
    // 模板根（Root 产生式）
    // -----------------------------------------------------------------------

    fn parse_template(&mut self) -> Result<Template> {
        // FTL 头部（HeaderElement 产生式）：仅允许模板开头（前导空白后）
        let save = self.lexer.save();
        let (text, stop, _, _) = self.next_text_chunk()?;
        let mut header_parsed = false;
        if stop == TextStop::Tag && text.trim().is_empty() {
            let open = self.lexer.read_tag_open();
            if let TagOpen::Dir { square } = open {
                if let Some(name) = self.lexer.read_name() {
                    if name.eq_ignore_ascii_case("ftl") {
                        self.enter_tag(square);
                        self.parse_ftl_header()?;
                        header_parsed = true;
                    }
                }
            }
        }
        if !header_parsed {
            self.lexer.restore(&save);
        }

        let (root, _) = self.parse_block(&[], &[])?;
        let mut root = root;
        Self::mark_stripping(&mut root, self.strip_ws);
        // Java TextBlock.postParseCleanup：<#lt>/<#rt>/<#t>/<#nt> 解析期消费
        // （TrimInstruction "does nothing at render-time, only parse-time"，
        //  直接改写相邻文本块并调整剥离标记）；随后每层移除 ignorable 元素
        // （TemplateElement.postParseCleanup :404-414：全空白文本夹在非输出元素之间
        //  或顶层边界时删除 —— 与剥离顺序一致：先清理后移除）
        Self::remove_ignorable(&mut root, true);
        // Java：宏定义在解析期已 clone 进注册表（Template.macros），空白剥离标记只打在
        // 树上 ElementKind::Macro 的副本上；须同步回注册表，否则宏体不剥离（hashconcat 用例）
        sync_macro_defs(&mut root, &mut self.macros);

        let mut template = Template::new(
            self.name.clone(),
            root,
            std::mem::take(&mut self.macros),
            self.cfg.clone(),
        );
        template.encoding = self.encoding.take();
        template.ns_prefixes = std::mem::take(&mut self.ns_prefixes);
        Ok(template)
    }

    /// 移除 ignorable 元素（Java isIgnorable：TextBlock.java:349-366 全空白判定 +
    /// nonOutputtingType :374-381；TrimInstruction.isIgnorable=true 恒被移除）。
    /// 逐个移除并实时更新相邻关系（Java :410-419 逐个搬移）。
    fn remove_ignorable(els: &mut Vec<Element>, is_root: bool) {
        let mut i = 0;
        while i < els.len() {
            let removable = match &els[i].kind {
                ElementKind::TrimLineStart
                | ElementKind::NoTrimLineStart
                | ElementKind::TrimLineEnd
                | ElementKind::LeftTrimLine => true,
                ElementKind::Text { text, .. } | ElementKind::NoParse { text, .. } => {
                    // Java isIgnorable（TextBlock.java:349-366）：空文本恒移除；
                    // 全空白文本仅在顶层边界或两侧非输出元素时移除
                    if text.is_empty() {
                        true
                    } else if !text.chars().all(is_ws) {
                        false
                    } else {
                        let prev_ok =
                            (i == 0 && is_root) || (i > 0 && is_non_outputting(&els[i - 1]));
                        let next_ok = (i + 1 == els.len() && is_root)
                            || (i + 1 < els.len() && is_non_outputting(&els[i + 1]));
                        prev_ok && next_ok
                    }
                }
                _ => false,
            };
            if removable {
                els.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// `[#ftl]` 头部：`key = expr`* 或直接结束；`inFTLHeader` 的 eatNewline 语义
    fn parse_ftl_header(&mut self) -> Result<()> {
        loop {
            let (t, l, c) = self.peek_tok()?;
            match t {
                Tok::TagEnd | Tok::EmptyTagEnd => {
                    self.next_tok()?;
                    break;
                }
                Tok::Ident(key) => {
                    self.next_tok()?;
                    self.expect_tok(Tok::Eq, "\"=\" after the FTL header parameter")?;
                    let value = self.expression()?;
                    match key.to_ascii_lowercase().as_str() {
                        "encoding" => match &value.kind {
                            ExprKind::Str(s) => self.encoding = Some(s.clone()),
                            _ => {
                                return Err(self.err(
                                    l,
                                    c,
                                    "Expected a string constant for \"encoding\".",
                                ))
                            }
                        },
                        "strip_whitespace" | "stripwhitespace" | "strip_text" | "striptext" => {
                            match header_bool(&value) {
                                Some(b) => self.strip_ws = b,
                                None => {
                                    // Java checkBooleanParam（FTL.jj :515，jar 实测
                                    // parse_ftl_header_bad 基线）：位置 = 值表达式
                                    return Err(self.err(
                                        value.span.line,
                                        value.span.col,
                                        "Expecting boolean (true/false) parameter",
                                    ));
                                }
                            }
                        }
                        "strict_syntax" | "strictsyntax" => match header_bool(&value) {
                            Some(b) => self.lexer.strict_syntax = b,
                            None => {
                                // Java checkBooleanParam（FTL.jj :515）
                                return Err(self.err(
                                    value.span.line,
                                    value.span.col,
                                    "Expecting boolean (true/false) parameter",
                                ));
                            }
                        },
                        // 渲染期设置（auto_esc / output_format / attributes）：
                        // 本实现解析并忽略（渲染引擎尚未实现；文档化偏差）
                        "auto_esc" | "autoesc" | "output_format" | "outputformat"
                        | "attributes" => {}
                        // ns_prefixes：hash literal `{"D": "...", "N": "..."}` → 前缀映射
                        // （Java Template.addNsPrefix；供 XML 节点查询解析前缀）
                        "ns_prefixes" | "nsprefixes" => {
                            match &value.kind {
                                ExprKind::HashLit(entries) => {
                                    for (k, v) in entries {
                                        // 键可为字符串字面量或标识符（Java hash literal
                                        // 两种写法等价：{"n": ...} / {n: ...}）
                                        let prefix = match &k.kind {
                                            ExprKind::Str(s) => s.clone(),
                                            ExprKind::Ident(i) => i.clone(),
                                            _ => continue,
                                        };
                                        let ExprKind::Str(uri) = &v.kind else {
                                            return Err(self.err(
                                                l,
                                                c,
                                                "Expected a string constant for the namespace URI in \"ns_prefixes\".",
                                            ));
                                        };
                                        self.ns_prefixes.insert(prefix.clone(), uri.clone());
                                    }
                                }
                                _ => {
                                    return Err(self.err(
                                        l,
                                        c,
                                        "Expected a hash literal for \"ns_prefixes\".",
                                    ))
                                }
                            }
                        }
                        other => {
                            return Err(self.err(
                                l,
                                c,
                                format!("Unknown FTL header parameter: {other}."),
                            ))
                        }
                    }
                }
                other => {
                    return Err(self.err(
                        l,
                        c,
                        format!(
                            "Expected an FTL header parameter or the closing \">\", but found {}.",
                            tok_desc(&other)
                        ),
                    ))
                }
            }
        }
        // eatNewline：吞掉头部后的空白含换行（FTL.jj eatNewline）
        loop {
            match self.lexer.peek() {
                Some(c) if c == ' ' || c == '\t' || c == '\r' || c == '\n' => {
                    if c == '\r' {
                        self.lexer.bump();
                        if self.lexer.peek() == Some('\n') {
                            self.lexer.bump();
                        }
                        break;
                    }
                    if c == '\n' {
                        self.lexer.bump();
                        break;
                    }
                    self.lexer.bump();
                }
                _ => break,
            }
        }
        Ok(())
    }
}
