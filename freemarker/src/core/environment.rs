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
use crate::core::{
    AssignOp, CallTarget, Element, ElementKind, Expr, MacroDef, MacroParam, Settings, TzSetting,
};
use crate::error::{ErrorCtx, Result, StackFrame, TemplateError};
use crate::span::Span;
use crate::template::{TModel, Template, TemplateHashModel, TemplateHashModelEx};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::{Rc, Weak};

/// 单次渲染允许的最大模板包含层数。
///
/// 防止 `<#include>` 自包含或 A → B → A 环路耗尽调用栈。该限制只约束当前
/// 包含链；同一模板在前一次包含返回后可再次包含。
const MAX_INCLUDE_DEPTH: usize = 16;

/// 单次渲染允许的最大宏/函数调用深度，防止无终止递归耗尽调用栈。
const MAX_MACRO_CALL_DEPTH: usize = 16;

/// 单次输出或捕获缓冲允许的最大字节数。
///
/// 引擎在成功结束时才写出 `output_buffer`，因此必须在写入时限制其大小，避免
/// 无界循环或异常输入把宿主进程的内存耗尽。
const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

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
    /// 输出缓冲（内部始终 UTF-8；process() 结束时按 output_encoding 转码写出）
    output_buffer: Vec<u8>,
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
    /// 当前包含链（含主模板）。用于限制包含嵌套深度；同名递归可由模板状态主动终止，
    /// 因此不能仅凭名称判为环。
    include_stack: Vec<String>,
    /// 宏调用帧栈（栈顶 = Java `currentMacroContext` :174；`<#local>`/`<#nested>`/`<#return>` 依赖）
    pub(crate) macro_frames: Vec<Rc<MacroFrame>>,
    /// 访问节点栈（Java `visitorStack`，Environment.java:109；`<#visit>` 压入、
    /// 宏体结束弹出；`.node` 读栈顶）
    visit_stack: Vec<TModel>,
    /// `<#return>` 发起时的宏帧深度（Java `Return.INSTANCE` 携带发起 Macro.Context 的
    /// 等价物——Macro.invoke 的 catch(Return) 按 macroCtx 归属判定捕获；穿透的
    /// return（如 `<@b><#return></@>` 中 return 归调用者宏 m 而非被调宏 b）继续上传）
    pub(crate) return_depth: Option<usize>,
    /// 运行时设置快照（Java Configurable 继承链；v1 单层，`<#setting>` 修改此副本）。
    /// Cow：默认借用 Configuration 的设置（渲染零克隆）；`<#setting>`/`<#outputformat>`
    /// 首次修改时 to_mut() 惰性深克隆（此后原地修改）。
    pub(crate) settings: std::borrow::Cow<'a, Settings>,
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
    /// 当前模板的 ns_prefixes（`<#ftl ns_prefixes=...>`；include 沿用主模板、
    /// import 用库模板自己的——Java currentNamespace.getTemplate().getNamespaceForPrefix）
    current_ns_prefixes: HashMap<String, String>,
    /// attempt/recover 错误栈（Java `recoveredErrorStack`，Environment.java:575-578：
    /// recover 期间压栈、结束弹出——嵌套 attempt 的内层 recover 结束后 `.error`
    /// 恢复为外层错误；BuiltinVariable.java:283-285 `.error` 读栈顶）
    pub(crate) recovered_errors: Vec<String>,
    /// 数字格式解析缓存（默认 `#,##0.###` 模式的 DecimalFmt；首次使用时解析，
    /// 此后直接复用——热路径（`${n}` 循环输出）避免每次重新解析模式串）。
    /// 键为 (number_format, locale)，`<#setting>` 改动任一后自然失效重解析。
    pub(crate) number_fmt_cache: RefCell<
        Option<(
            String,
            String,
            std::rc::Rc<crate::builtins::format::DecimalFmt>,
        )>,
    >,
    /// FTL 指令栈 —— 对应 Java `Environment.instructionStack`（:3563+ pushElement/popElement）：
    /// 每个可描述元素执行前压帧、执行后弹帧（docs/09 §6.4）。错误发生时
    /// `stack_snapshot` 取其快照（栈顶帧 + 其余 isShownInStackTrace 帧），经
    /// `TemplateError::with_stack` 附加到错误消息（`----\nFTL stack trace ...` 段）。
    /// 平行栈 `stack_shown` 记录各帧是否属于 Java 的显示集合（Interpolation/UnifiedCall/
    /// Include/LibraryLoad/BodyInstruction/Transform/Visit/Recurse/Fallback——
    /// 对应各类的 isShownInStackTrace() 覆盖，jar 实测；栈顶失败帧不受此限制）。
    pub(crate) instruction_stack: Vec<StackFrame>,
    /// 平行栈：对应帧是否在 Java 快照过滤中显示（见 instruction_stack 注释）
    stack_shown: Vec<bool>,
    /// 当前词法包围宏名（帧位置 `in macro "m"` 段 —— Java `getEnclosingMacro`
    /// 沿父元素链找最近 Macro 元素的等价物；宏体执行时置名、`<#nested>` 回插时
    /// 恢复调用方值）
    pub(crate) current_macro_name: Option<String>,
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
/// `has_next` 前视：从迭代器拉一项入 cache（peek 语义，IteratorBlock.java:293-300）。
/// 有界范围（`1..100`）走 `range` 快路径：has_next 按 index/cap 判定（O(1) 零物化，
/// 对应 Java BoundedRangeModel 迭代器同样不构造下一项值）。
pub(crate) struct PendingItems {
    cache: std::collections::VecDeque<LoopItem>,
    iter: Option<Box<dyn Iterator<Item = Result<LoopItem>>>>,
    /// 范围快路径状态（start ± index 按需取值；仅遍历，不物化 ahead）
    range: Option<RangeIterState>,
}

/// 范围迭代状态（start + ascending*index；cap = 元素总数）
#[derive(Clone, Copy)]
pub(crate) struct RangeIterState {
    pub start: i64,
    pub index: usize,
    pub cap: usize,
    pub ascending: bool,
}

impl PendingItems {
    /// 已物化来源（hashListing / TemplateSequenceModel size-get 路径）
    pub(crate) fn eager(items: std::collections::VecDeque<LoopItem>) -> Self {
        PendingItems {
            cache: items,
            iter: None,
            range: None,
        }
    }

    /// 惰性来源（TemplateCollectionModel 迭代器）
    pub(crate) fn lazy(iter: Box<dyn Iterator<Item = Result<LoopItem>>>) -> Self {
        PendingItems {
            cache: std::collections::VecDeque::new(),
            iter: Some(iter),
            range: None,
        }
    }

    /// 有界范围来源（`1..100` 等；has_next 零物化前视）
    pub(crate) fn range(state: RangeIterState) -> Self {
        PendingItems {
            cache: std::collections::VecDeque::new(),
            iter: None,
            range: Some(state),
        }
    }

    /// 取下一项（无 → None）；迭代器错误向上传播
    pub(crate) fn pop(&mut self) -> Result<Option<LoopItem>> {
        if let Some(item) = self.cache.pop_front() {
            return Ok(Some(item));
        }
        if let Some(r) = &mut self.range {
            if r.index < r.cap {
                let v = if r.ascending {
                    r.start + r.index as i64
                } else {
                    r.start - r.index as i64
                };
                r.index += 1;
                return Ok(Some(LoopItem {
                    key: None,
                    value: Some(TModel::from_number(crate::value::TNumber::from_i64(v))),
                }));
            }
            return Ok(None);
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

    /// 是否还有下一项（前视：从迭代器拉一项进 cache；范围路径 O(1) 判定）
    pub(crate) fn has_next(&mut self) -> Result<bool> {
        if !self.cache.is_empty() {
            return Ok(true);
        }
        if let Some(r) = &self.range {
            return Ok(r.index < r.cap);
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
        // `x_index`/`x_has_next` 判定：strip_suffix 零分配（format! 每次变量查找
        // 都会构造新 String——循环体内热路径）
        if let Some(prefix) = name.strip_suffix("_index") {
            if prefix == self.var_name {
                return Some(TModel::from_number(crate::value::TNumber::from_i64(
                    self.index as i64,
                )));
            }
        }
        if let Some(prefix) = name.strip_suffix("_has_next") {
            if prefix == self.var_name {
                return Some(TModel::from_boolean(self.has_next));
            }
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
    /// FNV 哈希（热路径查找）
    pub(crate) locals: RefCell<HashMap<String, TModel, crate::utility::FnvBuildHasher>>,
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
    /// 调用方词法宏名（`<#nested>` 回插调用方 body 时 current_macro_name 恢复用——
    /// 调用方 body 元素的 `in macro "m"` 定位；Java 父元素链的宏归属）
    pub(crate) caller_macro_name: Option<String>,
    /// `.args` 特殊变量值（Java Macro.Context.argsSpecialVariableValue——
    /// checkParamsSetAndApplyDefaults :344-397：macro → 参数哈希、function → 参数序列）
    ///
    /// **惰性构建**：Java 只在模板实际访问 `.args` 时构建该值（BuiltinVariable.Args），
    /// 因此位置 catch-all 非空的"仅 .args 才报错"限制只在访问时触发；不访问 `.args`
    /// 的宏（如 `<@m 1 2 3/>` 纯位置调用）不受影响。构建依赖宏定义与函数/宏标志，
    /// 故帧保存快照，首次访问时经 `build_args_special` 填充。
    /// Box 内嵌：绝大多数宏不访问 `.args`（None 常驻），避免 TModel 内联撑大帧分配。
    pub(crate) args_value: RefCell<Option<Box<TModel>>>,
    /// 宏定义快照（供惰性构建 `.args`）。
    pub(crate) def: Rc<MacroDef>,
    /// 是否为函数（供惰性构建 `.args`：函数 → 序列、宏 → 哈希）。
    pub(crate) is_function: bool,
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
    /// 宏所属命名空间（Java `macroToNamespaceLookup` :185；宏体内 currentNamespace 切换）。
    ///
    /// 必须为 `Weak`：`Namespace.macros` 已强持有 `MacroValue`，若此处再强持有
    /// `Namespace`，每个含宏的渲染环境都会形成不可释放的 `Rc` 环。
    pub ns: Weak<Namespace>,
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
/// 变量/宏表用 FNV 哈希（热路径查找；迭代序无依赖——keys() 已排序）。
pub struct Namespace {
    vars: RefCell<HashMap<String, TModel, crate::utility::FnvBuildHasher>>,
    macros: RefCell<HashMap<String, Rc<MacroValue>, crate::utility::FnvBuildHasher>>,
    /// 关联模板名（Java `Namespace.getTemplate()` :3470-3478；错误定位/相对 include 基名）
    /// Rc<str>：主/全局命名空间共享同一份（Environment::new 构造期 1 次分配）
    template_name: Rc<str>,
    /// 所属模板的 ns_prefixes（`<#ftl ns_prefixes=...>`；宏体内 currentNamespace
    /// 切换时随之切换——Java currentNamespace.getTemplate().getNamespaceForPrefix）
    ns_prefixes: RefCell<HashMap<String, String>>,
}

impl Namespace {
    fn new(template_name: String) -> Self {
        Namespace {
            vars: RefCell::new(HashMap::with_hasher(
                crate::utility::FnvBuildHasher::default(),
            )),
            macros: RefCell::new(HashMap::with_hasher(
                crate::utility::FnvBuildHasher::default(),
            )),
            template_name: Rc::from(template_name),
            ns_prefixes: RefCell::new(HashMap::new()),
        }
    }

    /// 共享模板名构造（主/全局命名空间复用同一 Rc<str>）
    fn new_shared(template_name: Rc<str>) -> Self {
        Namespace {
            vars: RefCell::new(HashMap::with_hasher(
                crate::utility::FnvBuildHasher::default(),
            )),
            macros: RefCell::new(HashMap::with_hasher(
                crate::utility::FnvBuildHasher::default(),
            )),
            template_name,
            ns_prefixes: RefCell::new(HashMap::new()),
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

    /// 仅变量表读取（宏快路径用：变量存在则以变量为准——可遮蔽宏）
    pub(crate) fn get_variable_only(&self, name: &str) -> Option<TModel> {
        self.vars.borrow().get(name).cloned()
    }

    /// 仅宏表读取（宏快路径用）
    pub(crate) fn get_macro(&self, name: &str) -> Option<Rc<MacroValue>> {
        self.macros.borrow().get(name).cloned()
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
        // Java SimpleHash.size 含宏——变量 + 宏定义总数
        Ok(self.vars.borrow().len() + self.macros.borrow().len())
    }
    fn keys(&self) -> Result<Vec<String>> {
        // Java SimpleHash.keys() 返回 LinkedHashMap 插入序的所有键（含宏定义）；
        // Rust HashMap 无序遍历 → 排序保证确定性
        let mut k: Vec<String> = self.vars.borrow().keys().cloned().collect();
        k.extend(self.macros.borrow().keys().cloned());
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
        let base_settings = &template.configuration.settings;
        let base_time_zone = base_settings.time_zone;
        let base_time_zone_id = base_settings.time_zone_id.clone();
        let main_ns = Rc::new(Namespace::new(template.name.clone()));
        *main_ns.ns_prefixes.borrow_mut() = template.ns_prefixes.clone();
        for (name, def) in &template.macros {
            register_macro(&main_ns, name, def);
        }
        let current_ns = main_ns.clone();
        // 主/全局命名空间共享模板名（构造期 1 次 Rc 分配替代 2 次 String 克隆）
        let name_shared: Rc<str> = Rc::from(template.name.as_str());
        let global_ns = Rc::new(Namespace::new_shared(name_shared));
        // Java autoEscaping 默认随 outputFormat 与 incompatibleImprovements（docs/08 §1）
        let auto_escape = match base_settings.auto_escaping {
            crate::core::AutoEscaping::On => true,
            crate::core::AutoEscaping::Off => false,
            crate::core::AutoEscaping::Default => base_settings.output_format.is_markup(),
        };
        Environment {
            template,
            root,
            out,
            local_stack: Vec::new(),
            main_ns,
            current_ns,
            global_ns,
            loaded_libs: HashMap::new(),
            include_stack: vec![template.name.clone()],
            macro_frames: Vec::new(),
            visit_stack: Vec::new(),
            return_depth: None,
            settings: std::borrow::Cow::Borrowed(base_settings),
            base_time_zone,
            base_time_zone_id,
            attempt_depth: 0,
            redirect: None,
            // 预分配输出缓冲（小模板避免多次扩容拷贝；大模板按需增长）
            output_buffer: Vec::with_capacity(128),
            escapes: Vec::new(),
            auto_escape,
            current_template_name: template.name.clone(),
            current_ns_prefixes: template.ns_prefixes.clone(),
            recovered_errors: Vec::new(),
            number_fmt_cache: RefCell::new(None),
            instruction_stack: Vec::new(),
            stack_shown: Vec::new(),
            current_macro_name: None,
        }
    }

    /// 渲染入口 —— 对应 Java `Environment.process()`（:315-336）：
    /// 执行根元素，按 output_encoding 将内部 UTF-8 缓冲转码后写出，最后 flush。
    /// 根层错误按 `template_exception_handler` 设置处理（docs/09 §6.3；Java 在
    /// `handleTemplateException` :1199-1235 逐错误处理，v1 在 process() 边界统一处理）：
    /// - `rethrow`：原样上传（生产默认）
    /// - `ignore`：吞掉错误，不写任何输出（Java 保留已输出内容并继续渲染——
    ///   v1 文档化偏差，等价 `<#attempt>` 语义）
    /// - `debug`：把 `FreeMarker template error (DEBUG mode; use RETHROW in
    ///   production!):` 前缀 + 完整消息 + `(Java stack trace omitted)` 段写入输出并
    ///   视为成功（Java 写后仍抛出——v1 文档化偏差）
    /// - `html_debug`：同上，消息 HTML 转义
    pub fn process(&mut self) -> Result<()> {
        if !self.root.is_hash() {
            return Err(TemplateError::misc("The data model must be a hash"));
        }
        // 引用拷贝技巧：先复制 &Template 引用再借 root，避免整棵根元素树深克隆
        // （run 零克隆执行，见 run_slice）
        let t = self.template;
        match self.run(&t.root) {
            Ok(signal) => match signal {
                RunSignal::Completed => {
                    let output_encoding = &self.settings.output_encoding;
                    if output_encoding.eq_ignore_ascii_case("UTF-8") || output_encoding.is_empty() {
                        // UTF-8 或未指定：缓冲中的 UTF-8 直接写出
                        self.out
                            .write_all(&self.output_buffer)
                            .map_err(TemplateError::Io)?;
                    } else {
                        // 非 UTF-8：将 UTF-8 缓冲转码为输出编码
                        let encoded = transcode_output(&self.output_buffer, output_encoding)?;
                        self.out.write_all(&encoded).map_err(TemplateError::Io)?;
                    }
                    self.out.flush().map_err(TemplateError::Io)
                }
                RunSignal::Returned(_) => Err(TemplateError::misc(
                    "<#return> is illegal here (not inside a macro or function)",
                )),
            },
            Err(e) => self.handle_root_error(e),
        }
    }

    /// process() 根层错误 → template_exception_handler 分发（rethrow 原样上传；
    /// debug/html_debug 写出调试文本后视为成功；ignore 吞掉不写输出）
    fn handle_root_error(&mut self, e: TemplateError) -> Result<()> {
        match self.settings.template_exception_handler.as_str() {
            "rethrow" => Err(e),
            "ignore" => Ok(()),
            "debug" => {
                // Java DebugTemplateExceptionHandler：写 "FreeMarker template error
                // (DEBUG mode; use RETHROW in production!):\n" + printStackTrace
                // （消息 + FTL 栈 + Java 栈）后仍抛出——v1 省略 Java 栈段（docs/09 §4
                // 容忍清单）且视为成功（文档化偏差）
                let msg = e.to_user_message();
                let out = format!(
                    "FreeMarker template error (DEBUG mode; use RETHROW in production!):\n\
                     {msg}\n\
                     ----\n(Java stack trace omitted)"
                );
                self.out
                    .write_all(out.as_bytes())
                    .map_err(TemplateError::Io)?;
                self.out.flush().map_err(TemplateError::Io)
            }
            "html_debug" => {
                // Java HtmlDebugTemplateExceptionHandler：HTML 转义 + 巨型装饰块——
                // v1 与 debug 同形（消息转义），文档化偏差（docs/09 §6.3）
                let msg = crate::utility::html_escape(&e.to_user_message());
                let out = format!(
                    "FreeMarker template error (DEBUG mode; use RETHROW in production!):\n\
                     {msg}\n\
                     ----\n(Java stack trace omitted)"
                );
                self.out
                    .write_all(out.as_bytes())
                    .map_err(TemplateError::Io)?;
                self.out.flush().map_err(TemplateError::Io)
            }
            other => {
                // 非法值在 exec_setting / apply_settings 已被拒——防御性兜底
                Err(TemplateError::misc(format!(
                    "Invalid template_exception_handler value: {other}"
                )))
            }
        }
    }

    /// 执行一组元素 —— Java `visit(TemplateElement[])`（:367-405）的等价物。
    /// 零克隆驱动：`els` 借引用执行（`run_slice`），Next/Replace 产物进本地 mini 栈；
    /// - `Next(children)`：子元素入栈（逆序保证执行顺序）；`Replace`：入栈替换；
    /// - `ReturnValue` → RunSignal::Returned（Java ReturnInstruction.Return 异常语义）；
    /// - `Flow`/`Stop` → Err 上传（Java RuntimeException / StopException 穿透）；
    /// - 其他错误：附加源码位置（模板名 + 行列，docs/09 §2）后上传。
    ///   嵌套调用（宏体/指令 body/捕获块）各持有自己的 mini 栈，外层待执行元素
    ///   不受影响（旧实现以指令栈保存/恢复达成同一效果）。
    pub(crate) fn run(&mut self, els: &[Element]) -> Result<RunSignal> {
        self.run_slice(els)
    }

    /// 切片驱动：els 借引用执行（零元素克隆）；Next/Replace 产物压入本地 mini 栈
    /// （子元素优先于后续元素——与旧栈驱动一致）。Returned/Flow/Stop 返回时
    /// mini 栈遗留元素丢弃（旧实现中由 run() 的栈保存/恢复实现同样的丢弃）。
    fn run_slice(&mut self, els: &[Element]) -> Result<RunSignal> {
        let mut mini: Vec<Element> = Vec::new();
        let mut i = 0usize;
        loop {
            if let Some(el) = mini.pop() {
                let span = el.span;
                // 指令帧：执行前压入（栈顶 = 失败帧；Java pushElement/visit）；
                // 错误在弹帧前附加快照（Java TemplateException 构造时取快照）
                self.push_instruction_frame(&el);
                let outcome = crate::core::exec::exec_owned(self, el);
                let sig = match self.consume_outcome(outcome, span, &mut mini) {
                    Ok(sig) => sig,
                    Err(e) => {
                        let e = self.attach_stack_to_error(e);
                        self.pop_instruction_frame();
                        return Err(e);
                    }
                };
                self.pop_instruction_frame();
                if let Some(sig) = sig {
                    return Ok(sig);
                }
            } else if i < els.len() {
                let el = &els[i];
                i += 1;
                let span = el.span;
                self.push_instruction_frame(el);
                let outcome = crate::core::exec::exec(self, el);
                let sig = match self.consume_outcome(outcome, span, &mut mini) {
                    Ok(sig) => sig,
                    Err(e) => {
                        let e = self.attach_stack_to_error(e);
                        self.pop_instruction_frame();
                        return Err(e);
                    }
                };
                self.pop_instruction_frame();
                if let Some(sig) = sig {
                    return Ok(sig);
                }
            } else {
                break;
            }
        }
        Ok(RunSignal::Completed)
    }

    /// 压入指令帧 —— 对应 Java `pushElement`（Environment.java:3563+）。
    /// 仅可描述元素压帧（Java 快照过滤含 "description 非空" 前提；Text/Comment/
    /// 空白指令等无描述 → 不压，与 Java 的 getDescription()==null 跳过一致）。
    /// 帧位置（模板名/行列/`in macro "m"`）取当前渲染上下文（include 时模板名
    /// 已切换；宏体内 macro 名由 current_macro_name 提供）。
    fn push_instruction_frame(&mut self, el: &Element) {
        let Some(desc) = describe_element(self, el) else {
            return;
        };
        let shown = element_shown_in_stack_trace(el);
        // `<#nested>` 帧标记嵌套（Java BodyInstruction——打印 `~` 且紧随帧同标）
        let nesting = matches!(el.kind, ElementKind::Nested { .. });
        let frame = StackFrame {
            description: desc,
            template_name: self.current_template_name.clone(),
            line: el.span.line,
            col: el.span.col,
            in_macro: self.current_macro_name.clone(),
            nesting,
        };
        self.instruction_stack.push(frame);
        self.stack_shown.push(shown);
    }

    /// 弹出指令帧（对应 Java `popElement`；执行成功或错误上传后均弹——错误已
    /// 在 consume_outcome 阶段经 attach_stack_to_error 携带自己的快照拷贝）
    fn pop_instruction_frame(&mut self) {
        self.instruction_stack.pop();
        self.stack_shown.pop();
    }

    /// 错误附加指令栈快照（Java `TemplateException` 构造时
    /// `env.getInstructionStackSnapshot()` 的等价物；已带栈的错误为 no-op——
    /// with_stack 幂等）——在弹帧前调用（快照须含当前失败帧）
    pub(crate) fn attach_stack_to_error(&self, e: TemplateError) -> TemplateError {
        e.with_stack(self.stack_snapshot())
    }

    /// 指令栈快照 —— 对应 Java `getInstructionStackSnapshot()`（Environment.java:2690+）：
    /// 自栈顶向下（最新帧在前）取「栈顶帧（总是显示，Java 末帧无条件包含）+
    /// 其余 isShownInStackTrace 帧」；空栈 → 空
    pub(crate) fn stack_snapshot(&self) -> Vec<StackFrame> {
        let mut out = Vec::new();
        let mut first = true; // 栈顶（最新）帧总是显示
        for (frame, shown) in self
            .instruction_stack
            .iter()
            .zip(self.stack_shown.iter())
            .rev()
        {
            let keep = first || *shown;
            first = false;
            if keep {
                out.push(frame.clone());
            }
        }
        out
    }

    /// exec 结果消费（Next/Replace → mini 栈；Returned → 信号；Flow/Stop/Err → 上传）
    fn consume_outcome(
        &mut self,
        outcome: Result<crate::core::exec::ExecOutcome>,
        span: Span,
        mini: &mut Vec<Element>,
    ) -> Result<Option<RunSignal>> {
        match outcome {
            Ok(crate::core::exec::ExecOutcome::Next(children)) => {
                for c in children.into_iter().rev() {
                    mini.push(c);
                }
                Ok(None)
            }
            Ok(crate::core::exec::ExecOutcome::Replace(e)) => {
                mini.push(e);
                Ok(None)
            }
            Ok(crate::core::exec::ExecOutcome::Done) => Ok(None),
            Ok(crate::core::exec::ExecOutcome::ReturnValue(v)) => Ok(Some(RunSignal::Returned(v))),
            Ok(crate::core::exec::ExecOutcome::Flow(k)) => Err(TemplateError::Flow(k)),
            Ok(crate::core::exec::ExecOutcome::Stop(m)) => Err(TemplateError::Stop { message: m }),
            Err(e) => Err(attach_location(e, &self.current_template_name, span)),
        }
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
        if self.macro_frames.len() >= MAX_MACRO_CALL_DEPTH {
            return Err(TemplateError::misc(format!(
                "Maximum macro/function call depth ({MAX_MACRO_CALL_DEPTH}) exceeded."
            )));
        }
        // Java :848-879：宏帧 + 参数绑定（求值发生在调用方上下文）
        let frame = Rc::new(MacroFrame {
            locals: RefCell::new(HashMap::with_hasher(
                crate::utility::FnvBuildHasher::default(),
            )),
            call_body: body,
            body_params,
            caller_ns: self.current_ns.clone(),
            caller_local_stack: self.local_stack.clone(),
            caller_macro_name: self.current_macro_name.clone(),
            args_value: RefCell::new(None),
            def: mv.def.clone(),
            is_function,
        });
        // 无参数宏：跳过参数绑定（空循环开销——热路径 `<@m/>` 调用）
        if !mv.def.params.is_empty() {
            bind_macro_args(self, &frame, &mv.def, args)?;
        }
        // Java :880-894：压帧、切换命名空间、清空局部上下文
        self.macro_frames.push(frame.clone());
        let prev_ns = self.current_ns.clone();
        let prev_ns_prefixes = self.current_ns_prefixes.clone();
        self.current_ns = mv
            .ns
            .upgrade()
            .ok_or_else(|| TemplateError::misc("The macro's namespace is no longer available."))?;
        // 宏体内 ns_prefixes 随宏所属命名空间切换（Java 宏的 currentNamespace）；
        // 空映射跳过 clone（热路径优化：多数模板无 ns_prefixes；borrow 立即释放）
        let ns_prefixes: HashMap<String, String> = self.current_ns.ns_prefixes.borrow().clone();
        if !ns_prefixes.is_empty() {
            self.current_ns_prefixes = ns_prefixes;
        }
        let prev_local = std::mem::take(&mut self.local_stack);
        // Java ICI 2.3.28+：宏定义帧在参数绑定（setMacroContextLocalsFromArguments）
        // **之后**、默认参数求值（checkParamsSetAndApplyDefaults）**之前**压入
        // （invokeMacroOrFunctionCommonPart 的 pushElement(macro)，jar 实测）——
        // "required parameter 未指定"/"默认值缺失"以 `#macro m a` 为失败帧；
        // 参数过多/未声明在绑定期报错，失败帧为调用元素 `@m 1, 2`。宏体执行期间
        // 该帧为"不可显示"帧（Macro 非 isShownInStackTrace），仅作失败帧候选
        let prev_macro_name = self.current_macro_name.clone();
        self.push_macro_frame(&mv.def);
        self.current_macro_name = Some(mv.def.name.clone());
        let r = self.run_macro_body(frame, is_function);
        // 错误在弹宏帧前附加快照（Java 异常创建时取快照——默认参数/宏体错误含
        // `#macro m` 帧；宏体错误已被最深层 run_slice 附加，with_stack 幂等）
        let r = r.map_err(|e| self.attach_stack_to_error(e));
        // Java finally :895-901：恢复（错误路径同样还原现场——错误已携带快照拷贝）
        self.current_ns = prev_ns;
        self.current_ns_prefixes = prev_ns_prefixes;
        self.local_stack = prev_local;
        self.macro_frames.pop();
        self.pop_instruction_frame();
        self.current_macro_name = prev_macro_name;
        r
    }

    /// 宏/函数体执行（invoke_macro_common 已压宏帧并切换上下文后调用）：
    /// 默认参数求值 + 宏体/函数体 run；`<#return>` 归属判定（Java Macro.invoke 的
    /// catch(Return)——穿透的 return 继续上传）
    fn run_macro_body(&mut self, frame: Rc<MacroFrame>, is_function: bool) -> Result<RunSignal> {
        // Java :893 checkParamsSetAndApplyDefaults（宏上下文内求值默认参数；
        // 必须在压帧/清空局部上下文之后——默认值表达式经宏帧局部变量解析）
        if !frame.def.params.is_empty() {
            apply_macro_defaults(self, &frame, &frame.def)?;
        }
        // Java :344-397：`.args` 特殊变量值**惰性**构建——仅在模板访问 `.args` 时
        // 由 build_args_special 填充（Java BuiltinVariable.Args 访问时才构造，且
        // "位置 catch-all 非空 + .args" 限制只在访问时触发）；此处不再急切构建
        let sig = if is_function {
            let sig = self
                .capture(|env| env.run(&frame.def.body))
                .map(|(sig, _)| sig)?;
            // Java Macro.invoke 的 catch(Return) 归属判定：return 由本函数帧发起
            // （深度匹配）才作为返回值捕获；穿透的 return（更外层宏）继续上传
            if let RunSignal::Returned(_) = &sig {
                if self.return_depth == Some(self.macro_frames.len()) {
                    self.return_depth = None;
                }
            }
            sig
        } else {
            let sig = self.run(&frame.def.body)?;
            // Java Macro.invoke：宏边界捕获归属本帧的 return（宏不能 return 值 →
            // 值恒 None，捕获即宏正常完成）；穿透的 return 继续上传
            if let RunSignal::Returned(_) = &sig {
                if self.return_depth == Some(self.macro_frames.len()) {
                    self.return_depth = None;
                    RunSignal::Completed
                } else {
                    sig
                }
            } else {
                sig
            }
        };
        Ok(sig)
    }

    /// 压入宏定义帧（`#macro m a` —— 对应 Java `pushElement(macro)`；
    /// 位置 `in macro "m"` 取宏自身：getEnclosingMacro 沿父链首个 Macro 即自身）
    fn push_macro_frame(&mut self, def: &MacroDef) {
        self.instruction_stack.push(StackFrame {
            description: macro_def_description(def),
            template_name: self.current_template_name.clone(),
            line: def.span.line,
            col: def.span.col,
            in_macro: Some(def.name.clone()),
            nesting: false,
        });
        self.stack_shown.push(false);
    }

    /// 压入访问节点（Java `visitStack.push`；`<#visit>` 分派前）
    pub(crate) fn push_visitor_node(&mut self, node: TModel) {
        self.visit_stack.push(node);
    }

    /// 弹出访问节点（Java `visitStack.pop`；`<#visit>` 分派完成后）
    pub(crate) fn pop_visitor_node(&mut self) {
        self.visit_stack.pop();
    }

    /// 当前访问节点（Java `getCurrentVisitorNode` :2931-2933；非 visit 上下文 → None）
    pub(crate) fn get_current_visitor_node(&self) -> Option<TModel> {
        self.visit_stack.last().cloned()
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
            let err = last_err.unwrap_or(TemplateError::NotFound { name: full });
            // Java Include.accept（Include.java:73-90）：加载失败（模板缺失/被包含
            // 模板解析错误）→ "Template inclusion failed (for parameter value
            // \"{name}\"):\n{原因}"（jar 实测 include_not_found / include_parse_error
            // 基线；被包含模板体内部的渲染错误不加此包装——include_template 路径）
            return Err(TemplateError::misc(format!(
                "Template inclusion failed (for parameter value \"{name}\"):\n{}",
                err.to_user_message()
            )));
        };
        self.include_template(&t)
    }

    /// 执行被包含模板（Java include(includedTemplate) :3126-3145：
    /// 先 importMacros 把宏注册进当前命名空间，再执行根元素；不切换命名空间）
    pub fn include_template(&mut self, included: &Template) -> Result<()> {
        if self.include_stack.len() >= MAX_INCLUDE_DEPTH {
            return Err(TemplateError::misc(format!(
                "Maximum template include depth ({MAX_INCLUDE_DEPTH}) exceeded."
            )));
        }
        let cur_ns = self.current_ns.clone();
        for (name, def) in &included.macros {
            register_macro(&cur_ns, name, def);
        }
        let prev_name = self.current_template_name.clone();
        let prev_ns_prefixes = self.current_ns_prefixes.clone();
        self.current_template_name = included.name.clone();
        // Java include：currentNamespace 不变 → ns_prefixes 沿用主模板（不切换）
        self.include_stack.push(included.name.clone());
        let r = self.run(&included.root);
        self.include_stack.pop();
        self.current_template_name = prev_name;
        self.current_ns_prefixes = prev_ns_prefixes;
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
            *ns.ns_prefixes.borrow_mut() = t.ns_prefixes.clone();
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
        let prev_ns_prefixes = self.current_ns_prefixes.clone();
        self.current_ns = ns.clone();
        self.current_template_name = t.name.clone();
        self.current_ns_prefixes = t.ns_prefixes.clone();
        for (name, def) in &t.macros {
            register_macro(ns, name, def);
        }
        let r = self.capture(|env| env.run(&t.root));
        self.current_ns = prev_ns;
        self.current_template_name = prev_name;
        self.current_ns_prefixes = prev_ns_prefixes;
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
            ns: Rc::downgrade(ns),
        }),
    );
}

/// 校验一次写入后不会超过单个输出/捕获缓冲上限。
fn ensure_output_limit(current_len: usize, additional_len: usize) -> Result<()> {
    match current_len.checked_add(additional_len) {
        Some(total) if total <= MAX_OUTPUT_BYTES => Ok(()),
        _ => Err(TemplateError::misc(format!(
            "Template output exceeds the {MAX_OUTPUT_BYTES}-byte safety limit."
        ))),
    }
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
        // Java Environment.getVariable 不抛错（缺失变量 → null）：参数求值 lenient
        // （`f(11, null, 33)` 的 null 即缺失变量，Java checkParamsSetAndApplyDefaults
        // 对有默认值的参数回退默认值——Macro.java:273-322）
        let value = match eval::eval(env, arg_expr) {
            Ok(v) => v,
            Err(TemplateError::InvalidReference { .. }) => TModel::nothing(),
            Err(e) => return Err(e),
        };
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
            // Java newUndeclaredParamNameException（Environment.java:1105-1113）：
            // "Macro "m" has no parameter with name "b". Valid parameter names are: a"
            // （jar 实测 macro_undeclared_param 基线——含合法参数名清单）
            let valid: Vec<&str> = normal_params.iter().map(|p| p.name.as_str()).collect();
            return Err(TemplateError::misc(format!(
                "{} {} has no parameter with name {}. Valid parameter names are: {}",
                if def.is_function { "Function" } else { "Macro" },
                quote_name(&def.name),
                quote_name(arg_name),
                valid.join(", "),
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

/// 默认参数求值 —— 对应 Java `Macro.Context.checkParamsSetAndApplyDefaults`
/// （Macro.java:255-340，bytecode 实测）：
///
/// 多遍重试循环（默认值可相互引用，如 `b=c[a] a=d c={"3":"4"}`）：
/// 每遍遍历参数——已设置跳过；带默认值 → 求值：成功非 null → 绑定 +
/// somethingChanged；求值为 null → 记录首个 null 默认值表达式；抛
/// InvalidReferenceException → 记录首个 IR（Java 只 catch 这一种，其余直接上传）。
/// 遍末：有失败且本遍有赋值 → 整遍重试（失败记录重置）；无失败 → 成功；
/// 失败但无进展 → 抛 firstIR；无 firstIR → 抛默认值表达式的
/// `InvalidReferenceException.getInstance(expr, env)`（classic 兼容吞掉）。
/// 无默认值且未设置 → 循环内立即抛必需参数错误（containsKey 区分
/// "specified, but had null/missing value." 与 "not specified."）。
fn apply_macro_defaults(
    env: &mut Environment,
    frame: &Rc<MacroFrame>,
    def: &MacroDef,
) -> Result<()> {
    loop {
        let mut first_ir: Option<TemplateError> = None;
        let mut first_null_expr: Option<&Expr> = None;
        let mut has_failure = false;
        let mut something_changed = false;
        for (idx, param) in def.params.iter().enumerate() {
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
                        something_changed = true;
                    }
                    Ok(_) => {
                        // 默认值本身为 null → 记录首个 null 默认值表达式
                        // （Java bytecode :115-130；遍末无 firstIR 时按它构造 IR）
                        if !has_failure {
                            first_null_expr = Some(def_expr);
                            has_failure = true;
                        }
                    }
                    Err(e) => {
                        // Java 只 catch InvalidReferenceException 重试
                        // （bytecode Exception table: InvalidReferenceException）；
                        // 其余异常（TypeMismatch 等）直接上传
                        if !matches!(e, TemplateError::InvalidReference { .. }) {
                            return Err(e);
                        }
                        if !has_failure {
                            first_ir = Some(e);
                            has_failure = true;
                        }
                    }
                }
            } else if !env.settings.classic_compatible {
                // 必需参数（无默认值且未设置）→ 循环内立即抛（Java bytecode :176-356）；
                // localVars.containsKey 区分显式传 null 与完全未传
                // （"specified, but had null/missing value." vs "not specified."）
                let specified_but_null = frame.locals.borrow().contains_key(&param.name);
                if specified_but_null {
                    return Err(TemplateError::misc(format!(
                        "When calling {} {}, required parameter {} (parameter #{}) was specified, but had null/missing value.\n\n----\nTip: If the parameter value expression on the caller side is known to be legally null/missing, you may want to specify a default value for it with the \"!\" operator, like paramValue!defaultValue.\n----",
                        if def.is_function { "function" } else { "macro" },
                        quote_name(&def.name),
                        quote_name(&param.name),
                        idx + 1,
                    )));
                }
                return Err(TemplateError::misc(format!(
                    "When calling {} {}, required parameter {} (parameter #{}) was not specified.\n\n----\nTip: If the omission was deliberate, you may consider making the parameter optional in the macro by specifying a default value for it, like <#macro macroName paramName=defaultExpr>)\n----",
                    if def.is_function { "function" } else { "macro" },
                    quote_name(&def.name),
                    quote_name(&param.name),
                    idx + 1,
                )));
            }
        }
        if has_failure && something_changed {
            continue; // 整遍重试（Java goto 29：失败记录与 changed 全部重置）
        }
        if let Some(ir) = first_ir {
            return Err(ir);
        }
        // 无 firstIR：默认值表达式求值为 null → InvalidReferenceException.getInstance
        // （Java bytecode :398-411；blame = 默认值表达式及其位置；classic 兼容吞掉）
        if !env.settings.classic_compatible {
            if let Some(expr) = first_null_expr {
                return Err(
                    TemplateError::invalid_reference_at(expr_desc(expr), expr.span)
                        .with_location(&env.current_template_name, expr.span),
                );
            }
        }
        return Ok(());
    }
}

/// 构造 `.args` 特殊变量值 —— 对应 Java `Macro.Context.checkParamsSetAndApplyDefaults`
/// （Macro.java:344-397）：
/// - macro → SimpleHash（参数名 → 最终值，含默认值解析后；命名 catch-all 哈希展开；
///   位置 catch-all 序列非空 → "The macro can only by called with named arguments,
///   because it uses both .args and a non-empty catch-all parameter."）
/// - function → SimpleSequence（位置参数值 + 位置 catch-all 展开）
///
/// 该函数被 `BuiltinVariable.Args` 惰性调用（Java 仅在访问 `.args` 时构造）；
/// `pub(crate)` 供 eval.rs 的 `.args` 求值路径复用。
pub(crate) fn build_args_special(
    frame: &Rc<MacroFrame>,
    def: &MacroDef,
    is_function: bool,
) -> Result<TModel> {
    let normal: Vec<&MacroParam> = def.params.iter().filter(|p| !p.catch_all).collect();
    let catch_all_name = def
        .params
        .iter()
        .find(|p| p.catch_all)
        .map(|p| p.name.clone());
    let locals = frame.locals.borrow();
    let get = |name: &str| locals.get(name).cloned().unwrap_or_else(TModel::nothing);
    if is_function {
        // Java :346-370：SimpleSequence（参数值 + 位置 catch-all 展开）
        let mut vals: Vec<TModel> = normal.iter().map(|p| get(&p.name)).collect();
        if let Some(cn) = &catch_all_name {
            let catch = get(cn);
            if let Ok(seq) = catch.get_sequence() {
                for i in 0..seq.size()? {
                    vals.push(seq.get(i)?);
                }
            }
        }
        return Ok(TModel::from_sequence(vals));
    }
    // Java :374-396：SimpleHash（参数名 → 值 + 命名 catch-all 展开）
    let mut map: IndexMap<String, TModel> = IndexMap::new();
    for p in &normal {
        map.insert(p.name.clone(), get(&p.name));
    }
    if let Some(cn) = &catch_all_name {
        let catch = get(cn);
        if catch.is_sequence() {
            if catch.get_sequence()?.size()? != 0 {
                return Err(TemplateError::misc(
                    "The macro can only by called with named arguments, because it uses both .args and a non-empty catch-all parameter.",
                ));
            }
        } else if let Some(h) = &catch.hash_ex {
            // Java Macro.java:387-394：catchAllHash.keyValuePairIterator（哈希条目展开）
            for k in h.keys()? {
                if let Some(v) = h.get(&k)? {
                    map.insert(k, v);
                }
            }
        }
    }
    Ok(TModel::from_hash(map))
}

/// Java `_CoreStringUtils.jQuote` 的简化形式（错误消息用）
fn quote_name(s: &str) -> String {
    format!("\"{}\"", s)
}

/// 输出转码：内部 UTF-8 → 目标编码（ISO-8859-1 / UTF-16BE 等）
/// 对应 Java `Writer` + `OutputStreamWriter` 包装：
/// OutputStreamWriter(out, Charset.forName(outputEncoding))
fn transcode_output(utf8: &[u8], encoding_name: &str) -> Result<Vec<u8>> {
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
            "Can't convert boolean to string automatically, because the \"boolean_format\" setting was \"true,false\", which is the legacy deprecated default, and we treat it as if no format was set. This is the default configuration; you should provide the format explicitly for each place where you print a boolean.\n\n----\nTip: Write something like myBool?string('yes', 'no') to specify boolean formatting in place.\n----\nTip: If you want \"true\"/\"false\" result as you are generating computer-language output (not for direct human consumption), then use \"?c\", like ${myBool?c}. (If you always generate computer-language output, then it's might be reasonable to set the \"boolean_format\" setting to \"c\" instead.)\n----\nTip: If you need the same two values on most places, the programmers can set the \"boolean_format\" setting to something like \"yes,no\". However, then it will be easy to unwillingly format booleans like that.\n----",
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
        K::ListLit(items) => format!(
            "[{}]",
            items.iter().map(expr_desc).collect::<Vec<_>>().join(", ")
        ),
        K::BuiltIn { target, name, args } => match args {
            Some(args) => format!(
                "{}?{}({})",
                expr_desc(target),
                name,
                args.iter().map(expr_desc).collect::<Vec<_>>().join(", ")
            ),
            None => format!("{}?{}", expr_desc(target), name),
        },
        _ => "...".to_string(),
    }
}

/// 元素是否在 Java 指令栈快照中显示 —— 对应各类 `isShownInStackTrace()` 覆盖
/// （jar 实测：BodyInstruction/Include/Interpolation/LibraryLoad/UnifiedCall/
/// TransformBlock/VisitNode/RecurseNode/FallbackInstruction 返回 true，其余 false；
/// 栈顶失败帧不受限——getInstructionStackSnapshot 对末帧无条件包含）
fn element_shown_in_stack_trace(el: &Element) -> bool {
    matches!(
        el.kind,
        ElementKind::Interpolation { .. }
            | ElementKind::Call { .. }
            | ElementKind::Include { .. }
            | ElementKind::Import { .. }
            | ElementKind::Nested { .. }
            | ElementKind::Transform { .. }
            | ElementKind::Visit { .. }
            | ElementKind::Recurse { .. }
            | ElementKind::Fallback
    )
}

/// 元素描述 —— 对应 Java `TemplateElement.getDescription()`（= `dump(false)`，元素
/// 源码形式的规范化文本；`_MessageUtil.shorten(desc, 40)` 截断，Environment.java:2622）。
/// 返回 None = 无描述（Java 快照过滤跳过 description==null 的帧；Text/Comment 等
/// 不可能失败的元素不压帧）。
fn describe_element(env: &Environment, el: &Element) -> Option<String> {
    use ElementKind as E;
    let d = match &el.kind {
        E::Text { .. }
        | E::NoParse { .. }
        | E::Comment { .. }
        | E::FtlHeader { .. }
        | E::TrimLineStart
        | E::NoTrimLineStart
        | E::TrimLineEnd
        | E::LeftTrimLine
        | E::RawText(_) => return None,
        E::Interpolation {
            expr,
            legacy_min_frac,
            ..
        } => {
            // Java DollarVariable.dump：`${expr}` / `#{expr}`；自动转义/`<#escape>` 内
            // 追加 " auto-escaped"（escapedExpression != expression）
            let s = if legacy_min_frac.is_some() {
                format!("#{{{}}}", expr_desc(expr))
            } else {
                format!("${{{}}}", expr_desc(expr))
            };
            if legacy_min_frac.is_none() && (!env.escapes.is_empty() || env.auto_escape) {
                format!("{s} auto-escaped")
            } else {
                s
            }
        }
        E::If { cond, .. } => format!("#if {}", expr_desc(cond)),
        E::List { seq, var, var2, .. } => {
            let mut s = format!("#list {}", expr_desc(seq));
            if !var.is_empty() {
                s.push_str(&format!(" as {var}"));
                if let Some(v2) = var2 {
                    s.push_str(&format!(", {v2}"));
                }
            }
            s
        }
        E::Items { var, var2, .. } => {
            let mut s = format!("#items as {var}");
            if let Some(v2) = var2 {
                s.push_str(&format!(", {v2}"));
            }
            s
        }
        E::Assign {
            target, expr, op, ..
        } => format!(
            "#assign {target} {} {}",
            assign_op_symbol(*op),
            expr_desc(expr)
        ),
        E::Global {
            target,
            expr: Some(e),
            ..
        } => format!("#global {target} = {}", expr_desc(e)),
        E::Local {
            target,
            expr: Some(e),
            ..
        } => format!("#local {target} = {}", expr_desc(e)),
        E::Macro { def } => macro_def_description(def),
        E::Call {
            callee,
            args,
            body_params,
            ..
        } => {
            // Java UnifiedCall.getDescription（dump）：`@name` + 位置参数 `1, 2`
            // （逗号空格）与命名参数 `b=1` 混合（位置在前）
            let mut s = String::from("@");
            match callee {
                CallTarget::Name(n) => s.push_str(n),
                CallTarget::Namespaced { ns, name } => {
                    s.push_str(ns);
                    s.push('.');
                    s.push_str(name);
                }
                CallTarget::Expr(e) => s.push_str(&expr_desc(e)),
            }
            let mut parts: Vec<String> = Vec::new();
            for (k, e) in args {
                if k.is_empty() {
                    parts.push(expr_desc(e));
                } else {
                    parts.push(format!("{k}={}", expr_desc(e)));
                }
            }
            if !parts.is_empty() {
                s.push(' ');
                s.push_str(&parts.join(", "));
            }
            // `; a, b` 体参数（Java `@m 1; a, b` 描述含体参数——场景未覆盖，按
            // 源码形式附加）
            if !body_params.is_empty() {
                s.push_str("; ");
                s.push_str(&body_params.join(", "));
            }
            s
        }
        E::Nested { .. } => "#nested".to_string(),
        E::Switch { expr, .. } => format!("#switch {}", expr_desc(expr)),
        E::Break => "#break".to_string(),
        E::Continue => "#continue".to_string(),
        E::Return { expr } => match expr {
            Some(e) => format!("#return {}", expr_desc(e)),
            None => "#return".to_string(),
        },
        E::Stop { msg } => match msg {
            Some(e) => format!("#stop {}", expr_desc(e)),
            None => "#stop".to_string(),
        },
        E::Flush => "#flush".to_string(),
        E::Include { path, .. } => format!("#include {}", expr_desc(path)),
        E::Import { path, ns } => format!("#import {} as {ns}", expr_desc(path)),
        E::Setting { key, value } => format!("#setting {key}={}", expr_desc(value)),
        E::Transform { expr, .. } => format!("#transform {}", expr_desc(expr)),
        E::Visit { expr, .. } => match expr {
            Some(e) => format!("#visit {}", expr_desc(e)),
            None => "#visit".to_string(),
        },
        E::Recurse { expr, .. } => match expr {
            Some(e) => format!("#recurse {}", expr_desc(e)),
            None => "#recurse".to_string(),
        },
        E::On { expr, .. } => format!("#on {}", expr_desc(expr)),
        E::Fallback => "#fallback".to_string(),
        // 容器类指令（Escape/NoEscape/AutoEsc/NoAutoEsc/OutputFormat/Compress/
        // Attempt/Trim/Assignments/BlockAssign/Sep）：Java 无描述或失败帧为内部
        // 元素——不压帧
        _ => return None,
    };
    Some(shorten_java(&d, 40))
}

/// 赋值操作符符号（Java Assignment.dump 的标签文本）
fn assign_op_symbol(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Equals => "=",
        AssignOp::PlusEq => "+=",
        AssignOp::MinusEq => "-=",
        AssignOp::TimesEq => "*=",
        AssignOp::DivideEq => "/=",
        AssignOp::ModuloEq => "%=",
        AssignOp::PlusPlus => "++",
        AssignOp::MinusMinus => "--",
    }
}

/// 宏/函数定义描述 —— 对应 Java `Macro.getDescription()`（dump）：
/// `#macro {name} {param}[={default}]...` / `#function ...`（参数空格分隔，
/// 默认值按表达式描述）
fn macro_def_description(def: &MacroDef) -> String {
    let mut s = format!(
        "{} {name}",
        if def.is_function {
            "#function"
        } else {
            "#macro"
        },
        name = def.name
    );
    for p in &def.params {
        if p.catch_all {
            s.push_str(&format!(
                " {}{}",
                p.name,
                if p.default.is_some() { "..." } else { "" }
            ));
            continue;
        }
        match &p.default {
            Some(e) => s.push_str(&format!(" {}={}", p.name, expr_desc(e))),
            None => s.push_str(&format!(" {}", p.name)),
        }
    }
    s
}

/// Java `_MessageUtil.shorten(s, maxLen)`：超长截断为 `前 maxLen-3 字符 + "..."`
fn shorten_java(s: &str, max_len: usize) -> String {
    let count = s.chars().count();
    if count <= max_len {
        return s.to_string();
    }
    let cut = max_len.saturating_sub(3);
    let head: String = s.chars().take(cut).collect();
    format!("{head}...")
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
pub(crate) fn attach_location(
    err: TemplateError,
    template_name: &str,
    span: Span,
) -> TemplateError {
    // 错误已带位置（eval 包装按失败表达式位置附加）或消息已含位置 → 不重复附加
    if err.has_location() {
        return err;
    }
    match err {
        TemplateError::InvalidReference { name, ctx } => {
            if name.contains("[in template") {
                TemplateError::InvalidReference { name, ctx }
            } else {
                TemplateError::InvalidReference {
                    name,
                    ctx: Box::new(ErrorCtx {
                        span,
                        template_name: Some(template_name.to_string()),
                        ..*ctx
                    }),
                }
            }
        }
        TemplateError::TypeMismatch {
            expected,
            actual,
            ctx,
        } => {
            if actual.contains("[in template") || ctx.assignment_target.is_some() {
                // 赋值目标错误（Java UnexpectedTypeException(blamedAssignmentTargetVarName)）
                // 无 blame 表达式 → 消息不含位置（位置仅出现在 FTL stack 段）
                TemplateError::TypeMismatch {
                    expected,
                    actual,
                    ctx,
                }
            } else {
                TemplateError::TypeMismatch {
                    expected,
                    actual,
                    ctx: Box::new(ErrorCtx {
                        span,
                        template_name: Some(template_name.to_string()),
                        ..*ctx
                    }),
                }
            }
        }
        // Java _MiscTemplateException / TemplateModelException 消息不含位置
        // （位置仅由 FTL stack trace 段承载，jar 实测 div_by_zero/宏参数错误等）
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
            TemplateError::InvalidReference { name, ctx } => {
                assert!(name.contains("missing"), "{name}");
                // 位置段由 ctx（失败表达式位置）渲染：`==> missing  [in template ...]`
                assert!(
                    ctx.template_name.as_deref() == Some("err.ftl") && ctx.span.line == 1,
                    "{name} / ctx: {ctx:?}"
                );
            }
            other => panic!("expected InvalidReference, got {other:?}"),
        }
    }

    #[test]
    fn macro_namespace_is_released_after_environment_drops() {
        let (c, loader) = cfg();
        loader.put("macro.ftl", "<#macro m>ok</#macro>");
        let template = c.get_template("macro.ftl").unwrap();
        let namespace = {
            let mut out = Vec::new();
            let env = Environment::new(&template, TModel::from_hash(IndexMap::new()), &mut out);
            let weak = Rc::downgrade(&env.main_ns);
            assert!(weak.upgrade().is_some());
            weak
        };
        assert!(
            namespace.upgrade().is_none(),
            "宏值不得与命名空间形成 Rc 强引用环"
        );
    }

    #[test]
    fn recursive_include_is_stopped_at_depth_limit() {
        let (c, loader) = cfg();
        let err = render_src(
            &c,
            &loader,
            "self.ftl",
            "before<#include 'self.ftl'>",
            DynValue::Map(vec![]),
        )
        .unwrap_err();
        assert!(
            err.to_user_message()
                .contains("Maximum template include depth"),
            "{err}"
        );
    }

    #[test]
    fn recursive_macro_is_stopped_at_depth_limit() {
        let (c, loader) = cfg();
        let err = render_src(
            &c,
            &loader,
            "recursive.ftl",
            "<#macro m><@m/></#macro><@m/>",
            DynValue::Map(vec![]),
        )
        .unwrap_err();
        assert!(
            err.to_user_message()
                .contains("Maximum macro/function call depth"),
            "{err}"
        );
    }

    #[test]
    fn output_limit_rejects_overflow_without_allocating() {
        assert!(ensure_output_limit(MAX_OUTPUT_BYTES - 1, 1).is_ok());
        assert!(ensure_output_limit(MAX_OUTPUT_BYTES, 1).is_err());
        assert!(ensure_output_limit(usize::MAX, 1).is_err());
    }
}
