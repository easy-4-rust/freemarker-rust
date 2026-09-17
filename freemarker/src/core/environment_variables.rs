//! 环境变量解析链、命名空间访问器、局部上下文栈、输出捕获与转义栈
//! （对应 Java `Environment.getVariable` :2460-2487 / write :3666 / escape 处理）。

use super::environment_loop::{BodyCtx, LocalEntry};
use super::environment_macro_args::ensure_output_limit;
use super::environment_macros::{MacroValue, Namespace};
use super::environment_models::model_to_string;
use super::environment_stack_trace::collect_ident_names;
use super::EscapeState;
use super::LoopCtx;
use crate::core::eval;
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use std::cell::RefCell;
use std::rc::Rc;

impl<'a> super::Environment<'a> {
    // ---------------------------------------------------------------------
    // 变量解析（Java getVariable :2460-2487 / getGlobalVariable / getDataModelOrSharedVariable）
    // ---------------------------------------------------------------------

    /// 变量解析链（docs/04 §3，对应 Java `getVariable`）：
    /// ① 局部上下文栈（自顶向下：循环变量/`<#nested>` 体参数）→ ② 当前宏帧局部变量（宏参数与
    /// `<#local>`，Java getNullableLocalVariable :2426-2442）→ ③ 当前命名空间（`<#assign>` 变量与宏）
    /// → ④ 全局命名空间（`<#global>`）→ ⑤ 根数据模型 → ⑥ 共享变量 → ⑦ 未找到 Err(InvalidReference)
    /// （消息含模板名/行列由渲染层 attach_location 拼接）。
    pub fn get_variable(&self, name: &str) -> Result<TModel> {
        // ① 局部上下文（自顶向下）
        for entry in self.local_stack.iter().rev() {
            if let Some(m) = entry.get(name, self.settings.fallback_on_null_loop_variable) {
                return Ok(m);
            }
        }
        // ② 当前宏帧局部变量
        if let Some(frame) = self.macro_frames.last() {
            if let Some(m) = frame.get_local_variable(name) {
                return Ok(m);
            }
        }
        // ③ 当前命名空间
        if let Some(m) = self.current_ns.get_member(name) {
            return Ok(m);
        }
        // ④ 全局命名空间
        if let Some(m) = self.global_ns.get_member(name) {
            return Ok(m);
        }
        // ⑤ 根数据模型
        if let Ok(h) = self.root.get_hash() {
            if let Some(m) = h.get(name)? {
                return Ok(m);
            }
        }
        // ⑥ 共享变量（Java getDataModelOrSharedVariable :2568-2578）
        if let Some(m) = self.template.configuration.shared_vars.get(name) {
            return Ok(m.clone());
        }
        // ⑦ 未找到 —— Java Environment.getVariable（:2460-2472）返回 null 不抛错，
        // 错误在使用点抛出（EvalUtil.coerceModelToTextualCommon / modelToBoolean 等）。
        // 本引擎 strict 模式在此抛 InvalidReference（等效）；classic 兼容模式按 Java
        // 语义返回 nothing，由使用点回退（插值 → ""、布尔 → false 等）。
        if self.settings.classic_compatible {
            return Ok(TModel::nothing());
        }
        Err(TemplateError::invalid_reference(name))
    }

    /// 宏值解析快路径（`<@m>` 调用热路径）——与 get_variable 相同的解析链，
    /// 但直接取回 Rc<MacroValue>（跳过 macro_model TModel 构造 + 后续 downcast）。
    /// 名字解析为宏值 → Some；解析为其他值或未找到 → None（调用方回退
    /// get_variable 常规路径，错误语义不变）。
    pub fn get_macro(&self, name: &str) -> Option<Rc<MacroValue>> {
        // ① 局部上下文（自顶向下；值可为宏值 TModel）
        for entry in self.local_stack.iter().rev() {
            if let Some(m) = entry.get(name, self.settings.fallback_on_null_loop_variable) {
                return m.internal::<MacroValue>();
            }
        }
        // ② 当前宏帧局部变量
        if let Some(frame) = self.macro_frames.last() {
            if let Some(m) = frame.get_local_variable(name) {
                return m.internal::<MacroValue>();
            }
        }
        // ③ 当前命名空间（变量优先——变量可遮蔽宏）
        if let Some(m) = self.current_ns.get_variable_only(name) {
            return m.internal::<MacroValue>();
        }
        if let Some(mv) = self.current_ns.get_macro(name) {
            return Some(mv);
        }
        // ④ 全局命名空间
        if let Some(m) = self.global_ns.get_variable_only(name) {
            return m.internal::<MacroValue>();
        }
        if let Some(mv) = self.global_ns.get_macro(name) {
            return Some(mv);
        }
        // ⑤ 根数据模型（成员可为宏值）
        if let Ok(h) = self.root.get_hash() {
            if let Ok(Some(m)) = h.get(name) {
                return m.internal::<MacroValue>();
            }
        }
        // ⑥ 共享变量
        self.template
            .configuration
            .shared_vars
            .get(name)
            .and_then(|m| m.internal::<MacroValue>())
    }

    /// 设置当前命名空间变量（Java `setVariable` :2523-2528；`<#assign>`）
    pub fn set_variable(&mut self, name: &str, value: TModel) {
        self.current_ns.put_var(name.to_string(), value);
    }

    /// 设置全局命名空间变量（Java `setGlobalVariable` :2506-2511；`<#global>`）
    pub fn set_global_variable(&mut self, name: &str, value: TModel) {
        self.global_ns.put_var(name.to_string(), value);
    }

    /// 设置宏帧局部变量（Java `setLocalVariable` :2540-2556；`<#local>`）。
    /// 无宏上下文时报错（解析器已禁止 `<#local>` 出现在宏外，此处防御）。
    pub fn set_local_variable(&mut self, name: &str, value: TModel) -> Result<()> {
        let frame = self
            .macro_frames
            .last()
            .ok_or_else(|| TemplateError::misc("Not executing macro body"))?;
        frame.locals.borrow_mut().insert(name.to_string(), value);
        Ok(())
    }

    /// 读取宏帧局部变量（Java getLocalVariable :2419-2424）
    pub fn get_local_variable(&self, name: &str) -> Option<TModel> {
        for entry in self.local_stack.iter().rev() {
            if let Some(m) = entry.get(name, self.settings.fallback_on_null_loop_variable) {
                return Some(m);
            }
        }
        self.macro_frames
            .last()
            .and_then(|f| f.get_local_variable(name))
    }

    // ---------------------------------------------------------------------
    // 命名空间（Java getCurrentNamespace :2795-2807 等）
    // ---------------------------------------------------------------------

    pub fn get_current_namespace(&self) -> Rc<Namespace> {
        self.current_ns.clone()
    }
    pub fn get_main_namespace(&self) -> Rc<Namespace> {
        self.main_ns.clone()
    }
    pub fn get_global_namespace(&self) -> Rc<Namespace> {
        self.global_ns.clone()
    }

    /// 当前模板的命名空间前缀映射（`<#ftl ns_prefixes=...>`；XML 节点前缀解析——
    /// Java `currentNamespace.getTemplate().getNamespaceForPrefix`；include/import
    /// 切换时随之切换）
    pub(crate) fn current_ns_prefixes(&self) -> crate::xml::NsPrefixes {
        crate::xml::NsPrefixes::new(self.current_ns_prefixes.clone())
    }

    /// TModel → 命名空间值（内部槽位下沉，Java Namespace 对象）
    pub fn as_namespace(&self, m: &TModel) -> Option<Rc<Namespace>> {
        m.internal::<Namespace>()
    }

    /// TModel → 宏/函数值（内部槽位下沉，Java Macro 对象）
    pub fn as_macro(&self, m: &TModel) -> Option<Rc<MacroValue>> {
        m.internal::<MacroValue>()
    }

    /// TModel → 变换模型（对应 Java `instanceof TemplateTransformModel`；`<#transform>` 目标）
    pub fn as_transform(
        &self,
        m: &TModel,
    ) -> Option<Rc<dyn crate::template::TemplateTransformModel>> {
        m.transform.clone()
    }

    // ---------------------------------------------------------------------
    // 局部上下文 / 循环（Java pushLocalContext :2753-2759）
    // ---------------------------------------------------------------------

    pub(crate) fn push_local(&mut self, entry: LocalEntry) {
        self.local_stack.push(entry);
    }

    pub(crate) fn pop_local(&mut self) {
        self.local_stack.pop();
    }

    /// 查找循环上下文 —— 对应 Java `findClosestEnclosingIterationContext`（?index/?counter/
    /// ?has_next 等 BuiltInsForLoopVariables 的读数）。`target_var` 为目标表达式标识符名
    /// （`x?index` 定位名为 x 的循环层；非标识符目标取最近循环层）。
    pub fn get_loop_context(&self, target_var: Option<&str>) -> Option<Rc<RefCell<LoopCtx>>> {
        for entry in self.local_stack.iter().rev() {
            if let LocalEntry::Loop(lc) = entry {
                let c = lc.borrow();
                let matches = target_var.is_none()
                    || c.var_name == target_var.unwrap()
                    || c.var2_name.as_deref() == Some(target_var.unwrap());
                if matches {
                    drop(c);
                    return Some(lc.clone());
                }
            }
        }
        None
    }

    // ---------------------------------------------------------------------
    // 输出（Java write(Writer) :3666；capture 对应 renderElementToString :3330-3342）
    // ---------------------------------------------------------------------

    /// 输出文本（重定向期间写入捕获缓冲）
    pub fn emit(&mut self, s: &str) -> Result<()> {
        if let Some(buf) = &self.redirect {
            let mut redirected = buf.borrow_mut();
            ensure_output_limit(redirected.len(), s.len())?;
            redirected.extend_from_slice(s.as_bytes());
            return Ok(());
        }
        ensure_output_limit(self.output_buffer.len(), s.len())?;
        self.output_buffer.extend_from_slice(s.as_bytes());
        Ok(())
    }

    /// 捕获输出（`<#assign x>...</#assign>`、`<#trim>`、`<#attempt>`、函数调用丢弃输出）
    pub fn capture<R>(&mut self, f: impl FnOnce(&mut Self) -> Result<R>) -> Result<(R, String)> {
        let prev = self.redirect.take();
        let buf = Rc::new(RefCell::new(Vec::new()));
        self.redirect = Some(buf.clone());
        let r = f(self);
        self.redirect = prev;
        let text = String::from_utf8_lossy(&buf.borrow()).into_owned();
        r.map(|v| (v, text))
    }

    // ---------------------------------------------------------------------
    // 转义栈（v1 基础；P4 完整自动转义矩阵 docs/08）
    // ---------------------------------------------------------------------

    pub(crate) fn push_escape(&mut self, s: EscapeState) {
        self.escapes.push(s);
    }
    pub(crate) fn pop_escape(&mut self) {
        self.escapes.pop();
    }
    pub(crate) fn set_auto_escape(&mut self, b: bool) {
        self.auto_escape = b;
    }
    pub(crate) fn is_auto_escape(&self) -> bool {
        self.auto_escape
    }

    /// 对插值输出应用当前转义 —— 对应 Java 解析期 EscapeBlock/NoEscapeBlock 变换
    /// （FTL.jj:483-497 `escapedExpression`/`doEscape`、NoEscape :4048-4067）：
    /// 转义栈从内到外逐层应用（外层 escape 包装内层结果），`<#noescape>` 取消最内层
    /// （Java `escapes.removeFirst()` 仅弹一层）；无显式转义时按 autoesc + output_format。
    /// 占位标识符绑定为插值模型（Java 解析期以插值表达式代入，等价——见 docs/08 §5）。
    pub(crate) fn apply_escape(&mut self, m: &TModel) -> Result<String> {
        // 热路径快路径：无转义栈且未开自动转义 → 直接字符串化（跳过快照/循环开销）
        if self.escapes.is_empty() && !self.auto_escape {
            return model_to_string(self, m);
        }
        // 从栈顶（最内层）向栈底走：每个 Plain 取消一个 Custom/Html/Xml（对应
        // Java NoEscapeBlock.parse 的 removeFirst 弹栈语义）
        // 先快照栈（借用冲突：求值期需 &mut self）
        let states: Vec<EscapeState> = self.escapes.clone();
        let mut value: Option<TModel> = None;
        let mut cancelled = 0usize;
        for state in states.iter().rev() {
            match state {
                EscapeState::Plain => {
                    cancelled += 1;
                }
                EscapeState::Html | EscapeState::Xml | EscapeState::Custom(_) => {
                    if cancelled > 0 {
                        cancelled -= 1;
                        continue;
                    }
                    let cur = value.take().unwrap_or_else(|| m.clone());
                    let next = match state {
                        EscapeState::Html => {
                            let s = model_to_string(self, &cur)?;
                            TModel::from_scalar(crate::template::utility::html_escape(&s))
                        }
                        EscapeState::Xml => {
                            let s = model_to_string(self, &cur)?;
                            TModel::from_scalar(crate::template::utility::xml_escape(&s))
                        }
                        // 占位标识符绑定当前值后求值（Java 解析期占位符替换为内层变换结果，
                        // FTL.jj escapedExpression/doEscape）；占位符与真实变量同名时绑定优先，
                        // 全部绑定失败回退缺失绑定启发式（见 eval_custom_escape_bound）
                        EscapeState::Custom(expr) => self.eval_custom_escape_bound(expr, &cur)?,
                        EscapeState::Plain => unreachable!(),
                    };
                    value = Some(next);
                }
            }
        }
        // Java DollarVariable.accept（DollarVariable.java:62-99）：
        // 插值结果已是 markup 输出（?esc/?no_esc/捕获提升产物）时：
        // - 输出格式一致（moOF == outputFormat）→ 原样输出，不按 autoEsc 二次转义
        //   （Java :72-77 moOF.output(mo)）；
        // - 当前格式允许混合（UndefinedOutputFormat，:84-95）→ 原样输出；
        // - 跨格式（HTML→XML/RTF 等，:78-92）：markup 有源纯文本 → 按当前格式
        //   重转义（getSourcePlainText → markupOutputFormat.output）；无源纯文本
        //   （fromMarkup/捕获产物）→ 报错（#attempt 可捕获 → recover）。
        let final_model = value.as_ref().unwrap_or(m);
        let s = model_to_string(self, final_model)?;
        if final_model.is_markup_output() {
            let mo_fmt = final_model
                .markup_format
                .unwrap_or(self.settings.output_format);
            let cur_fmt = self.settings.output_format;
            if mo_fmt == cur_fmt
                || crate::core::built_ins_for_markup_outputs::format_mixing_allowed(cur_fmt)
            {
                return Ok(s);
            }
            match &final_model.markup_plain {
                Some(plain) => Ok(crate::core::escape_markup(cur_fmt, plain)),
                None => Err(TemplateError::misc(format!(
                    "The value to print is in {} format, which differs from the current \
                     output format, {}. Format conversion wasn't possible.",
                    mo_fmt.name(),
                    cur_fmt.name()
                ))),
            }
        } else if self.auto_escape {
            // Java AutoEscBlock：按 outputFormat 转义（v1：html/xml；其余格式 P4 TODO）
            match self.settings.output_format {
                crate::core::OutputFormatKind::Html | crate::core::OutputFormatKind::XHtml => {
                    Ok(crate::template::utility::html_escape(&s))
                }
                crate::core::OutputFormatKind::Xml => Ok(crate::template::utility::xml_escape(&s)),
                _ => Ok(s),
            }
        } else {
            Ok(s)
        }
    }

    /// escape 表达式求值（Java `EscapeBlock.doEscape`：占位标识符绑定为当前插值值；
    /// 解析器丢弃了占位符名，故以"缺失标识符 → 绑定后重试"方式近似）
    fn eval_custom_escape(&mut self, expr: &Rc<crate::core::Expr>, cur: &TModel) -> Result<TModel> {
        let placeholder_names = collect_ident_names(expr);
        match eval::eval(self, expr) {
            Ok(m) => Ok(m),
            Err(TemplateError::InvalidReference { name, .. })
                if placeholder_names.contains(&name) =>
            {
                let body = BodyCtx {
                    vars: std::iter::once((name.clone(), cur.clone())).collect(),
                };
                self.push_local(LocalEntry::Body(Rc::new(body)));
                let r = eval::eval(self, expr);
                self.pop_local();
                r
            }
            Err(e) => Err(e),
        }
    }

    /// 外层 escape：占位标识符绑定当前值后求值（Java 解析期占位符替换语义的近似——
    /// 占位符与真实变量同名时，内层结果优先，见 apply_escape 注释）。
    /// 全部绑定可能误绑真实变量（如外层 `h[x]` 的 h）→ 失败时回退缺失绑定启发式。
    fn eval_custom_escape_bound(
        &mut self,
        expr: &Rc<crate::core::Expr>,
        cur: &TModel,
    ) -> Result<TModel> {
        let names = collect_ident_names(expr);
        let body = BodyCtx {
            vars: names.iter().cloned().map(|n| (n, cur.clone())).collect(),
        };
        self.push_local(LocalEntry::Body(Rc::new(body)));
        let r = eval::eval(self, expr);
        self.pop_local();
        if r.is_ok() {
            return r;
        }
        // 回退：仅绑定缺失标识符（h[x] 等真实变量不受影响）
        self.eval_custom_escape(expr, cur)
    }
}

/// 输出转码：内部 UTF-8 → 目标编码（ISO-8859-1 / UTF-16BE 等）
/// 对应 Java `Writer` + `OutputStreamWriter` 包装：
/// OutputStreamWriter(out, Charset.forName(outputEncoding))
pub(crate) fn transcode_output(utf8: &[u8], encoding_name: &str) -> Result<Vec<u8>> {
    let s = std::str::from_utf8(utf8)
        .map_err(|_| TemplateError::misc("Internal output is not valid UTF-8"))?;
    // ISO-8859-1（Latin-1）：Unicode 码点 ≤ 0xFF 逐字节输出；超出 → '?'
    if encoding_name.eq_ignore_ascii_case("ISO-8859-1") {
        let mut out = Vec::with_capacity(s.len());
        for ch in s.chars() {
            let cu = ch as u32;
            if cu <= 0xFF {
                out.push(cu as u8);
            } else {
                out.push(b'?');
            }
        }
        return Ok(out);
    }
    // UTF-16（Java 默认 UTF-16BE + BOM；含 UTF-16BE/UTF-16LE/UTF-16 等别名）
    if encoding_name.to_uppercase().contains("UTF-16") {
        let is_le = encoding_name.to_uppercase().contains("LE");
        let with_bom = !encoding_name.to_uppercase().contains("BE")
            && !encoding_name.to_uppercase().contains("LE");
        let mut out = Vec::new();
        // BOM（Java OutputStreamWriter 对 UTF-16 默认写 BOM）
        if with_bom {
            out.extend_from_slice(&[0xFE, 0xFF]); // UTF-16BE BOM
        }
        for cu in s.encode_utf16() {
            let bytes = if is_le {
                cu.to_le_bytes()
            } else {
                cu.to_be_bytes()
            };
            out.extend_from_slice(&bytes);
        }
        return Ok(out);
    }
    // 兜底：使用 encoding_rs（支持广泛的 IANA 编码名）
    if let Some(enc) = encoding_rs::Encoding::for_label(encoding_name.as_bytes()) {
        let (encoded, _enc, _replaced) = enc.encode(s);
        // encode 返回 (Cow<[u8]>, ...)，直接取字节
        return Ok(encoded.into_owned());
    }
    Err(TemplateError::misc(format!(
        "Unknown output encoding: \"{encoding_name}\""
    )))
}
