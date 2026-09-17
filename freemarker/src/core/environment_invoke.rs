//! 宏/函数调用、访问节点栈、include/import 库加载、custom state 与三层 auto
//! import/include（Java invokeMacro :848-917 / include :3126-3145 / importLib :3232-3290 /
//! setCustomState :3405-3446 / doAutoImportsAndIncludes → Configuration.java:3679-3748）。

use super::environment_macro_args::{apply_macro_defaults, bind_macro_args};
use super::environment_macros::{MacroFrame, MacroValue, Namespace};
use super::environment_stack_trace::macro_def_description;
use super::Environment;
use crate::cache::{NameFormatDefault020300, TemplateNameFormat};
use crate::core::{Element, MacroDef};
use crate::error::{Result, StackFrame, TemplateError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

impl<'a> Environment<'a> {
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
    ) -> Result<super::RunSignal> {
        self.invoke_macro_common(mv, args, body, body_params, false)
    }

    /// 函数调用（Java invokeFunction :832-847：输出丢弃到 NullWriter；无 `<#return>` → nothing）
    pub(crate) fn invoke_function(
        &mut self,
        mv: &MacroValue,
        args: &[(String, crate::core::Expr)],
    ) -> Result<crate::template::TModel> {
        let r = self.invoke_macro_common(mv, args, None, Vec::new(), true)?;
        match r {
            super::RunSignal::Returned(v) => Ok(v.unwrap_or_else(crate::template::TModel::nothing)),
            super::RunSignal::Completed => Ok(crate::template::TModel::nothing()),
        }
    }

    fn invoke_macro_common(
        &mut self,
        mv: &MacroValue,
        args: &[(String, crate::core::Expr)],
        body: Option<Vec<Element>>,
        body_params: Vec<String>,
        is_function: bool,
    ) -> Result<super::RunSignal> {
        if self.macro_frames.len() >= super::MAX_MACRO_CALL_DEPTH {
            return Err(TemplateError::misc(format!(
                "Maximum macro/function call depth ({}) exceeded.",
                super::MAX_MACRO_CALL_DEPTH
            )));
        }
        // Java :848-879：宏帧 + 参数绑定（求值发生在调用方上下文）
        let frame = Rc::new(MacroFrame {
            locals: RefCell::new(HashMap::with_hasher(
                crate::template::utility::FnvBuildHasher::default(),
            )),
            call_body: body,
            body_params,
            caller_ns: self.current_ns.clone(),
            caller_local_stack: self.local_stack.clone(),
            caller_macro_name: self.current_macro_name.clone(),
            // Java Macro.Context.callPlace（调用点 TemplateObject）的模板名
            // （BuiltinVariable.java:264-267 getRequiredMacroContext(env).callPlace
            // → callPlace.getTemplate().getName()）
            caller_template_name: Some(self.lexical_template_name.clone()),
            prev_lexical_template_name: Some(self.lexical_template_name.clone()),
            args_value: RefCell::new(None),
            def: mv.def.clone(),
            is_function,
        });
        // 无参数宏：跳过参数绑定（空循环开销——热路径 `<@m/>` 调用）
        if !mv.def.params.is_empty() || mv.with_args.is_some() {
            bind_macro_args(self, &frame, &mv.def, args, &mv.with_args)?;
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
        // 词法模板名切换为宏定义所在模板（Java getCurrentTemplate：指令栈顶元素
        // 的 template —— 宏体元素在定义模板中；`.caller_template_name` 的调用点
        // 判定依赖切换前的值，已记录于帧）
        let prev_lexical = std::mem::replace(
            &mut self.lexical_template_name,
            mv.def.template_name.clone(),
        );
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
        self.lexical_template_name = prev_lexical;
        self.macro_frames.pop();
        self.pop_instruction_frame();
        self.current_macro_name = prev_macro_name;
        r
    }

    /// 宏/函数体执行（invoke_macro_common 已压宏帧并切换上下文后调用）：
    /// 默认参数求值 + 宏体/函数体 run；`<#return>` 归属判定（Java Macro.invoke 的
    /// catch(Return)——穿透的 return 继续上传）
    fn run_macro_body(
        &mut self,
        frame: Rc<MacroFrame>,
        is_function: bool,
    ) -> Result<super::RunSignal> {
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
            if let super::RunSignal::Returned(_) = &sig {
                if self.return_depth == Some(self.macro_frames.len()) {
                    self.return_depth = None;
                }
            }
            sig
        } else {
            let sig = self.run(&frame.def.body)?;
            // Java Macro.invoke：宏边界捕获归属本帧的 return（宏不能 return 值 →
            // 值恒 None，捕获即宏正常完成）；穿透的 return 继续上传
            if let super::RunSignal::Returned(_) = &sig {
                if self.return_depth == Some(self.macro_frames.len()) {
                    self.return_depth = None;
                    super::RunSignal::Completed
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
    pub(crate) fn push_visitor_node(&mut self, node: crate::template::TModel) {
        self.visit_stack.push(node);
    }

    /// 弹出访问节点（Java `visitStack.pop`；`<#visit>` 分派完成后）
    pub(crate) fn pop_visitor_node(&mut self) {
        self.visit_stack.pop();
    }

    /// 当前访问节点（Java `getCurrentVisitorNode` :2931-2933；非 visit 上下文 → None）
    pub(crate) fn get_current_visitor_node(&self) -> Option<crate::template::TModel> {
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
        match self.lookup_template(&full, parse, encoding.as_deref())? {
            LookupOutcome::Found(_, LookupResult::Parsed(t)) => self.include_template(&t),
            LookupOutcome::Found(_, LookupResult::PlainText(text)) => self.emit(&text),
            LookupOutcome::Missing(err) => {
                if ignore_missing {
                    return Ok(());
                }
                // Java Include.accept（Include.java:73-90）：加载失败（模板缺失/被包含
                // 模板解析错误）→ "Template inclusion failed (for parameter value
                // \"{name}\"):\n{原因}"（jar 实测 include_not_found / include_parse_error
                // 基线；被包含模板体内部的渲染错误不加此包装——include_template 路径）
                Err(TemplateError::misc(format!(
                    "Template inclusion failed (for parameter value \"{name}\"):\n{}",
                    err.to_user_message()
                )))
            }
        }
    }

    /// 模板查找（Java `getTemplateForInclusion` 的加载部分：TemplateCache.java:914-948
    /// lookupWithLocalizedThenAcquisitionStrategy —— locale 变体（en_US → en → 无后缀）
    /// 外层 × acquisition 候选内层；parse=false → 直接读源文本（getPlainTextTemplate）；
    /// 全部候选失败 → Missing（携带最后错误；调用方决定静默跳过或报错）
    pub(crate) fn lookup_template(
        &mut self,
        full: &str,
        parse: bool,
        encoding: Option<&str>,
    ) -> Result<LookupOutcome> {
        let locale = self.settings.locale.clone();
        let locale_cands: Vec<String> = if locale.is_empty() {
            vec![full.to_string()]
        } else {
            crate::template::configuration::localized_candidates(full, &locale)
        };
        let mut last_err: Option<TemplateError> = None;
        for lc in &locale_cands {
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
                        .read_encoded(&*src, encoding.unwrap_or("UTF-8"))?;
                    return Ok(LookupOutcome::Found(acq, LookupResult::PlainText(text)));
                }
                match self
                    .template
                    .configuration
                    .get_template_encoded(&acq, encoding)
                {
                    Ok(t) => return Ok(LookupOutcome::Found(acq, LookupResult::Parsed(t))),
                    Err(e) => last_err = Some(e),
                }
            }
        }
        Ok(LookupOutcome::Missing(last_err.unwrap_or(
            TemplateError::NotFound {
                name: full.to_string(),
            },
        )))
    }

    /// 执行被包含模板（Java include(includedTemplate) :3126-3145：
    /// 先 importMacros 把宏注册进当前命名空间，再执行根元素；不切换命名空间）
    pub fn include_template(&mut self, included: &crate::template::Template) -> Result<()> {
        if self.include_stack.len() >= super::MAX_INCLUDE_DEPTH {
            return Err(TemplateError::misc(format!(
                "Maximum template include depth ({}) exceeded.",
                super::MAX_INCLUDE_DEPTH
            )));
        }
        let cur_ns = self.current_ns.clone();
        for (name, def) in &included.macros {
            register_macro(&cur_ns, name, def);
        }
        let prev_name = self.current_template_name.clone();
        let prev_lexical = self.lexical_template_name.clone();
        let prev_ns_prefixes = self.current_ns_prefixes.clone();
        self.current_template_name = included.name.clone();
        self.lexical_template_name = included.name.clone();
        // Java include：currentNamespace 不变 → ns_prefixes 沿用主模板（不切换）
        self.include_stack.push(included.name.clone());
        let r = self.run(&included.root);
        self.include_stack.pop();
        self.current_template_name = prev_name;
        self.lexical_template_name = prev_lexical;
        self.current_ns_prefixes = prev_ns_prefixes;
        match r {
            Ok(super::RunSignal::Completed) => Ok(()),
            Ok(super::RunSignal::Returned(_)) => Err(TemplateError::misc(
                "<#return> is illegal in an included template",
            )),
            Err(e) => Err(e),
        }
    }

    /// `<#import path as ns>`（Java LibraryLoad.accept → importLib :3232-3290）。
    /// 惰性开关取 lazyImports 设置（Java importLib(String,String) :3168-3170 →
    /// importLib(name, ns, getLazyImports())）。
    pub fn import_lib(&mut self, path: &str, ns_var: &str) -> Result<()> {
        let lazy = self.settings.lazy_imports;
        self.import_lib_explicit(path, ns_var, lazy)
    }

    /// 指定惰性与否的 import —— 对应 Java `importLib(String, String, boolean)`
    /// （Environment.java:3194-3207）：lazy → 只建 LazilyInitializedNamespace
    /// 占位并绑定变量，不查模板；eager → 立即 getTemplateForImporting +
    /// initializeImportLibNamespace。
    pub fn import_lib_explicit(&mut self, path: &str, ns_var: &str, lazy: bool) -> Result<()> {
        // Java importLib（:3232-3290）：toFullTemplateName 后按模板名格式规范化
        // （"/import_lib.ftl" 与 "import_lib.ftl" 是同一模板——loadedLibs 缓存键一致）
        let resolved = self.resolve_template_name(path);
        let full = NameFormatDefault020300
            .normalize_root_based_name(&resolved)
            .unwrap_or(resolved);
        let ns = if let Some(existing) = self.loaded_libs.get(&full) {
            existing.clone()
        } else if lazy {
            // Java :3202-3206 lazy 分支：不触发模板查找（TemplateLookupStrategy
            // 可能昂贵），只建 LazilyInitializedNamespace 占位（Environment.java
            // :3501-3513；快照 locale/encoding）。v1 占位后不加载——首次访问才
            // ensureInitialized 的按需初始化需 env 回引，登记文档化偏差
            // （include_and_import_configurable_layers_test.rs 头注）。
            let ns = Rc::new(Namespace::new(full.clone()));
            self.loaded_libs.insert(full, ns.clone());
            ns
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
        self.set_variable(
            ns_var,
            super::environment_models::namespace_model(ns.clone()),
        );
        if Rc::ptr_eq(&self.current_ns, &self.main_ns) {
            self.global_ns.put_var(
                ns_var.to_string(),
                super::environment_models::namespace_model(ns),
            );
        }
        Ok(())
    }

    /// `<#import>` 已加载模板变体（Java `importLib(loadedTemplate, null)` ——
    /// GetOptionalTemplateMethod 的 `import` 方法调用；Java :3234-3250 注释：
    /// 缓存键用模板查找名（template.getName），与 import_lib 的根基准归一化名一致；
    /// 已缓存 → 直接返回现有命名空间，不重复初始化）
    pub(crate) fn import_lib_loaded(
        &mut self,
        key: &str,
        found: &LookupResult,
    ) -> Result<crate::template::TModel> {
        if let Some(existing) = self.loaded_libs.get(key) {
            return Ok(super::environment_models::namespace_model(existing.clone()));
        }
        let ns = match found {
            LookupResult::Parsed(t) => {
                let ns = Rc::new(Namespace::new(t.name.clone()));
                *ns.ns_prefixes.borrow_mut() = t.ns_prefixes.clone();
                self.loaded_libs.insert(key.to_string(), ns.clone());
                // Java initializeImportLibNamespace（importLib :3280-3293）：命名空间
                // 切换 + NullWriter 输出丢弃执行（Rust capture 丢弃）
                self.initialize_import_lib_namespace(&ns, t)?;
                ns
            }
            // plain text 模板无宏可注册：Java include(plainTemplate) 在 NullWriter 下
            // 输出被丢弃，等价于仅创建空命名空间
            LookupResult::PlainText(_) => {
                let ns = Rc::new(Namespace::new(key.to_string()));
                self.loaded_libs.insert(key.to_string(), ns.clone());
                ns
            }
        };
        Ok(super::environment_models::namespace_model(ns))
    }

    fn initialize_import_lib_namespace(
        &mut self,
        ns: &Rc<Namespace>,
        t: &crate::template::Template,
    ) -> Result<()> {
        let prev_ns = self.current_ns.clone();
        let prev_name = self.current_template_name.clone();
        let prev_lexical = self.lexical_template_name.clone();
        let prev_ns_prefixes = self.current_ns_prefixes.clone();
        self.current_ns = ns.clone();
        self.current_template_name = t.name.clone();
        self.lexical_template_name = t.name.clone();
        self.current_ns_prefixes = t.ns_prefixes.clone();
        for (name, def) in &t.macros {
            register_macro(ns, name, def);
        }
        let r = self.capture(|env| env.run(&t.root));
        self.current_ns = prev_ns;
        self.current_template_name = prev_name;
        self.lexical_template_name = prev_lexical;
        self.current_ns_prefixes = prev_ns_prefixes;
        match r {
            Ok((super::RunSignal::Completed, _)) => Ok(()),
            Ok((super::RunSignal::Returned(_), _)) => Err(TemplateError::misc(
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

    // ---------------------------------------------------------------------
    // custom state 与三层 auto import/include 分层
    // （Java Environment.setCustomState/getCustomState :3405-3446；
    //  doAutoImportsAndIncludes → Configuration.java:3679-3748）
    // ---------------------------------------------------------------------

    /// 读取 custom state —— 对应 Java `Environment.getCustomState(Object)`
    /// （Environment.java:3413-3420：表未建或键缺失或存 null → null）
    pub fn get_custom_state(&self, key: &str) -> Option<crate::template::TModel> {
        self.custom_state.borrow().get(key).and_then(|v| v.clone())
    }

    /// 写入 custom state —— 对应 Java `Environment.setCustomState(Object, Object)`
    /// （Environment.java:3436-3446：返回旧值（缺失/null 均 null）；值可为 null——
    /// 模板数字/日期格式工厂等可插拔对象的 Environment 级状态（如缓存）。
    /// Java 键按对象 identity，Rust 侧键为 String）
    pub fn set_custom_state(
        &mut self,
        key: &str,
        value: Option<crate::template::TModel>,
    ) -> Option<crate::template::TModel> {
        // 旧值为 Option<Option<TModel>>：键缺失 → None；键存在 → Some(旧存值)。
        // flatten 后：缺失或旧存 null → None（Java put 返回 null），等价
        // getCustomState 的 null-for-both 语义
        self.custom_state
            .borrow_mut()
            .insert(key.to_string(), value)
            .flatten()
    }

    /// 环境层添加 auto import —— 对应 Java `Environment.addAutoImport(String, String)`
    /// （Configurable.java:1944-1960：同名先移除再追加——移到插入序末尾）
    pub fn add_auto_import(&mut self, namespace_var_name: &str, template_name: &str) {
        self.env_auto_imports
            .retain(|(n, _)| n != namespace_var_name);
        self.env_auto_imports
            .push((namespace_var_name.to_string(), template_name.to_string()));
    }

    /// 环境层移除 auto import —— 对应 Java `Environment.removeAutoImport`
    /// （Configurable.java:1966-1974）
    pub fn remove_auto_import(&mut self, namespace_var_name: &str) {
        self.env_auto_imports
            .retain(|(n, _)| n != namespace_var_name);
    }

    /// 环境层添加 auto include —— 对应 Java `Environment.addAutoInclude(String)`
    /// （Configurable.java:2083-2096 → :2098-2112：同层去重——先移除再追加）
    pub fn add_auto_include(&mut self, template_name: &str) {
        self.env_auto_includes.retain(|n| n != template_name);
        self.env_auto_includes.push(template_name.to_string());
    }

    /// 环境层移除 auto include —— 对应 Java `Environment.removeAutoInclude`
    /// （Configurable.java:2175-2186）
    pub fn remove_auto_include(&mut self, template_name: &str) {
        self.env_auto_includes.retain(|n| n != template_name);
    }

    /// 设置 lazyImports —— 对应 Java `Environment.setLazyImports(boolean)`
    /// （Configurable.java:1882-1889；`<#import>` 指令与 lazyAutoImports 未设置时
    /// auto imports 的惰性开关）
    pub fn set_lazy_imports(&mut self, lazy: bool) {
        self.settings.to_mut().lazy_imports = lazy;
    }

    /// 设置 lazyAutoImports —— 对应 Java `Environment.setLazyAutoImports(Boolean)`
    /// （Configurable.java:1912-1920；None = 未设置 → 回退 lazyImports）
    pub fn set_lazy_auto_imports(&mut self, lazy: Option<bool>) {
        self.settings.to_mut().lazy_auto_imports = lazy;
    }

    /// lazyImports 读数 —— 对应 Java `Environment.getLazyImports()`
    /// （Configurable.java:1852-1854）
    pub fn get_lazy_imports(&self) -> bool {
        self.settings.lazy_imports
    }

    /// lazyAutoImports 读数 —— 对应 Java `Environment.getLazyAutoImports()`
    /// （Configurable.java:1900-1904；None = 未设置）
    pub fn get_lazy_auto_imports(&self) -> Option<bool> {
        self.settings.lazy_auto_imports
    }

    /// 渲染前执行三层 auto imports/includes —— 对应 Java
    /// `Configuration.doAutoImportsAndIncludes(env)`（Configuration.java:3679-3748；
    /// Environment.process :322 调用）：cfg 层（父）→ t 层（主模板）→ env 层（子），
    /// 低层同名被高层覆盖（cfg 层跳过 t/env 已有名字，t 层跳过 env 已有名字）。
    /// auto imports 的惰性开关：
    /// `env.getLazyAutoImports() ?? env.getLazyImports()`（Configuration.java:3690-3692；
    /// Java 的三级回退链 env → template → cfg 以 Environment::new 合并后的
    /// settings 等价）。
    pub fn do_auto_imports_and_includes(&mut self) -> Result<()> {
        let cfg_auto_imports = self.template.configuration.auto_imports.clone();
        let cfg_auto_includes = self.template.configuration.auto_includes.clone();
        let t_auto_imports = self.t_auto_imports.clone();
        let t_auto_includes = self.t_auto_includes.clone();
        let env_auto_imports = self.env_auto_imports.clone();
        let env_auto_includes = self.env_auto_includes.clone();
        let lazy_auto = self
            .settings
            .lazy_auto_imports
            .unwrap_or(self.settings.lazy_imports);
        // doAutoImports（Configuration.java:3687-3713）：按层 importLib，
        // 低层跳过高层已有 ns 名
        for (ns, path) in &cfg_auto_imports {
            if !t_auto_imports.iter().any(|(n, _)| n == ns)
                && !env_auto_imports.iter().any(|(n, _)| n == ns)
            {
                self.import_lib_explicit(path, ns, lazy_auto)?;
            }
        }
        for (ns, path) in &t_auto_imports {
            if !env_auto_imports.iter().any(|(n, _)| n == ns) {
                self.import_lib_explicit(path, ns, lazy_auto)?;
            }
        }
        for (ns, path) in &env_auto_imports {
            self.import_lib_explicit(path, ns, lazy_auto)?;
        }
        // doAutoIncludes（Configuration.java:3715-3742）：按层 include
        // （env.include(getTemplate(templateName, env.getLocale()))，:3736-3741）
        let locale = self.settings.locale.clone();
        for name in &cfg_auto_includes {
            if !t_auto_includes.iter().any(|n| n == name)
                && !env_auto_includes.iter().any(|n| n == name)
            {
                let t = self
                    .template
                    .configuration
                    .get_template_localized(name, Some(&locale))?;
                self.include_template(&t)?;
            }
        }
        for name in &t_auto_includes {
            if !env_auto_includes.iter().any(|n| n == name) {
                let t = self
                    .template
                    .configuration
                    .get_template_localized(name, Some(&locale))?;
                self.include_template(&t)?;
            }
        }
        for name in &env_auto_includes {
            let t = self
                .template
                .configuration
                .get_template_localized(name, Some(&locale))?;
            self.include_template(&t)?;
        }
        Ok(())
    }
}

/// 模板查找产物（Java `getTemplateForInclusion` 的两种结果：parseAsFTL=true →
/// 解析的 Template；false → 直接读源文本的 plain text Template，TemplateCache
/// loadTemplate :564-580 的 StringWriter 分支）
#[derive(Clone)]
pub(crate) enum LookupResult {
    Parsed(Rc<crate::template::Template>),
    PlainText(String),
}

/// 模板查找结果（对应 Java `getTemplateForInclusion` 的 ignoreMissing 语义：
/// 缺失时由调用方决定静默跳过，或按携带的 last_err / NotFound 报错）
pub(crate) enum LookupOutcome {
    Found(String, LookupResult),
    Missing(TemplateError),
}

/// 注册宏（Java visitMacroDef :1164-1167：currentNamespace.put(macroName, macro)）
pub(crate) fn register_macro(ns: &Rc<Namespace>, name: &str, def: &MacroDef) {
    ns.put_macro(
        name.to_string(),
        Rc::new(MacroValue {
            def: Rc::new(def.clone()),
            ns: Rc::downgrade(ns),
            with_args: None,
        }),
    );
}

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
