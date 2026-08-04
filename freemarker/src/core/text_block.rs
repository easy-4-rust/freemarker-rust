//! 文本块 —— 对应 Java `freemarker.core.TextBlock`
//! （accept :65-70；postParseCleanup :140-167 的空白剥离标记；
//! unparsed 标记承载 `<#noparse>`，TextBlock.java:31-33）

use crate::core::exec::{strip_text, ExecOutcome};
use crate::error::Result;

/// 模板文本（对应 TextBlock.java；解析器经 `ElementKind::Text` / `NoParse` 承载；
/// whitespace stripping 标记在解析期决定渲染期是否裁剪）
pub struct TextBlock {
    pub text: String,
    pub strip_before: bool,
    pub strip_after: bool,
    /// 原始结束行（token 行号；Java endLine 在空白剥离时**不更新**，
    /// 而内容裁剪会改变换行数 —— prev/next 链的行号判定须用原始值）
    #[allow(dead_code)]
    pub orig_end_line: u32,
    /// `<#noparse>` 的 unparsed 标记（Java TextBlock unparsed 构造参数；
    /// 渲染期与普通文本同路径）
    #[allow(dead_code)]
    pub unparsed: bool,
}

impl TextBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(
        text: String,
        strip_before: bool,
        strip_after: bool,
        orig_end_line: u32,
        unparsed: bool,
    ) -> Self {
        TextBlock {
            text,
            strip_before,
            strip_after,
            orig_end_line,
            unparsed,
        }
    }

    /// 执行（Java accept :65-70：剥离后写出）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        // Java TextBlock.accept :65-70；裁剪语义对应 postParseCleanup :140-167：
        // opening/trailing 两段在原始文本上独立计算，取其中间段
        // （顺序执行会导致纯空白文本 "\n  " 残留，见 TextBlock.java:148-167）
        let t = strip_text(&self.text, self.strip_before, self.strip_after, env);
        env.emit(t)?;
        Ok(ExecOutcome::Done)
    }
}
