//! 宏调用帧与命名空间 —— MacroFrame/MacroValue/WithArgs/LambdaValue/Namespace
//! （对应 Java `Macro.Context`（Macro.java:227）与 `Environment.Namespace`（Environment.java:3445-3500））。

use crate::error::Result;
use crate::template::{TModel, TemplateHashModel, TemplateHashModelEx};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// 宏调用帧 —— 对应 Java `Macro.Context`（Macro.java:227-250）+ invokeMacroOrFunctionCommonPart
/// （Environment.java:848-917）。`<#local>` 写入 locals；`<#nested>` 依据 call_body/body_param/
/// caller_ns/caller_local_stack 回插调用方 body。宏定义与所属命名空间经 MacroValue 传递
/// （Java Context 中的 getMacro/getLocals 对应本帧 + macro_frames 栈）。
pub struct MacroFrame {
    /// 宏参数 + `<#local>` 变量（Java `Context.localVars`；:414 setLocalVar）
    /// FNV 哈希（热路径查找）
    pub(crate) locals: RefCell<HashMap<String, TModel, crate::template::utility::FnvBuildHasher>>,
    /// 调用方 body 元素（`<@m>body</@m>`；`<#nested>` 回插，Java callPlace.getChildBuffer()）
    pub(crate) call_body: Option<Vec<crate::core::Element>>,
    /// 体参数名列表（`<@m ; a, b>`；`<#nested v1 v2>` 按位置赋给 a、b ——
    /// Java UnifiedCall.bodyParameters，BodyInstruction.Context :122-155）
    pub(crate) body_params: Vec<String>,
    /// 调用方命名空间（Java `nestedContentNamespace`；`<#nested>` 恢复用）
    pub(crate) caller_ns: Rc<Namespace>,
    /// 调用方局部上下文栈快照（Java `prevLocalContextStack`；`<#nested>` 恢复用；
    /// 调用方宏帧链由 macro_frames 栈顶自然表达，等价 Java `prevMacroContext`）
    pub(crate) caller_local_stack: Vec<super::environment_loop::LocalEntry>,
    /// 调用方词法宏名（`<#nested>` 回插调用方 body 时 current_macro_name 恢复用——
    /// 调用方 body 元素的 `in macro "m"` 定位；Java 父元素链的宏归属）
    pub(crate) caller_macro_name: Option<String>,
    /// 调用方模板名 —— 对应 Java `Macro.Context.callPlace`（Macro.java:227-250：
    /// 调用点 TemplateObject）的 `getTemplate().getName()`（BuiltinVariable.java:264-267）：
    /// 调用点所在模板的查找名；无名模板 → None（Java getName()==null →
    /// EMPTY_STRING）。`<#nested>` 回插时当前帧为调用方帧，读数自然恢复。
    pub(crate) caller_template_name: Option<String>,
    /// 调用时的词法模板名（`<#nested>` 回插期间 lexical_template_name 恢复用——
    /// 嵌套内容词法上位于调用点模板，Java getCurrentTemplate 的指令栈顶元素语义）
    pub(crate) prev_lexical_template_name: Option<String>,
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
    pub(crate) def: Rc<crate::core::MacroDef>,
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
    pub def: Rc<crate::core::MacroDef>,
    /// 宏所属命名空间（Java `macroToNamespaceLookup` :185；宏体内 currentNamespace 切换）。
    ///
    /// 必须为 `Weak`：`Namespace.macros` 已强持有 `MacroValue`，若此处再强持有
    /// `Namespace`，每个含宏的渲染环境都会形成不可释放的 `Rc` 环。
    pub ns: Weak<Namespace>,
    /// `?with_args`/`?with_args_last` 预绑定参数 —— 对应 Java `Macro.WithArgs`
    /// （Macro.java:492-510，`new Macro(that, withArgs)` 复制构造 :98-104）；
    /// None = 无预绑定（普通宏/函数值）
    pub(crate) with_args: Option<WithArgs>,
}

/// `?with_args` 预绑定参数 —— 对应 Java `Macro.WithArgs`（Macro.java:492-510）：
/// 按名（TemplateHashModelEx）或按位（TemplateSequenceModel）二选一 + 顺序标志。
#[derive(Clone)]
pub(crate) struct WithArgs {
    pub kind: WithArgsKind,
    pub order_last: bool,
}

/// 预绑定参数的形态（Java WithArgs.byName / byPosition 二选一）
#[derive(Clone)]
pub(crate) enum WithArgsKind {
    /// 按名绑定（Java byName TemplateHashModelEx）
    ByName(indexmap::IndexMap<String, TModel>),
    /// 按位绑定（Java byPosition TemplateSequenceModel）
    ByPosition(Vec<TModel>),
}

/// lambda 值 —— 对应 Java `LocalLambdaExpression` 求值结果（v1 仅存槽位；
/// `?map`/`?filter` 等消费方由内建函数智能体扩展，docs/04 §5）
pub struct LambdaValue {
    /// 参数名列表（Java LambdaParameterList；多参数 lambda 自 2.3.32 起）
    #[allow(dead_code)]
    pub params: Vec<String>,
    #[allow(dead_code)]
    pub body: Rc<crate::core::Expr>,
}

/// 命名空间 —— 对应 Java `Environment.Namespace`（Environment.java:3445-3500，extends SimpleHash）
/// 变量与宏同表（Java Namespace 是 SimpleHash，宏以 `Macro` 对象存入）；
/// 同时实现 TemplateHashModel/Ex → 可作 TModel 值（`<@ns.macro>`、`ns.var`、`?keys` 等）。
/// 变量/宏表用 FNV 哈希（热路径查找；迭代序无依赖——keys() 已排序）。
pub struct Namespace {
    vars: RefCell<HashMap<String, TModel, crate::template::utility::FnvBuildHasher>>,
    macros: RefCell<HashMap<String, Rc<MacroValue>, crate::template::utility::FnvBuildHasher>>,
    /// 关联模板名（Java `Namespace.getTemplate()` :3470-3478；错误定位/相对 include 基名）
    /// Rc<str>：主/全局命名空间共享同一份（Environment::new 构造期 1 次分配）
    template_name: Rc<str>,
    /// 所属模板的 ns_prefixes（`<#ftl ns_prefixes=...>`；宏体内 currentNamespace
    /// 切换时随之切换——Java currentNamespace.getTemplate().getNamespaceForPrefix）
    pub(crate) ns_prefixes: RefCell<HashMap<String, String>>,
}

impl Namespace {
    pub(crate) fn new(template_name: String) -> Self {
        Namespace {
            vars: RefCell::new(HashMap::with_hasher(
                crate::template::utility::FnvBuildHasher::default(),
            )),
            macros: RefCell::new(HashMap::with_hasher(
                crate::template::utility::FnvBuildHasher::default(),
            )),
            template_name: Rc::from(template_name),
            ns_prefixes: RefCell::new(HashMap::new()),
        }
    }

    /// 共享模板名构造（主/全局命名空间复用同一 Rc<str>）
    pub(crate) fn new_shared(template_name: Rc<str>) -> Self {
        Namespace {
            vars: RefCell::new(HashMap::with_hasher(
                crate::template::utility::FnvBuildHasher::default(),
            )),
            macros: RefCell::new(HashMap::with_hasher(
                crate::template::utility::FnvBuildHasher::default(),
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
            .map(|mv| super::environment_models::macro_model(mv.clone()))
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
