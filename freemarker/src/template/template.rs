//! Template —— 对应 Java `freemarker.template.Template`
//! （解析产物：根元素树 + 宏表；process 入口见 docs/02 §2.2）

use crate::core::Element;
use crate::core::MacroDef;
use crate::error::Result;
use crate::template::Configuration;
use crate::template::TModel;
use std::collections::HashMap;
use std::rc::Rc;

/// 解析产物（对应 `freemarker.template.Template`）
pub struct Template {
    pub name: String,
    pub root: Vec<Element>,
    pub macros: HashMap<String, MacroDef>,
    pub configuration: Rc<Configuration>,
    pub encoding: Option<String>,
    /// `<#ftl ns_prefixes=...>` 命名空间前缀映射（prefix → URI；XML 节点查询用）
    pub ns_prefixes: HashMap<String, String>,
    /// per-template 配置（Java `Template.getTemplateConfiguration`：
    /// Configuration.setTemplateConfigurations 匹配结果；渲染时应用到
    /// Environment 设置）
    pub(crate) template_configuration: Option<crate::core::TemplateConfiguration>,
}

impl Template {
    pub fn new(
        name: String,
        root: Vec<Element>,
        macros: HashMap<String, MacroDef>,
        configuration: Rc<Configuration>,
    ) -> Self {
        Template {
            name,
            root,
            macros,
            configuration,
            encoding: None,
            ns_prefixes: HashMap::new(),
            template_configuration: None,
        }
    }

    /// 对应 `Template.process(rootMap, out)`
    pub fn process(&self, root: TModel, out: &mut dyn std::io::Write) -> Result<()> {
        crate::core::render(self, root, out)
    }
}
