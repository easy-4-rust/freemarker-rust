//! 渲染引擎核心 —— 对应 Java `freemarker.core.Environment`（Environment.java，3,709 行）
//!
//! 职责（docs/04 §1 对照表）：
//! - 渲染循环：`process()`（:315）/ `run()`（visit :340/:367 的栈驱动等价物）
//! - 变量解析链：`get_variable`（Java `getVariable` :2460-2487 / `_getVariable`）
//! - 命名空间：`Namespace`（:3445-3500）、import 库表（`loadedLibs`，:3283 importLib）；
//!   v1 import 立即初始化，`LazilyInitializedNamespace`（:3524-3593）语义注释见 exec.rs Import
//! - 宏调用帧：`MacroFrame`（Java `Macro.Context`，Macro.java:227；invokeMacro :848-917）
//! - 局部上下文栈：`LocalEntry`（Java `localContextStack`，:2753 pushLocalContext）
//! - 输出重定向（`<#assign x>...</#assign>` 块捕获、`<#attempt>`、`<#trim>`）
//! - 错误上下文：模板名 + 行列拼接（docs/09 §2）
//!
//! 流控设计：break/continue 以 `Err(TemplateError::Flow)` 沿 run 循环上传（由 `#list` 捕获，
//! Java `BreakOrContinueException` 是 RuntimeException 直接穿透 visit）；`<#return>` 以
//! `RunSignal::Returned` 返回（Java `ReturnInstruction.Return`）；`<#stop>` 以 `Err(Stop)` 上传
//! （Java `StopException`，attempt 可捕获）。

use crate::cache::{NameFormatDefault020300, TemplateNameFormat};
use crate::core::eval;
use crate::core::{Element, Expr, MacroDef, MacroParam, Settings, TzSetting};
use crate::error::{Result, TemplateError};
use crate::span::Span;
use crate::template::{TModel, Template, TemplateHashModel, TemplateHashModelEx};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

/// 渲染入口（对应 `Template.process(rootMap, out)` → `Environment.process()`）
pub fn render(template: &Template, root: TModel, out: &mut dyn Write) -> Result<()> {
    let mut env = Environment::new(template, root, out);
    // Java Environment.process :322 doAutoImportsAndIncludes → Configuration
    // doAutoImports（Configuration.java:3680-3685）：autoImports（ns → 模板路径）
    // 在每次渲染前 importLib
    for (ns, path) in &template.configuration.auto_imports {
        env.import_lib(path, ns)?;
    }
    env.process()
}

/// 渲染环境 —— 对应 Java `freemarker.core.Environment`
pub struct Environment<'a> {
    /// 主模板（Java `mainNamespace.getTemplate()`；Environment.java:224 getMainTemplate）
    pub template: &'a Template,
    /// 根数据模型（`Template.process` 传入的 rootMap；Java `rootDataModel`）
    pub root: TModel,
    /// 输出目标（Java `out: Writer`；:175 字段、:3666 write(Writer)）
    pub out: &'a mut dyn Write,
    /// 指令栈（Java `instructionStack` :107；docs/04 §2 渲染循环；run() 内保存/恢复）
    stack: Vec<Element>,
    /// 局部上下文栈（Java `localContextStack`；:2753 pushLocalContext / :2919 popElement）
    pub(crate) local_stack: Vec<LocalEntry>,
    /// 主命名空间（Java `mainNamespace` :181）
    main_ns: Rc<Namespace>,
    /// 当前命名空间（Java `currentNamespace` :181；include 不切换，宏体/import 执行时切换）
    pub(crate) current_ns: Rc<Namespace>,
    /// 全局命名空间（Java `globalNamespace` :181；`<#global>` 变量）
    global_ns: Rc<Namespace>,
    /// import 库表：模板路径 → 命名空间（Java `loadedLibs`；importLib :3232-3290）
    loaded_libs: HashMap<String, Rc<Namespace>>,
    /// 宏调用帧栈（栈顶 = Java `currentMacroContext` :174；`<#local>`/`<#nested>`/`<#return>` 依赖）
    pub(crate) macro_frames: Vec<Rc<MacroFrame>>,
    /// `<#return>` 发起时的宏帧深度（Java `Return.INSTANCE` 携带发起 Macro.Context 的
    /// 等价物——Macro.invoke 的 catch(Return) 按 macroCtx 归属判定捕获；穿透的
    /// return（如 `<@b><#return></@>` 中 return 归调用者宏 m 而非被调宏 b）继续上传）
    pub(crate) return_depth: Option<usize>,
    /// 运行时设置快照（Java Configurable 继承链；v1 单层，`<#setting>` 修改此副本）
    pub(crate) settings: Settings,
    /// 配置级时区（`<#setting time_zone="default">` 恢复目标；Java PropertySetting 的 null）
    pub(crate) base_time_zone: TzSetting,
    /// 配置级时区 ID（Java TimeZone.getID；`.time_zone` 读数）
    pub(crate) base_time_zone_id: String,
    /// attempt 嵌套深度（Java `inAttemptBlock` :184；attempt/recover 期间计数）
    pub(crate) attempt_depth: usize,
    /// 输出重定向（块捕获：`<#assign x>..</#assign>`、`<#trim>`、`<#attempt>`、函数调用丢弃）
    pub(crate) redirect: Option<Rc<RefCell<Vec<u8>>>>,
    /// 转义栈（v1：`<#escape>`/`<#noescape>`/autoesc 基础；自动转义完整矩阵属 P4，docs/08）
    escapes: Vec<EscapeState>,
    /// 自动转义开关（`<#autoesc>`/`<#noautoesc>` 与 settings.auto_escaping 决定）
    auto_escape: bool,
    /// 当前模板名（include/import 执行时切换；Java getCurrentTemplate :257-267；错误定位用）
    pub(crate) current_template_name: String,
    /// attempt/recover 错误栈（Java `recoveredErrorStack`，Environment.java:575-578：
    /// recover 期间压栈、结束弹出——嵌套 attempt 的内层 recover 结束后 `.error`
    /// 恢复为外层错误；BuiltinVariable.java:283-285 `.error` 读栈顶）
    pub(crate) recovered_errors: Vec<String>,
}

/// run 循环结束信号（`<#return>` 专用；Java ReturnInstruction.Return）。
/// pub：`TemplateTransformModel::transform_with_body` 的返回类型（内部信号，
/// 语义等同 Java 异常穿透；API 稳定性不承诺）
/// 豁免 large_enum_variant：信号枚举单次渲染至多出现一次，非热点分配
#[allow(clippy::large_enum_variant)]
pub enum RunSignal {
    Completed,
    Returned(Option<TModel>),
}

/// 局部上下文条目 —— 对应 Java `LocalContext` 接口 + `LocalContextStack`
#[derive(Clone)]
pub(crate) enum LocalEntry {
    /// 循环变量层（Java `IteratorBlock.IterationContext`；提供 `x`/`x_index`/`x_has_next`）
    Loop(Rc<RefCell<LoopCtx>>),
    /// `<#nested x>` 体参数层（Java `BodyInstruction.Context`，BodyInstruction.java:122-155）
    Body(Rc<BodyCtx>),
}

impl LocalEntry {
    fn get(&self, name: &str, fallback_null_loop_var: bool) -> Option<TModel> {
        match self {
            LocalEntry::Loop(lc) => lc.borrow().get(name, fallback_null_loop_var),
            LocalEntry::Body(bc) => bc.vars.get(name).cloned(),
        }
    }
}

/// 循环迭代上下文 —— 对应 Java `IteratorBlock.IterationContext`（IteratorBlock.java:190-468）
/// 提供循环变量 `x`、`x_index`、`x_has_next`，以及 `?index`/`?counter`/`?has_next` 等内建的读数
/// （docs/04 §6：循环变量作用域；fallbackOnNullLoopVariable 设置，IteratorBlock.java:368-376）。
/// 单个 #list 一个上下文（Java 模型）：`<#items>` 就地元素经 `pending` 队列驱动迭代
/// （loopForItemsElement，IteratorBlock.java:230-250）；hashListing（`as k, v`）时
/// var1=键、var2=值（getLocalVariable :452-482）。
pub struct LoopCtx {
    /// 第 1 循环变量名；`<#list>` 无 as 且未进入 `#items` 时为空串（循环变量不可见）
    pub(crate) var_name: String,
    /// 第 2 循环变量名（`as k, v` / `<#items as k, v>`）
    pub(crate) var2_name: Option<String>,
    /// 当前项（非 hash 列出）；None = null 项（fallbackOnNullLoopVariable 决定回退）
    pub(crate) value: Option<TModel>,
    /// 当前键（hashListing）
    pub(crate) key: Option<TModel>,
    /// 0 起始下标（Java `index` 字段；`x_index` 与 `?index` 读数）
    pub(crate) index: usize,
    /// 是否还有下一项（`x_has_next` 与 `<#sep>` 判定）
    pub(crate) has_next: bool,
    /// 待迭代项（Java IterationContext.openedIterator；`#items` 消费）
    pub(crate) pending: PendingItems,
    /// `#items` 是否已进入过（Java loopForItemsElement 的 alreadyEntered 校验）
    pub(crate) items_entered: bool,
}

/// 单个列表项（key=hashListing 键；value=None 表示 null 项）
#[derive(Clone)]
pub(crate) struct LoopItem {
    pub key: Option<TModel>,
    pub value: Option<TModel>,
}

/// 待迭代项（Java IterationContext.openedIterator，IteratorBlock.java:280-305）：
/// 惰性拉取——已物化项存 cache；集合角色保留底层迭代器按需取项
/// （`<#list (4..) as i>` 不会物化 2^31-1 项，Java 同样惰性驱动）；
/// `has_next` 前视：从迭代器拉一项入 cache（peek 语义，IteratorBlock.java:293-300）
pub(crate) struct PendingItems {
    cache: std::collections::VecDeque<LoopItem>,
    iter: Option<Box<dyn Iterator<Item = Result<LoopItem>>>>,
}

impl PendingItems {
    /// 已物化来源（hashListing / TemplateSequenceModel size-get 路径）
    pub(crate) fn eager(items: std::collections::VecDeque<LoopItem>) -> Self {
        PendingItems {
            cache: items,
            iter: None,
        }
    }

    /// 惰性来源（TemplateCollectionModel 迭代器）
    pub(crate) fn lazy(iter: Box<dyn Iterator<Item = Result<LoopItem>>>) -> Self {
        PendingItems {
            cache: std::collections::VecDeque::new(),
            iter: Some(iter),
        }
    }

    /// 取下一项（无 → None）；迭代器错误向上传播
    pub(crate) fn pop(&mut self) -> Result<Option<LoopItem>> {
        if let Some(item) = self.cache.pop_front() {
            return Ok(Some(item));
        }
        if let Some(it) = self.iter.as_mut() {
            return match it.next() {
                Some(Ok(item)) => Ok(Some(item)),
                Some(Err(e)) => Err(e),
                None => {
                    self.iter = None;
                    Ok(None)
                }
            };
        }
        Ok(None)
    }

    /// 是否还有下一项（前视：从迭代器拉一项进 cache）
    pub(crate) fn has_next(&mut self) -> Result<bool> {
        if !self.cache.is_empty() {
            return Ok(true);
        }
        if let Some(it) = self.iter.as_mut() {
            return match it.next() {
                Some(Ok(item)) => {
                    self.cache.push_back(item);
                    Ok(true)
                }
                Some(Err(e)) => Err(e),
                None => {
                    self.iter = None;
                    Ok(false)
                }
            };
        }
        Ok(false)
    }
}

impl LoopCtx {
    /// Java `IterationContext.getLocalVariable`（IteratorBlock.java:452-482）：
    /// 循环变量本身为 null 项时，fallbackOnNullLoopVariable=true → 返回 None（继续外层查找）；
    /// false → 返回 nothing（可见但为 null，读取报缺失）。
    fn get(&self, name: &str, fallback_null_loop_var: bool) -> Option<TModel> {
        if self.var_name.is_empty() {
            return None; // 循环变量不可见（#list 无 as 且未进入 #items）
        }
        if name == self.var_name {
            // var1：hashListing 时为键（Java loopVar1Value = kvp.getKey()）
            let v = if self.var2_name.is_some() {
                self.key.clone()
            } else {
                self.value.clone()
            };
            return match v {
                Some(m) if !m.is_nothing() => Some(m),
                _ if fallback_null_loop_var => None,
                _ => Some(TModel::nothing()),
            };
        }
        if let Some(v2) = &self.var2_name {
            if name == v2 {
                return match &self.value {
                    Some(m) if !m.is_nothing() => Some(m.clone()),
                    _ if fallback_null_loop_var => None,
                    _ => Some(TModel::nothing()),
                };
            }
        }
        if name == format!("{}_index", self.var_name) {
            return Some(TModel::from_number(crate::value::TNumber::from_i64(
                self.index as i64,
            )));
        }
        if name == format!("{}_has_next", self.var_name) {
            return Some(TModel::from_boolean(self.has_next));
        }
        None
    }
}

/// `<#nested x>` 体参数 —— 对应 Java `BodyInstruction.Context.bodyVars`
pub(crate) struct BodyCtx {
    pub(crate) vars: HashMap<String, TModel>,
}

/// 宏调用帧 —— 对应 Java `Macro.Context`（Macro.java:227-250）+ invokeMacroOrFunctionCommonPart
/// （Environment.java:848-917）。`<#local>` 写入 locals；`<#nested>` 依据 call_body/body_param/
/// caller_ns/caller_local_stack 回插调用方 body。宏定义与所属命名空间经 MacroValue 传递
/// （Java Context 中的 getMacro/getLocals 对应本帧 + macro_frames 栈）。
pub struct MacroFrame {
    /// 宏参数 + `<#local>` 变量（Java `Context.localVars`；:414 setLocalVar）
    pub(crate) locals: RefCell<HashMap<String, TModel>>,
    /// 调用方 body 元素（`<@m>body</@m>`；`<#nested>` 回插，Java callPlace.getChildBuffer()）
    pub(crate) call_body: Option<Vec<Element>>,
    /// 体参数名列表（`<@m ; a, b>`；`<#nested v1 v2>` 按位置赋给 a、b ——
    /// Java UnifiedCall.bodyParameters，BodyInstruction.Context :122-155）
    pub(crate) body_params: Vec<String>,
    /// 调用方命名空间（Java `nestedContentNamespace`；`<#nested>` 恢复用）
    pub(crate) caller_ns: Rc<Namespace>,
    /// 调用方局部上下文栈快照（Java `prevLocalContextStack`；`<#nested>` 恢复用；
    /// 调用方宏帧链由 macro_frames 栈顶自然表达，等价 Java `prevMacroContext`）
    pub(crate) caller_local_stack: Vec<LocalEntry>,
}

impl MacroFrame {
    /// 读取宏参数/局部变量（Java `Macro.Context.getLocalVariable` :403-406）
    pub(crate) fn get_local_variable(&self, name: &str) -> Option<TModel> {
        self.locals.borrow().get(name).cloned()
    }
}

/// 宏/函数值 —— 对应 Java `freemarker.core.Macro` 对象（作为 TemplateModel 值出现；
/// 经 TModel.internal 槽位承载，`?is_macro` 依据 kind 判定）
pub struct MacroValue {
    pub def: Rc<MacroDef>,
    /// 宏所属命名空间（Java `macroToNamespaceLookup` :185；宏体内 currentNamespace 切换）
    pub ns: Rc<Namespace>,
}

/// lambda 值 —— 对应 Java `LocalLambdaExpression` 求值结果（v1 仅存槽位；
/// `?map`/`?filter` 等消费方由内建函数智能体扩展，docs/04 §5）
pub struct LambdaValue {
    /// 参数名列表（Java LambdaParameterList；多参数 lambda 自 2.3.32 起）
    #[allow(dead_code)]
    pub params: Vec<String>,
    #[allow(dead_code)]
    pub body: Rc<Expr>,
}

/// 命名空间 —— 对应 Java `Environment.Namespace`（Environment.java:3445-3500，extends SimpleHash）
/// 变量与宏同表（Java Namespace 是 SimpleHash，宏以 `Macro` 对象存入）；
/// 同时实现 TemplateHashModel/Ex → 可作 TModel 值（`<@ns.macro>`、`ns.var`、`?keys` 等）。
pub struct Namespace {
    vars: RefCell<HashMap<String, TModel>>,
    macros: RefCell<HashMap<String, Rc<MacroValue>>>,
    /// 关联模板名（Java `Namespace.getTemplate()` :3470-3478；错误定位/相对 include 基名）
    template_name: String,
}

impl Namespace {
    fn new(template_name: String) -> Self {
        Namespace {
            vars: RefCell::new(HashMap::new()),
            macros: RefCell::new(HashMap::new()),
            template_name,
        }
    }

    /// 成员读取：先变量后宏（Java SimpleHash.get——含 Macro 对象）
    pub(crate) fn get_member(&self, name: &str) -> Option<TModel> {
        if let Some(m) = self.vars.borrow().get(name) {
            return Some(m.clone());
        }
        self.macros
            .borrow()
            .get(name)
            .map(|mv| macro_model(mv.clone()))
    }

    pub(crate) fn put_var(&self, name: String, m: TModel) {
        self.vars.borrow_mut().insert(name, m);
    }

    pub(crate) fn put_macro(&self, name: String, m: Rc<MacroValue>) {
        self.macros.borrow_mut().insert(name, m);
    }

    /// 模板名（Java Namespace.getTemplate().getName()）
    pub fn template_name(&self) -> &str {
        &self.template_name
    }

    /// 变量表只读视图（调试/测试辅助）
    pub fn var_names(&self) -> Vec<String> {
        self.vars.borrow().keys().cloned().collect()
    }
}

impl TemplateHashModel for Namespace {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        Ok(self.get_member(key))
    }
    fn is_empty(&self) -> Result<bool> {
        Ok(self.vars.borrow().is_empty() && self.macros.borrow().is_empty())
    }
}

impl TemplateHashModelEx for Namespace {
    fn size(&self) -> Result<usize> {
        // Java SimpleHash.size 含宏；v1 仅统计变量（文档化差异）
        Ok(self.vars.borrow().len())
    }
    fn keys(&self) -> Result<Vec<String>> {
        // Java 为 LinkedHashMap 插入序；Rust HashMap 无序遍历 → 排序保证确定性
        let mut k: Vec<String> = self.vars.borrow().keys().cloned().collect();
        k.sort();
        Ok(k)
    }
}

/// 转义状态（v1 基础；自动转义完整矩阵属 P4，docs/08）
#[derive(Clone)]
pub(crate) enum EscapeState {
    /// 无转义（默认 / `<#noescape>`）
    Plain,
    /// `<#escape x as html>`
    Html,
    /// `<#escape x as xml>`
    Xml,
    /// `<#escape x as 其他表达式>`：每次插值求值该表达式并按方法调用（Java EscapeBlock 逐插值包装）
    Custom(Rc<Expr>),
}

impl<'a> Environment<'a> {
    /// 构造环境 —— 对应 Java `Environment(Template, TemplateHashModel, Writer)`（:201-217）：
    /// 构造时 `importMacros(template)` 预先注册主模板宏（Java 宏定义在渲染前全局可见）。
    pub fn new(template: &'a Template, root: TModel, out: &'a mut dyn Write) -> Self {
        let settings = template.configuration.settings.clone();
        let base_time_zone = settings.time_zone;
        let base_time_zone_id = settings.time_zone_id.clone();
        let main_ns = Rc::new(Namespace::new(template.name.clone()));
        for (name, def) in &template.macros {
            register_macro(&main_ns, name, def);
        }
        let current_ns = main_ns.clone();
        let global_ns = Rc::new(Namespace::new(template.name.clone()));
        // Java autoEscaping 默认随 outputFormat 与 incompatibleImprovements（docs/08 §1）
        let auto_escape = match settings.auto_escaping {
            crate::core::AutoEscaping::On => true,
            crate::core::AutoEscaping::Off => false,
            crate::core::AutoEscaping::Default => settings.output_format.is_markup(),
        };
        Environment {
            template,
            root,
            out,
            stack: Vec::new(),
            local_stack: Vec::new(),
            main_ns,
            current_ns,
            global_ns,
            loaded_libs: HashMap::new(),
            macro_frames: Vec::new(),
            return_depth: None,
            settings,
            base_time_zone,
            base_time_zone_id,
            attempt_depth: 0,
            redirect: None,
            escapes: Vec::new(),
            auto_escape,
            current_template_name: template.name.clone(),
            recovered_errors: Vec::new(),
        }
    }

    /// 渲染入口 —— 对应 Java `Environment.process()`（:315-336）：
    /// 执行根元素，结束后自动 flush（Java :323-325 `if (getAutoFlush()) out.flush()`）。
    pub fn process(&mut self) -> Result<()> {
        if !self.root.is_hash() {
            return Err(TemplateError::misc("The data model must be a hash"));
        }
        let root = self.template.root.clone();
        match self.run(&root)? {
            RunSignal::Completed => self.out.flush().map_err(TemplateError::Io),
            RunSignal::Returned(_) => Err(TemplateError::misc(
                "<#return> is illegal here (not inside a macro or function)",
            )),
        }
    }

    /// 执行一组元素 —— Java `visit(TemplateElement[])`（:367-405）的栈驱动等价物。
    /// - `Next(children)`：子元素入栈（逆序保证执行顺序）；`Replace`：入栈替换；
    /// - `ReturnValue` → RunSignal::Returned（Java ReturnInstruction.Return 异常语义）；
    /// - `Flow`/`Stop` → Err 上传（Java RuntimeException / StopException 穿透）；
    /// - 其他错误：附加源码位置（模板名 + 行列，docs/09 §2）后上传。
    ///   嵌套调用（宏体/指令 body/捕获块）会保存并恢复外层待执行栈。
    pub(crate) fn run(&mut self, els: &[Element]) -> Result<RunSignal> {
        let saved = std::mem::take(&mut self.stack);
        // 逆序入栈：栈顶先出（与 run_loop 中 Next(children) 的逆序入栈一致）
        self.stack.extend(els.iter().rev().cloned());
        let r = self.run_loop();
        self.stack = saved;
        r
    }

    fn run_loop(&mut self) -> Result<RunSignal> {
        while let Some(el) = self.stack.pop() {
            let span = el.span;
            match crate::core::exec::exec(self, &el) {
                Ok(crate::core::exec::ExecOutcome::Next(children)) => {
                    for c in children.into_iter().rev() {
                        self.stack.push(c);
                    }
                }
                Ok(crate::core::exec::ExecOutcome::Replace(e)) => self.stack.push(e),
                Ok(crate::core::exec::ExecOutcome::Done) => {}
                Ok(crate::core::exec::ExecOutcome::ReturnValue(v)) => {
                    return Ok(RunSignal::Returned(v));
                }
                Ok(crate::core::exec::ExecOutcome::Flow(k)) => {
                    return Err(TemplateError::Flow(k));
                }
                Ok(crate::core::exec::ExecOutcome::Stop(m)) => {
                    return Err(TemplateError::Stop { message: m });
                }
                Err(e) => {
                    return Err(attach_location(e, &self.current_template_name, span));
                }
            }
        }
        Ok(RunSignal::Completed)
    }

    /// 执行元素序列到完成（自定义指令 body / include / 测试入口；
    /// Java `NestedElementTemplateDirectiveBody.render` :3445-3475 的语义等价物）
    pub fn run_elements(&mut self, els: &[Element]) -> Result<()> {
        match self.run(els)? {
            RunSignal::Completed => Ok(()),
            RunSignal::Returned(_) => {
                Err(TemplateError::misc("<#return> is illegal in this context"))
            }
        }
    }

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
            buf.borrow_mut().extend_from_slice(s.as_bytes());
            return Ok(());
        }
        self.out.write_all(s.as_bytes()).map_err(TemplateError::Io)
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
                            TModel::from_scalar(crate::utility::html_escape(&s))
                        }
                        EscapeState::Xml => {
                            let s = model_to_string(self, &cur)?;
                            TModel::from_scalar(crate::utility::xml_escape(&s))
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
        let s = match value {
            Some(v) => model_to_string(self, &v)?,
            None => model_to_string(self, m)?,
        };
        if self.auto_escape {
            // Java AutoEscBlock：按 outputFormat 转义（v1：html/xml；其余格式 P4 TODO）
            match self.settings.output_format {
                crate::core::OutputFormatKind::Html | crate::core::OutputFormatKind::XHtml => {
                    Ok(crate::utility::html_escape(&s))
                }
                crate::core::OutputFormatKind::Xml => Ok(crate::utility::xml_escape(&s)),
                _ => Ok(s),
            }
        } else {
            Ok(s)
        }
    }

    /// escape 表达式求值（Java `EscapeBlock.doEscape`：占位标识符绑定为当前插值值；
    /// 解析器丢弃了占位符名，故以"缺失标识符 → 绑定后重试"方式近似）
    fn eval_custom_escape(&mut self, expr: &Rc<Expr>, cur: &TModel) -> Result<TModel> {
        let placeholder_names = collect_ident_names(expr);
        match eval::eval(self, expr) {
            Ok(m) => Ok(m),
            Err(TemplateError::InvalidReference { name }) if placeholder_names.contains(&name) => {
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
    fn eval_custom_escape_bound(&mut self, expr: &Rc<Expr>, cur: &TModel) -> Result<TModel> {
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

    // ---------------------------------------------------------------------
    // 宏/函数调用（Java invokeMacro :819-829 / invokeMacroOrFunctionCommonPart :848-917）
    // ---------------------------------------------------------------------

    /// 宏调用（`<@m ...>`；输出到当前 out）。返回 RunSignal（`<#return>` 经 Returned 上传）。
    pub(crate) fn invoke_macro(
        &mut self,
        mv: &MacroValue,
        args: &[(String, crate::core::Expr)],
        body: Option<Vec<Element>>,
        body_params: Vec<String>,
    ) -> Result<RunSignal> {
        self.invoke_macro_common(mv, args, body, body_params, false)
    }

    /// 函数调用（Java invokeFunction :832-847：输出丢弃到 NullWriter；无 `<#return>` → nothing）
    pub(crate) fn invoke_function(
        &mut self,
        mv: &MacroValue,
        args: &[(String, crate::core::Expr)],
    ) -> Result<TModel> {
        let r = self.invoke_macro_common(mv, args, None, Vec::new(), true)?;
        match r {
            RunSignal::Returned(v) => Ok(v.unwrap_or_else(TModel::nothing)),
            RunSignal::Completed => Ok(TModel::nothing()),
        }
    }

    fn invoke_macro_common(
        &mut self,
        mv: &MacroValue,
        args: &[(String, crate::core::Expr)],
        body: Option<Vec<Element>>,
        body_params: Vec<String>,
        is_function: bool,
    ) -> Result<RunSignal> {
        // Java :848-879：宏帧 + 参数绑定（求值发生在调用方上下文）
        let frame = Rc::new(MacroFrame {
            locals: RefCell::new(HashMap::new()),
            call_body: body,
            body_params,
            caller_ns: self.current_ns.clone(),
            caller_local_stack: self.local_stack.clone(),
        });
        bind_macro_args(self, &frame, &mv.def, args)?;
        // Java :880-894：压帧、切换命名空间、清空局部上下文
        self.macro_frames.push(frame.clone());
        let prev_ns = self.current_ns.clone();
        self.current_ns = mv.ns.clone();
        let prev_local = std::mem::take(&mut self.local_stack);
        // Java :893 checkParamsSetAndApplyDefaults（宏上下文内求值默认参数）
        apply_macro_defaults(self, &frame, &mv.def)?;
        let r = if is_function {
            let sig = self
                .capture(|env| env.run(&mv.def.body))
                .map(|(sig, _)| sig)?;
            // Java Macro.invoke 的 catch(Return) 归属判定：return 由本函数帧发起
            // （深度匹配）才作为返回值捕获；穿透的 return（更外层宏）继续上传
            if let RunSignal::Returned(_) = &sig {
                if self.return_depth == Some(self.macro_frames.len()) {
                    self.return_depth = None;
                } else {
                    return self.restore_after_macro(prev_ns, prev_local, sig);
                }
            }
            sig
        } else {
            let sig = self.run(&mv.def.body)?;
            // Java Macro.invoke：宏边界捕获归属本帧的 return（宏不能 return 值 →
            // 值恒 None，捕获即宏正常完成）；穿透的 return 继续上传
            if let RunSignal::Returned(_) = &sig {
                if self.return_depth == Some(self.macro_frames.len()) {
                    self.return_depth = None;
                    RunSignal::Completed
                } else {
                    return self.restore_after_macro(prev_ns, prev_local, sig);
                }
            } else {
                sig
            }
        };
        // Java finally :895-901：恢复
        self.current_ns = prev_ns;
        self.local_stack = prev_local;
        self.macro_frames.pop();
        Ok(r)
    }

    /// 宏调用结束恢复（Java finally :895-901；穿透的 return 上传前同样恢复现场）
    fn restore_after_macro(
        &mut self,
        prev_ns: Rc<Namespace>,
        prev_local: Vec<LocalEntry>,
        r: RunSignal,
    ) -> Result<RunSignal> {
        self.current_ns = prev_ns;
        self.local_stack = prev_local;
        self.macro_frames.pop();
        Ok(r)
    }

    pub(crate) fn get_current_macro_frame(&self) -> Option<Rc<MacroFrame>> {
        self.macro_frames.last().cloned()
    }

    // ---------------------------------------------------------------------
    // include / import（Java include :3126-3145 / importLib :3232-3290）
    // ---------------------------------------------------------------------

    /// 模板名相对路径解析 —— 对应 Java `toFullTemplateName`（:3314-3349）：
    /// 绝对路径（`/` 开头或含 `://`）原样返回；相对路径基于当前模板所在目录，
    /// 并规范化 `../`/`./` 段。
    pub fn resolve_template_name(&self, target: &str) -> String {
        if target.starts_with('/') || target.contains("://") {
            return target.to_string();
        }
        let base = &self.current_template_name;
        let joined = match base.rfind('/') {
            Some(i) => {
                let dir = &base[..i];
                if dir.is_empty() {
                    format!("/{target}")
                } else {
                    format!("{dir}/{target}")
                }
            }
            None => target.to_string(),
        };
        normalize_template_path(&joined)
    }

    /// `<#include>`（Java Include.accept → getTemplateForInclusion :3095-3110 → include :3126-3145；
    /// 路径含 `*` → acquisition：localized 外层 + acquisition 内层，TemplateCache.java:914-948；
    /// parse=false → 源文本原样输出（getPlainTextTemplate，单 TextBlock）；
    /// ignore_missing=true → 模板缺失时静默跳过（Configuration.getTemplate 的 ignoreMissing）；
    /// encoding=None → 继承当前模板的 `<#ftl encoding>` 声明，否则默认 UTF-8
    /// （Java Environment.getIncludedTemplateEncoding :3099-3105））
    pub fn include_named(
        &mut self,
        name: &str,
        parse: bool,
        ignore_missing: bool,
        encoding: Option<String>,
    ) -> Result<()> {
        let full = self.resolve_template_name(name);
        let encoding = encoding.or_else(|| self.template.encoding.clone());
        let mut found: Option<(String, Rc<crate::template::Template>)> = None;
        let mut last_err: Option<TemplateError> = None;
        // Java lookupWithLocalizedThenAcquisitionStrategy（TemplateCache.java:914-948）：
        // 每个 locale 变体（en_US → en → 无后缀）内部做完整 acquisition
        let locale = self.settings.locale.clone();
        let locale_cands: Vec<String> = if locale.is_empty() {
            vec![full.clone()]
        } else {
            crate::template::configuration::localized_candidates(&full, &locale)
        };
        'outer: for lc in &locale_cands {
            for acq in acquisition_candidates(lc) {
                if !parse {
                    // Java parseAsFTL=false：直接读源文本（TemplateCache.loadTemplate
                    // :564-580 的 StringWriter 分支；不解析、不触发 ftl 头编码重读）
                    let Some(src) = self.template.configuration.template_loader.find(&acq)? else {
                        continue;
                    };
                    let text = self
                        .template
                        .configuration
                        .template_loader
                        .read_encoded(&*src, encoding.as_deref().unwrap_or("UTF-8"))?;
                    return self.emit(&text);
                }
                match self
                    .template
                    .configuration
                    .get_template_encoded(&acq, encoding.as_deref())
                {
                    Ok(t) => {
                        found = Some((acq, t));
                        break 'outer;
                    }
                    Err(e) => last_err = Some(e),
                }
            }
        }
        let Some((_, t)) = found else {
            if ignore_missing {
                return Ok(());
            }
            return Err(last_err.unwrap_or(TemplateError::NotFound { name: full }));
        };
        self.include_template(&t)
    }

    /// 执行被包含模板（Java include(includedTemplate) :3126-3145：
    /// 先 importMacros 把宏注册进当前命名空间，再执行根元素；不切换命名空间）
    pub fn include_template(&mut self, included: &Template) -> Result<()> {
        let cur_ns = self.current_ns.clone();
        for (name, def) in &included.macros {
            register_macro(&cur_ns, name, def);
        }
        let prev_name = self.current_template_name.clone();
        self.current_template_name = included.name.clone();
        let r = self.run(&included.root);
        self.current_template_name = prev_name;
        match r {
            Ok(RunSignal::Completed) => Ok(()),
            Ok(RunSignal::Returned(_)) => Err(TemplateError::misc(
                "<#return> is illegal in an included template",
            )),
            Err(e) => Err(e),
        }
    }

    /// `<#import path as ns>`（Java LibraryLoad.accept → importLib :3232-3290）。
    /// v1 立即初始化命名空间（懒初始化 LazilyInitializedNamespace :3524-3593 语义：
    /// 首次访问才 get_template 并注册宏——P4 优化项，v1 直接执行等价结果）。
    pub fn import_lib(&mut self, path: &str, ns_var: &str) -> Result<()> {
        // Java importLib（:3232-3290）：toFullTemplateName 后按模板名格式规范化
        // （"/import_lib.ftl" 与 "import_lib.ftl" 是同一模板——loadedLibs 缓存键一致）
        let resolved = self.resolve_template_name(path);
        let full = NameFormatDefault020300
            .normalize_root_based_name(&resolved)
            .unwrap_or(resolved);
        let ns = if let Some(existing) = self.loaded_libs.get(&full) {
            existing.clone()
        } else {
            let locale = self.settings.locale.clone();
            let t = self
                .template
                .configuration
                .get_template_localized(&full, Some(&locale))?;
            let ns = Rc::new(Namespace::new(t.name.clone()));
            self.loaded_libs.insert(full, ns.clone());
            // Java initializeImportLibNamespace :3290-3303：currentNamespace 切换 + 输出丢弃执行
            self.initialize_import_lib_namespace(&ns, &t)?;
            ns
        };
        // Java :3255-3264：setVariable(nsVar, namespace)；currentNamespace==mainNamespace 时
        // 同步到 globalNamespace（IcI 2.3.24+）
        self.set_variable(ns_var, namespace_model(ns.clone()));
        if Rc::ptr_eq(&self.current_ns, &self.main_ns) {
            self.global_ns
                .put_var(ns_var.to_string(), namespace_model(ns));
        }
        Ok(())
    }

    fn initialize_import_lib_namespace(&mut self, ns: &Rc<Namespace>, t: &Template) -> Result<()> {
        let prev_ns = self.current_ns.clone();
        let prev_name = self.current_template_name.clone();
        self.current_ns = ns.clone();
        self.current_template_name = t.name.clone();
        for (name, def) in &t.macros {
            register_macro(ns, name, def);
        }
        let r = self.capture(|env| env.run(&t.root));
        self.current_ns = prev_ns;
        self.current_template_name = prev_name;
        match r {
            Ok((RunSignal::Completed, _)) => Ok(()),
            Ok((RunSignal::Returned(_), _)) => Err(TemplateError::misc(
                "<#return> is illegal in an imported template",
            )),
            Err(e) => Err(e),
        }
    }

    /// 注册宏定义到命名空间（Java Environment.visitMacroDef :1164-1167）
    pub fn register_macro_def(&mut self, def: &MacroDef) {
        let ns = self.current_ns.clone();
        register_macro(&ns, &def.name, def);
    }
}

/// 注册宏（Java visitMacroDef :1164-1167：currentNamespace.put(macroName, macro)）
pub(crate) fn register_macro(ns: &Rc<Namespace>, name: &str, def: &MacroDef) {
    ns.put_macro(
        name.to_string(),
        Rc::new(MacroValue {
            def: Rc::new(def.clone()),
            ns: ns.clone(),
        }),
    );
}

/// 宏参数绑定 —— 对应 Java `setMacroContextLocalsFromArguments`（Environment.java:919-1094，
/// v1 无 `?with_args`）。位置参数按声明顺序（catch-all 不计入普通参数槽，Macro.java:74-81）；
/// 命名参数匹配普通参数，未声明者进入命名 catch-all 哈希（Java :1017-1039）。
fn bind_macro_args(
    env: &mut Environment,
    frame: &Rc<MacroFrame>,
    def: &MacroDef,
    args: &[(String, crate::core::Expr)],
) -> Result<()> {
    let normal_params: Vec<&MacroParam> = def.params.iter().filter(|p| !p.catch_all).collect();
    let catch_all_name = def
        .params
        .iter()
        .find(|p| p.catch_all)
        .map(|p| p.name.clone());
    let mut next_pos = 0usize;
    // Java 命名 catch-all → SimpleHash(LinkedHashMap)：参数插入序
    let mut named_catch_all: Option<IndexMap<String, TModel>> = None;
    let mut positional_catch_all: Option<Vec<TModel>> = None;

    for (arg_name, arg_expr) in args {
        let value = eval::eval(env, arg_expr)?;
        // Java Macro.Context.checkParamsSetAndApplyDefaults（Macro.java:273-322）：
        // 参数值为 null 时——有默认值 → 求默认值；无默认值且 classic 兼容 → 参数
        // 保持未设置（变量查找回退外层作用域）；strict → "required parameter ...
        // was specified, but had null/missing value."。本引擎 classic 模式跳过绑定
        // （回退外层），strict 保持绑定 nothing（既有偏差，见 docs）
        if value.is_nothing() && env.settings.classic_compatible {
            if arg_name.is_empty() {
                next_pos += 1; // 位置槽仍被消耗（Java localVars 含 null 条目，get 回 null）
            }
            continue;
        }
        if arg_name.is_empty() {
            // 位置参数（Java :1041-1080）
            if next_pos < normal_params.len() {
                let name = normal_params[next_pos].name.clone();
                next_pos += 1;
                frame.locals.borrow_mut().insert(name, value);
            } else if let Some(cn) = &catch_all_name {
                let list = positional_catch_all.get_or_insert_with(Vec::new);
                list.push(value);
                let seq = TModel::from_sequence(list.clone());
                frame.locals.borrow_mut().insert(cn.clone(), seq);
            } else {
                // Java newTooManyArgumentsException（Environment.java:1097-1103）
                return Err(TemplateError::misc(format!(
                    "{} {} only accepts {} parameters, but got {}.",
                    if def.is_function { "Function" } else { "Macro" },
                    quote_name(&def.name),
                    normal_params.len(),
                    args.len(),
                )));
            }
        } else if normal_params.iter().any(|p| p.name == *arg_name) {
            frame.locals.borrow_mut().insert(arg_name.clone(), value);
        } else if let Some(cn) = &catch_all_name {
            // 命名 catch-all（Java :1019-1036）
            let hash = named_catch_all.get_or_insert_with(IndexMap::new);
            hash.insert(arg_name.clone(), value);
            frame
                .locals
                .borrow_mut()
                .insert(cn.clone(), TModel::from_hash(hash.clone()));
        } else {
            // Java newUndeclaredParamNameException（Environment.java:1105-1113）
            return Err(TemplateError::misc(format!(
                "{} {} has no parameter with name {}",
                if def.is_function { "Function" } else { "Macro" },
                quote_name(&def.name),
                quote_name(arg_name),
            )));
        }
    }
    // Java Environment.java:1007-1013：catch-all 未收到任何额外参数时也必须绑定
    // ——by-position 调用（存在位置参数）→ 空序列；by-name 调用 → 空哈希
    // （如 `<@m foo=1/>` 后 `bar` 为 size 0 的哈希，宏体内可直接 ?keys）
    if let Some(cn) = &catch_all_name {
        let bound = frame.locals.borrow().contains_key(cn);
        if !bound {
            let by_position = args.iter().any(|(n, _)| n.is_empty());
            let value = if by_position {
                TModel::from_sequence(Vec::new())
            } else {
                TModel::from_hash(indexmap::IndexMap::new())
            };
            frame.locals.borrow_mut().insert(cn.clone(), value);
        }
    }
    Ok(())
}

/// 默认参数求值 —— 对应 Java `Macro.Context.checkParamsSetAndApplyDefaults`（Macro.java:255-340）。
/// 默认值在宏上下文内求值；循环重试直到无进展（默认值可相互引用）；仍未设置且带默认值 →
/// 抛默认值的 InvalidReference；不带默认值 → "required parameter not specified"。
fn apply_macro_defaults(
    env: &mut Environment,
    frame: &Rc<MacroFrame>,
    def: &MacroDef,
) -> Result<()> {
    loop {
        let mut resolved = false;
        for param in &def.params {
            if param.catch_all {
                continue;
            }
            let cur = frame.locals.borrow().get(&param.name).cloned();
            let set = matches!(&cur, Some(m) if !m.is_nothing());
            if set {
                continue;
            }
            if let Some(def_expr) = &param.default {
                match eval::eval(env, def_expr) {
                    Ok(v) if !v.is_nothing() => {
                        frame.locals.borrow_mut().insert(param.name.clone(), v);
                        resolved = true;
                    }
                    Ok(_) => {} // 默认值本身为 null：继续重试（Java hasUnresolvedDefaultValue）
                    Err(TemplateError::InvalidReference { .. }) => {} // 未决：继续重试
                    Err(e) => return Err(e),
                }
            }
        }
        if !resolved {
            break;
        }
    }
    // 收尾：仍未设置的参数
    for (idx, param) in def.params.iter().enumerate() {
        if param.catch_all {
            continue;
        }
        let cur = frame.locals.borrow().get(&param.name).cloned();
        let set = matches!(&cur, Some(m) if !m.is_nothing());
        if !set {
            if env.settings.classic_compatible {
                // Java Macro.java :301-322/:328-333：classic 兼容模式参数保持未设置
                // （变量查找回退外层作用域），不报错
                continue;
            }
            if param.default.is_some() {
                return Err(TemplateError::invalid_reference(format!(
                    "Default value of parameter \"{}\" (parameter #{}) of {} {} could not be resolved",
                    param.name,
                    idx + 1,
                    if def.is_function { "function" } else { "macro" },
                    quote_name(&def.name),
                )));
            }
            // Java :301-322：When calling macro "m", required parameter "x" (parameter #N) was not specified.
            return Err(TemplateError::misc(format!(
                "When calling {} {}, required parameter {} (parameter #{}) was not specified.",
                if def.is_function { "function" } else { "macro" },
                quote_name(&def.name),
                quote_name(&param.name),
                idx + 1,
            )));
        }
    }
    Ok(())
}

/// Java `_CoreStringUtils.jQuote` 的简化形式（错误消息用）
fn quote_name(s: &str) -> String {
    format!("\"{}\"", s)
}

// ---------------------------------------------------------------------------
// 模型值构造与下沉（内部槽位）
// ---------------------------------------------------------------------------

/// 命名空间值模型（Java Namespace 作为 TemplateHashModel 值；FTL 类型 extended_hash）
pub fn namespace_model(ns: Rc<Namespace>) -> TModel {
    let h: Rc<dyn TemplateHashModel> = ns.clone();
    let ex: Rc<dyn TemplateHashModelEx> = ns.clone();
    TModel {
        hash: Some(h),
        hash_ex: Some(ex),
        internal: Some(ns),
        type_name: "extended_hash",
        kind: crate::template::ModelKind::Hash,
        ..TModel::nothing()
    }
}

/// `.vars` 快照 —— 对应 Java `BuiltinVariable.VarsHash`（BuiltinVariable.java:330-337：
/// get(key) = env.getVariable 完整解析链）。v1 以快照近似活视图：按
/// 局部上下文（循环变量）> 宏帧局部变量 > 当前命名空间 > 全局命名空间 > 根模型 >
/// 共享变量的优先级合并（低优先级先入、高优先级覆盖）。
pub fn vars_snapshot(env: &Environment) -> IndexMap<String, TModel> {
    let mut map = IndexMap::new();
    // 根数据模型
    if let Some(ex) = &env.root.hash_ex {
        let keys = ex.keys().unwrap_or_default();
        for k in keys {
            if let Ok(Some(v)) = ex.get(&k) {
                map.insert(k, v);
            }
        }
    }
    // 共享变量
    for (k, v) in &env.template.configuration.shared_vars {
        map.insert(k.clone(), v.clone());
    }
    // 全局命名空间
    for k in env.get_global_namespace().var_names() {
        if let Some(v) = env.get_global_namespace().get_member(&k) {
            map.insert(k, v);
        }
    }
    // 当前命名空间
    for k in env.get_current_namespace().var_names() {
        if let Some(v) = env.get_current_namespace().get_member(&k) {
            map.insert(k, v);
        }
    }
    // 宏帧局部变量
    if let Some(frame) = env.macro_frames.last() {
        for (k, v) in frame.locals.borrow().iter() {
            map.insert(k.clone(), v.clone());
        }
    }
    // 局部上下文（循环变量/body 参数）
    for entry in env.local_stack.iter().rev() {
        if let LocalEntry::Loop(lc) = entry {
            let c = lc.borrow();
            if !c.var_name.is_empty() {
                if let Some(v) = c.get(&c.var_name, env.settings.fallback_on_null_loop_variable) {
                    map.insert(c.var_name.clone(), v);
                }
                if let Some(v2) = &c.var2_name {
                    if let Some(v) = c.get(v2, env.settings.fallback_on_null_loop_variable) {
                        map.insert(v2.clone(), v);
                    }
                }
            }
        } else if let LocalEntry::Body(bc) = entry {
            for (k, v) in bc.vars.iter() {
                map.insert(k.clone(), v.clone());
            }
        }
    }
    map
}

/// 宏/函数值模型（Java Macro 对象；`?is_macro` 依据 kind 判定）
pub fn macro_model(mv: Rc<MacroValue>) -> TModel {
    TModel {
        internal: Some(mv),
        type_name: "macro",
        kind: crate::template::ModelKind::Macro,
        ..TModel::nothing()
    }
}

/// lambda 值模型（Java LocalLambdaExpression 求值结果；v1 仅存槽位）
pub fn lambda_model(params: Vec<String>, body: Rc<Expr>) -> TModel {
    TModel {
        internal: Some(Rc::new(LambdaValue { params, body })),
        type_name: "lambda",
        kind: crate::template::ModelKind::Lambda,
        ..TModel::nothing()
    }
}

// ---------------------------------------------------------------------------
// 输出字符串化与错误上下文（docs/09 §2）
// ---------------------------------------------------------------------------

/// 布尔格式的 true/false 字符串（Java `getTrueStringValue`/`getFalseStringValue`；
/// "true,false" 遗留默认 → None——视为未设置）
pub(crate) fn boolean_format_strings(env: &Environment) -> Option<(String, String)> {
    let format = env.settings.boolean_format.as_str();
    if format == "true,false" {
        // Java parseBooleanFormat（Configurable.java:1087-1090）：BOOLEAN_FORMAT_LEGACY_DEFAULT
        // → null（视为未设置，即使显式设置）→ getTrueStringValue null → 报错
        None
    } else if format == "c" {
        Some(("true".to_string(), "false".to_string()))
    } else {
        format
            .split_once(',')
            .map(|(t, f)| (t.to_string(), f.to_string()))
    }
}

/// 布尔格式化 —— 对应 Java `Environment.formatBoolean`（Environment.java:1795）：
/// - "true,false"（BOOLEAN_FORMAT_LEGACY_DEFAULT）→ parseBooleanFormat 返回 null
///   （Configurable.java:1087-1090，显式设置亦然）→ 视为未设置：
///   fallback=false（插值/字符串拼接路径）报 legacy 错误（jar 实测 `${false}` 默认配置报错）；
///   fallback=true（?string 路径）返回 true/false；
/// - "c" → C 格式 true/false（Java parseBooleanFormat 空数组 → CFormat.getTrue/FalseString）；
/// - 其余 → 按首个逗号切分（Java indexOf(',')；不做 trim）。
pub(crate) fn boolean_format(env: &Environment, b: bool, fallback: bool) -> Result<String> {
    let format = env.settings.boolean_format.as_str();
    if format == "true,false" {
        if fallback {
            return Ok(if b {
                "true".to_string()
            } else {
                "false".to_string()
            });
        }
        return Err(TemplateError::misc(
            "Can't convert boolean to string automatically, because the \"boolean_format\" setting was \"true,false\", which is the legacy deprecated default, and we treat it as if no format was set. This is the default configuration; you should provide the format explicitly for each place where you print a boolean.",
        ));
    }
    if format == "c" {
        return Ok(if b {
            "true".to_string()
        } else {
            "false".to_string()
        });
    }
    match format.split_once(',') {
        Some((t, f)) => Ok(if b { t.to_string() } else { f.to_string() }),
        None => Err(TemplateError::misc(format!(
            "Setting value must be a string that contains two comma-separated values for true and false, or it must be \"c\", but it was {format:?}."
        ))),
    }
}

/// 模型 → 输出字符串 —— 对应 Java 插值/字符串拼接的模型转字符串规则
/// （Java EvalUtil.coerceModelToStringOrMarkup）：
/// - 标量原样；数字按 number_format（"number" → canonical plain、"c" → C 格式、
///   其余 → DecimalFormat 子集，见 builtins/format.rs）；
/// - 布尔：classic-compatible 模式（Java EvalUtil.coerceModelToTextualCommon :486-518：
///   true → "true"、false → ""；双角色标量模型优先返回字符串），否则 boolean_format；
/// - 日期按 date_format/time_format/date_time_format 设置（Java formatDateToPlainText）。
pub(crate) fn model_to_string(env: &mut Environment, m: &TModel) -> Result<String> {
    if m.is_nothing() {
        // Java EvalUtil.coerceModelToTextualCommon :482-494：tm == null → classic 兼容
        // 模式回退空串；strict 模式为 InvalidReferenceException（本引擎 strict 的
        // 缺失变量在解析层已抛 Err，此处仅显式 null 值会到达）
        if env.settings.classic_compatible {
            return Ok(String::new());
        }
    }
    if let Some(s) = &m.scalar {
        return s.as_string();
    }
    if let Some(n) = &m.number {
        return n
            .as_number()
            .map(|n| crate::builtins::format::format_number(env, &n));
    }
    if let Some(b) = &m.boolean {
        let bv = b.as_boolean()?;
        // Java coerceModelToTextualCommon：classic 模式布尔 → "true"/""（先于 formatBoolean）
        if env.settings.classic_compatible {
            return Ok(if bv {
                "true".to_string()
            } else {
                String::new()
            });
        }
        // Java EvalUtil.coerceModelToStringOrMarkup：formatBoolean(value, false)
        return boolean_format(env, bv, false);
    }
    if let Some(d) = &m.date {
        let d = d.as_date()?;
        let format = match d.kind {
            crate::value::DateType::Date => env.settings.date_format.clone(),
            crate::value::DateType::Time => env.settings.time_format.clone(),
            crate::value::DateType::DateTime => env.settings.date_time_format.clone(),
            // Java newCantFormatUnknownTypeDateException（_MessageUtil.java:38-45）：
            // 未知类型须先 ?date/?time/?datetime；消息含 UNKNOWN_DATE_TO_STRING_TIPS
            crate::value::DateType::Unknown => {
                return Err(TemplateError::misc(
                    "Can't convert the date-like value to string because it isn't known if it's a date (no time part), time or date-time value.\n\n----\nTip: Use ?date, ?time, or ?datetime to tell FreeMarker the exact type.\n----\nTip: If you need a particular format only once, use ?string(pattern), like ?string('dd.MM.yyyy HH:mm:ss'), to specify which fields to display. \n----",
                ))
            }
        };
        return format_date_value(env, &d, &format);
    }
    Err(TemplateError::type_mismatch(
        "string-like value",
        m.type_name,
    ))
}

/// 日期格式化分派 —— 对应 Java `Environment.getTemplateDateFormat` 的格式串解析
/// （getTemplateDateFormatWithoutCache :2304-2333）：
/// `xs...` → XML Schema 格式、`iso...` → ISO 8601 格式、其余 → Java 模式
/// （命名模式 short/medium/long/full 或 SimpleDateFormat 子集）
pub(crate) fn format_date_value(
    env: &Environment,
    d: &crate::value::DateValue,
    format_string: &str,
) -> Result<String> {
    use crate::builtins::iso_date_format::{format_iso_like, is_iso_like, parse_iso_params};
    // Java Environment.java:2184：格式串无效 → "Can't create ... based on format string" 包装
    // （dateformat-iso-like 用例断言消息含 "format string"）
    let r: Result<String> = (|| match is_iso_like(format_string) {
        Some((prefix_len, xs_mode)) => {
            let spec = parse_iso_params(format_string, prefix_len, xs_mode)?;
            format_iso_like(d, &spec, xs_mode, &env.settings.time_zone)
        }
        None => {
            let locale = env.settings.locale.as_str();
            let pattern = crate::builtins::java_date_format::resolve_named_style(
                format_string,
                d.kind,
                locale,
            )
            .unwrap_or_else(|| format_string.to_string());
            crate::builtins::java_date_format::format_java(
                &pattern,
                d,
                locale,
                &env.settings.time_zone,
            )
        }
    })();
    r.map_err(|e| {
        TemplateError::misc(format!(
            "Can't create date/time/datetime format based on format string \"{format_string}\". Reason given: {e}"
        ))
    })
}

/// 日期解析分派 —— 对应 Java `TemplateDateFormat.parse`（ISO/XS/Java 三种格式）
pub(crate) fn parse_date_value(
    env: &Environment,
    s: &str,
    kind: crate::value::DateType,
    format_string: &str,
) -> Result<crate::value::DateValue> {
    use crate::builtins::iso_date_format::{is_iso_like, parse_iso_like, parse_iso_params};
    match is_iso_like(format_string) {
        Some((prefix_len, xs_mode)) => {
            let spec = parse_iso_params(format_string, prefix_len, xs_mode)?;
            parse_iso_like(s, kind, &spec, &env.settings.time_zone, xs_mode)
        }
        None => {
            let locale = env.settings.locale.as_str();
            let pattern =
                crate::builtins::java_date_format::resolve_named_style(format_string, kind, locale)
                    .unwrap_or_else(|| format_string.to_string());
            crate::builtins::java_date_format::parse_java(
                &pattern,
                s,
                kind,
                locale,
                &env.settings.time_zone,
            )
        }
    }
}

/// 表达式求值后转输出字符串（`<#include path>`、`<#stop msg>` 等；缺失 → InvalidReference）
pub(crate) fn eval_to_string(env: &mut Environment, e: &Expr) -> Result<String> {
    let m = eval::eval(env, e)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(expr_desc(e)));
    }
    model_to_string(env, &m)
}

/// 表达式描述（Java `getCanonicalForm()` 的 v1 简化子集；错误消息用）
pub(crate) fn expr_desc(e: &Expr) -> String {
    use crate::core::ExprKind as K;
    match &e.kind {
        K::Str(s) => format!("\"{}\"", s),
        K::Num(n) => n.to_plain_string(),
        K::Bool(b) => b.to_string(),
        K::Ident(n) => n.clone(),
        K::Dot { target, name } => format!("{}.{}", expr_desc(target), name),
        K::DynKey { target, key } => format!("{}[{}]", expr_desc(target), expr_desc(key)),
        K::Call { callee, args } => format!(
            "{}({})",
            expr_desc(callee),
            args.iter().map(expr_desc).collect::<Vec<_>>().join(", ")
        ),
        K::BuiltIn { target, name, .. } => format!("{}?{}", expr_desc(target), name),
        K::Paren(i) => format!("({})", expr_desc(i)),
        K::Not(i) => format!("!{}", expr_desc(i)),
        K::UnaryMinus(i) => format!("-{}", expr_desc(i)),
        K::Add(a, b) => format!("{} + {}", expr_desc(a), expr_desc(b)),
        K::Sub(a, b) => format!("{} - {}", expr_desc(a), expr_desc(b)),
        K::Mul(a, b) => format!("{} * {}", expr_desc(a), expr_desc(b)),
        K::Div(a, b) => format!("{} / {}", expr_desc(a), expr_desc(b)),
        K::Mod(a, b) => format!("{} % {}", expr_desc(a), expr_desc(b)),
        K::Eq(a, b) => format!("{} == {}", expr_desc(a), expr_desc(b)),
        K::NotEq(a, b) => format!("{} != {}", expr_desc(a), expr_desc(b)),
        K::Gt(a, b) => format!("{} > {}", expr_desc(a), expr_desc(b)),
        K::Gte(a, b) => format!("{} >= {}", expr_desc(a), expr_desc(b)),
        K::Lt(a, b) => format!("{} < {}", expr_desc(a), expr_desc(b)),
        K::Lte(a, b) => format!("{} <= {}", expr_desc(a), expr_desc(b)),
        K::And(a, b) => format!("{} && {}", expr_desc(a), expr_desc(b)),
        K::Or(a, b) => format!("{} || {}", expr_desc(a), expr_desc(b)),
        K::Default { target, .. } => format!("{}!", expr_desc(target)),
        K::Exists(t) => format!("{}??", expr_desc(t)),
        _ => "...".to_string(),
    }
}

/// 收集表达式中的标识符名（`<#escape x as x?html>` 的占位符识别；v1 近似
/// Java 解析期替换，见 apply_escape 注释）
fn collect_ident_names(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    collect_ident_names_into(e, &mut out);
    out
}

fn collect_ident_names_into(e: &Expr, out: &mut Vec<String>) {
    use crate::core::ExprKind as K;
    match &e.kind {
        K::Ident(n) => out.push(n.clone()),
        K::InterpStr(parts) => {
            for p in parts {
                if let crate::core::StrPart::Interp(inner) = p {
                    collect_ident_names_into(inner, out);
                }
            }
        }
        K::Dot { target, .. }
        | K::UnaryMinus(target)
        | K::Not(target)
        | K::Exists(target)
        | K::Paren(target) => collect_ident_names_into(target, out),
        K::DynKey { target, key } => {
            collect_ident_names_into(target, out);
            collect_ident_names_into(key, out);
        }
        K::Default { target, default } => {
            collect_ident_names_into(target, out);
            if let Some(d) = default {
                collect_ident_names_into(d, out);
            }
        }
        K::Add(a, b)
        | K::Sub(a, b)
        | K::Mul(a, b)
        | K::Div(a, b)
        | K::Mod(a, b)
        | K::Eq(a, b)
        | K::NotEq(a, b)
        | K::Gt(a, b)
        | K::Gte(a, b)
        | K::Lt(a, b)
        | K::Lte(a, b)
        | K::And(a, b)
        | K::Or(a, b) => {
            collect_ident_names_into(a, out);
            collect_ident_names_into(b, out);
        }
        K::Range { start, end, .. } => {
            collect_ident_names_into(start, out);
            if let Some(end) = end {
                collect_ident_names_into(end, out);
            }
        }
        K::BuiltIn { target, args, .. } => {
            collect_ident_names_into(target, out);
            if let Some(args) = args {
                for a in args {
                    collect_ident_names_into(a, out);
                }
            }
        }
        K::Call { callee, args } => {
            collect_ident_names_into(callee, out);
            for a in args {
                collect_ident_names_into(a, out);
            }
        }
        K::ListLit(items) => {
            for i in items {
                collect_ident_names_into(i, out);
            }
        }
        K::HashLit(pairs) => {
            for (k, v) in pairs {
                collect_ident_names_into(k, out);
                collect_ident_names_into(v, out);
            }
        }
        K::Lambda { body, .. } => collect_ident_names_into(body, out),
        _ => {}
    }
}

/// 错误附加源码位置 —— `[in template "name" at line L, column C]`（docs/09 §2 消息模板）。
/// 只附加一次（消息已含 "[in template" 则跳过）；Flow/Stop/Parse/Io 不附加
/// （Flow 是流控信号；Stop 是 Java StopException 语义，自带消息）。
fn attach_location(err: TemplateError, template_name: &str, span: Span) -> TemplateError {
    let loc = format!(
        "[in template \"{template_name}\" at line {}, column {}]",
        span.line, span.col
    );
    match err {
        TemplateError::InvalidReference { name } => {
            if name.contains("[in template") {
                TemplateError::InvalidReference { name }
            } else {
                TemplateError::InvalidReference {
                    name: format!("{name}  {loc}"),
                }
            }
        }
        TemplateError::TypeMismatch { expected, actual } => {
            if actual.contains("[in template") {
                TemplateError::TypeMismatch { expected, actual }
            } else {
                TemplateError::TypeMismatch {
                    expected,
                    actual: format!("{actual}  {loc}"),
                }
            }
        }
        TemplateError::Misc { message } => {
            if message.contains("[in template") {
                TemplateError::Misc { message }
            } else {
                TemplateError::Misc {
                    message: format!("{message}  {loc}"),
                }
            }
        }
        TemplateError::Model { message } => {
            if message.contains("[in template") {
                TemplateError::Model { message }
            } else {
                TemplateError::Model {
                    message: format!("{message}  {loc}"),
                }
            }
        }
        other => other,
    }
}

// ---------------------------------------------------------------------------
// 测试辅助（端到端渲染）
// ---------------------------------------------------------------------------

/// 模板路径规范化（`./`/`../` 段折叠；Java toFullTemplateName 的语义）
fn normalize_template_path(p: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().map(|s| *s != "..").unwrap_or(false) {
                    out.pop();
                } else {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    if out.is_empty() {
        return String::new();
    }
    out.join("/")
}

/// Java TemplateCache.lookupTemplateWithAcquisitionStrategy 的 acquisition 语义
/// （TemplateCache.java:742-788）：输入为 toFullTemplateName 后的完整路径；
/// 分词后取最后一个 `*`（重复 `*` 段先删除），`*` 之前的段为 basePath、之后为
/// resourcePath；候选 = basePath(完整→逐级去尾段→空) + resourcePath，首个找到即返回。
/// 不含 `*` 原样返回。
fn acquisition_candidates(full: &str) -> Vec<String> {
    let mut cleaned: Vec<&str> = Vec::new();
    let mut last_asterisk: Option<usize> = None;
    for t in full.split('/') {
        if t == "*" {
            if let Some(idx) = last_asterisk {
                cleaned.remove(idx);
            }
            last_asterisk = Some(cleaned.len());
        }
        cleaned.push(t);
    }
    let Some(ai) = last_asterisk else {
        return vec![full.to_string()];
    };
    let resource = cleaned[ai + 1..].join("/");
    let mut out = Vec::new();
    let mut l = ai;
    loop {
        let mut p = cleaned[..l].join("/");
        if !p.is_empty() {
            p.push('/');
        }
        out.push(format!("{p}{resource}"));
        if l == 0 {
            break;
        }
        l -= 1; // Java：basePath.lastIndexOf(SLASH, l-2)+1 → 段级等价于去尾段
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
    use std::sync::Arc;

    fn cfg() -> (Configuration, Arc<StringLoader>) {
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        (c, loader)
    }

    /// 加载模板 + 渲染，返回输出（wrapper 为 SimpleObjectWrapper）
    fn render_src(
        c: &Configuration,
        loader: &Arc<StringLoader>,
        name: &str,
        src: &str,
        root: DynValue,
    ) -> Result<String> {
        loader.put(name, src);
        let t = c.get_template(name)?;
        let root_model = SimpleObjectWrapper
            .wrap(&root)?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn helloworld_e2e() {
        let (c, loader) = cfg();
        let out = render_src(
            &c,
            &loader,
            "hello.ftl",
            "Hello, ${name}!",
            DynValue::Map(vec![("name".into(), DynValue::Str("world".into()))]),
        )
        .unwrap();
        assert_eq!(out, "Hello, world!");
    }

    #[test]
    fn undefined_variable_errors_with_name() {
        let (c, loader) = cfg();
        let err =
            render_src(&c, &loader, "err.ftl", "${missing}", DynValue::Map(vec![])).unwrap_err();
        match err {
            TemplateError::InvalidReference { name } => {
                assert!(name.contains("missing"), "{name}");
                assert!(name.contains("[in template"), "{name}");
            }
            other => panic!("expected InvalidReference, got {other:?}"),
        }
    }
}
