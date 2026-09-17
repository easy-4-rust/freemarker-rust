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
//!
//! 实现文件按职责拆分（`#[path]` 聚合，参照 parser/grammar.rs 模式）：
//! - environment_loop.rs —— 循环迭代上下文（LocalEntry/LoopCtx/PendingItems 等）
//! - environment_macros.rs —— 宏帧与命名空间（MacroFrame/MacroValue/Namespace 等）
//! - environment_variables.rs —— 变量解析链、输出捕获、转义栈、输出转码
//! - environment_invoke.rs —— 宏/函数调用、include/import、custom state、auto imports
//! - environment_macro_args.rs —— 宏参数绑定、默认参数求值、`.args` 构建、输出上限
//! - environment_models.rs —— 模型构造下沉、输出字符串化、日期格式分派
//! - environment_stack_trace.rs —— 指令栈快照描述、错误位置附加

use crate::core::{Element, ElementKind, Expr, Settings, TzSetting};
use crate::error::{Result, StackFrame, TemplateError};
use crate::span::Span;
use crate::template::{TModel, Template};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

#[path = "environment_invoke.rs"]
mod environment_invoke;
#[path = "environment_loop.rs"]
mod environment_loop;
#[path = "environment_macro_args.rs"]
mod environment_macro_args;
#[path = "environment_macros.rs"]
mod environment_macros;
#[path = "environment_models.rs"]
mod environment_models;
#[path = "environment_stack_trace.rs"]
mod environment_stack_trace;
#[path = "environment_variables.rs"]
mod environment_variables;

// 保持既有类型路径不变（api-baseline：公开 API diff = 0）
pub(crate) use self::environment_invoke::{register_macro, LookupOutcome, LookupResult};
pub(crate) use self::environment_loop::{
    BodyCtx, LocalEntry, LoopItem, PendingItems, RangeIterState,
};
pub(crate) use self::environment_macro_args::build_args_special;
pub use self::environment_macros::{LambdaValue, MacroFrame, MacroValue, Namespace};
pub(crate) use self::environment_macros::{WithArgs, WithArgsKind};
pub(crate) use self::environment_models::{
    boolean_format, boolean_format_strings, eval_to_string, expr_desc, format_date_value,
    model_to_string, parse_date_value,
};
pub use self::environment_models::{lambda_model, macro_model, namespace_model, vars_snapshot};
pub(crate) use self::environment_stack_trace::{
    attach_location, describe_element, element_shown_in_stack_trace,
};
pub(crate) use self::environment_variables::transcode_output;

/// 循环迭代上下文 —— 对应 Java `IteratorBlock.IterationContext`（IteratorBlock.java:190-468）
/// 提供循环变量 `x`、`x_index`、`x_has_next`，以及 `?index`/`?counter`/`?has_next` 等内建的读数
/// （docs/04 §6：循环变量作用域；fallbackOnNullLoopVariable 设置，IteratorBlock.java:368-376）。
/// 单个 #list 一个上下文（Java 模型）：`<#items>` 就地元素经 `pending` 队列驱动迭代
/// （loopForItemsElement，IteratorBlock.java:230-250）；hashListing（`as k, v`）时
/// var1=键、var2=值（getLocalVariable :452-482）。
///
/// 类型定义留在本文件：`get_loop_context` 公开签名引用本类型，定义位置决定
/// rustdoc 呈现路径（须保持 `environment::LoopCtx` 不变，api-baseline）。
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

/// 单次渲染允许的最大模板包含层数。
///
/// 防止 `<#include>` 自包含或 A → B → A 环路耗尽调用栈。该限制只约束当前
/// 包含链；同一模板在前一次包含返回后可再次包含。
pub(crate) const MAX_INCLUDE_DEPTH: usize = 16;

/// 单次渲染允许的最大宏/函数调用深度，防止无终止递归耗尽调用栈。
pub(crate) const MAX_MACRO_CALL_DEPTH: usize = 16;

/// 单次输出或捕获缓冲允许的最大字节数。
///
/// 引擎在成功结束时才写出 `output_buffer`，因此必须在写入时限制其大小，避免
/// 无界循环或异常输入把宿主进程的内存耗尽。
pub(crate) const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// 渲染入口（对应 `Template.process(rootMap, out)` → `Environment.process()`）
pub fn render(template: &Template, root: TModel, out: &mut dyn Write) -> Result<()> {
    let mut env = Environment::new(template, root, out);
    // Java Environment.process :322 doAutoImportsAndIncludes → Configuration
    // doAutoImports/doAutoIncludes（Configuration.java:3679-3748）在 process()
    // 内执行（三层 auto import/include 合并，见 do_auto_imports_and_includes）
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
    /// 词法模板名 —— 对应 Java `getCurrentTemplate()`（Environment.java:257-267：
    /// instructionStack 栈顶元素的 template 字段 = 当前词法所在模板）。与
    /// `current_template_name`（运行期当前模板，仅 include/import 切换）不同：
    /// 宏/函数体内词法模板 = **宏定义所在模板**（Java 各元素解析期绑定的 template），
    /// `<#nested>` 回插时 = 调用点模板。`.caller_template_name`
    /// （BuiltinVariable.java:264-267）与 `?absolute_template_name` 的标量基准
    /// （BuiltInsForStringsMisc.java:165-167 getTemplate().getName()）依赖它。
    pub(crate) lexical_template_name: String,
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
    // -----------------------------------------------------------------------
    // 三层 auto import/include 分层（Java Configurable 继承链：
    // Configuration → Template → Environment；Environment.java:322 process 前
    // doAutoImportsAndIncludes → Configuration.java:3679-3748）：
    // -----------------------------------------------------------------------
    /// 本环境（env）层 auto imports（Java `Environment.addAutoImport` ——
    /// Configurable.autoImports 在 env 层自有表；getAutoImportsWithoutFallback）
    pub(crate) env_auto_imports: Vec<(String, String)>,
    /// 本环境（env）层 auto includes（Java `Environment.addAutoInclude`）
    pub(crate) env_auto_includes: Vec<String>,
    /// 主模板（t）层 auto imports —— Environment 构造时从
    /// `template.template_configuration` 复制（Java：TemplateConfiguration.apply
    /// 在模板加载时把 tc.autoImports 合并进 Template 对象，
    /// TemplateConfiguration.java:399-402 + TemplateCache.java:583；
    /// t.getAutoImportsWithoutFallback()）
    pub(crate) t_auto_imports: Vec<(String, String)>,
    /// 主模板（t）层 auto includes（Java t.getAutoIncludesWithoutFallback()）
    pub(crate) t_auto_includes: Vec<String>,
    /// custom state 表 —— 对应 Java `Environment.customStateVariables`
    /// （Environment.java:3405-3446：IdentityHashMap<Object,Object>；
    /// 键 identity 语义 Rust 侧用 String；值可为 null → Option 槽位）
    pub(crate) custom_state: RefCell<HashMap<String, Option<TModel>>>,
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
        // per-template 配置（Java：Environment 构造时
        // `setTemplateConfiguration` 应用模板配置的渲染期设置；未设置项继承
        // 全局值 → Cow::Owned 覆盖副本）
        let settings_cow = match &template.template_configuration {
            Some(tc) => {
                let mut s = (*base_settings).clone();
                tc.apply_to(&mut s);
                std::borrow::Cow::Owned(s)
            }
            None => std::borrow::Cow::Borrowed(base_settings),
        };
        let base_settings = &settings_cow;
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
        // t 层 auto import/include —— Java TemplateConfiguration.apply 在模板
        // 加载时合并进 Template 对象（TemplateConfiguration.java:399-402）；
        // Rust 侧 tc 挂于 template.template_configuration，构造时复制为
        // doAutoImportsAndIncludes 的 t 层数据（Configuration.java:3691/3721
        // t.getAutoImportsWithoutFallback()）
        let (t_auto_imports, t_auto_includes) = match &template.template_configuration {
            Some(tc) => (tc.auto_imports.clone(), tc.auto_includes.clone()),
            None => (Vec::new(), Vec::new()),
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
            settings: settings_cow,
            base_time_zone,
            base_time_zone_id,
            attempt_depth: 0,
            redirect: None,
            // 预分配输出缓冲（小模板避免多次扩容拷贝；大模板按需增长）
            output_buffer: Vec::with_capacity(128),
            escapes: Vec::new(),
            auto_escape,
            current_template_name: template.name.clone(),
            lexical_template_name: template.name.clone(),
            current_ns_prefixes: template.ns_prefixes.clone(),
            recovered_errors: Vec::new(),
            number_fmt_cache: RefCell::new(None),
            instruction_stack: Vec::new(),
            stack_shown: Vec::new(),
            current_macro_name: None,
            env_auto_imports: Vec::new(),
            env_auto_includes: Vec::new(),
            t_auto_imports,
            t_auto_includes,
            custom_state: RefCell::new(HashMap::new()),
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
        // Java Environment.process :322：渲染主模板前先执行三层
        // autoImports/autoIncludes（Configuration.doAutoImportsAndIncludes）
        self.do_auto_imports_and_includes()?;
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
                let msg = crate::template::utility::html_escape(&e.to_user_message());
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
}

#[cfg(test)]
mod tests {
    use super::environment_macro_args::ensure_output_limit;
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
    use indexmap::IndexMap;
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
