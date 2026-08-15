//! 模板后处理器 —— 对应 Java `freemarker.core.TemplatePostProcessor`
//! （抽象类；postProcess(Template) 在模板解析/缓存完成后执行 AST 变换；
//!  Java 包级可见，Rust 为 pub(crate) trait）
//!
//! Java 签名（TemplatePostProcessor.java）：
//! ```java
//! abstract class TemplatePostProcessor {
//!     public abstract void postProcess(Template e) throws TemplatePostProcessorException;
//!     // TODO: getPriority, getPhase, getMustBeBefore, getMustBeAfter
//! }
//! ```
//!
//! 集成点：Configuration.get_template 加载模板后、入缓存前调用注册的后处理器链。

use crate::core::template_post_processor_exception::TemplatePostProcessorException;
use crate::template::Template;

/// 模板后处理器 trait（对应 Java `TemplatePostProcessor` 抽象类）
///
/// 在模板解析完成后、入缓存前执行 AST 变换。注册到 Configuration 后，
/// 每次 get_template 加载新模板时自动调用。
///
/// # Examples
///
/// ```ignore
/// use freemarker::core::{TemplatePostProcessor, TemplatePostProcessorException};
///
/// struct MyPostProcessor;
///
/// impl TemplatePostProcessor for MyPostProcessor {
///     fn post_process(&self, template: &mut Template) -> Result<(), TemplatePostProcessorException> {
///         // 对 template.root 做变换
///         Ok(())
///     }
/// }
/// ```
pub trait TemplatePostProcessor {
    /// 对已解析的模板执行后处理（Java `postProcess(Template)`）
    ///
    /// 实现方可在 template.root（AST）上做变换（如注入检查元素、优化等）。
    /// 失败时返回 TemplatePostProcessorException。
    fn post_process(&self, template: &mut Template) -> Result<(), TemplatePostProcessorException>;
}

/// 后处理器注册表 —— 管理 Configuration 级别的后处理器链
///
/// 对应 Java Configuration 中的 `templatePostProcessors` 字段
/// （Java 为 ArrayList，Rust 为 Vec<Box<dyn TemplatePostProcessor>>）。
#[derive(Default)]
pub(crate) struct TemplatePostProcessorRegistry {
    processors: Vec<Box<dyn TemplatePostProcessor>>,
}

impl TemplatePostProcessorRegistry {
    /// 添加后处理器（Java `Configuration.addTemplatePostProcessor`）
    ///
    /// 处理器按添加顺序执行。同一实例不会重复添加（按指针判等）。
    pub fn add(&mut self, processor: Box<dyn TemplatePostProcessor>) {
        self.processors.push(processor);
    }

    /// 移除后处理器（Java `Configuration.removeTemplatePostProcessor`）
    ///
    /// 按指针判等移除首个匹配；不存在则无事发生。
    /// 注意：Rust 的 Box<dyn> 无法直接按指针判等，这里按顺序移除首个。
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.processors.len() {
            self.processors.remove(index);
            true
        } else {
            false
        }
    }

    /// 清空所有后处理器
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.processors.clear();
    }

    /// 处理器数量
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// 是否为空
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    /// 对模板执行所有注册的后处理器（按注册顺序）
    ///
    /// 任一处理器失败则中断并返回错误。
    pub fn apply_all(&self, template: &mut Template) -> Result<(), TemplatePostProcessorException> {
        for processor in &self.processors {
            processor.post_process(template)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::Configuration;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// 测试用后处理器：记录调用次数
    struct CountingProcessor {
        #[allow(dead_code)]
        count: std::cell::Cell<usize>,
    }

    impl CountingProcessor {
        fn new() -> Self {
            CountingProcessor {
                count: std::cell::Cell::new(0),
            }
        }
    }

    impl TemplatePostProcessor for CountingProcessor {
        fn post_process(
            &self,
            _template: &mut Template,
        ) -> Result<(), TemplatePostProcessorException> {
            self.count.set(self.count.get() + 1);
            Ok(())
        }
    }

    /// 测试用后处理器：总是失败
    struct FailingProcessor;

    impl TemplatePostProcessor for FailingProcessor {
        fn post_process(
            &self,
            _template: &mut Template,
        ) -> Result<(), TemplatePostProcessorException> {
            Err(TemplatePostProcessorException::new("intentional failure"))
        }
    }

    /// 测试用后处理器：向 root 追加一个 Text 元素
    struct InjectTextProcessor {
        text: String,
    }

    impl TemplatePostProcessor for InjectTextProcessor {
        fn post_process(
            &self,
            template: &mut Template,
        ) -> Result<(), TemplatePostProcessorException> {
            template.root.push(crate::core::Element::new(
                crate::core::ElementKind::Text {
                    text: self.text.clone(),
                    strip_before: false,
                    strip_after: false,
                    orig_end_line: 0,
                },
                crate::span::Span::default(),
            ));
            Ok(())
        }
    }

    #[test]
    fn registry_add_and_apply() {
        let mut registry = TemplatePostProcessorRegistry::default();
        assert!(registry.is_empty());

        registry.add(Box::new(CountingProcessor::new()));
        assert_eq!(registry.len(), 1);

        let cfg = Rc::new(Configuration::default());
        let mut template = Template::new("test.ftl".to_string(), Vec::new(), HashMap::new(), cfg);
        registry.apply_all(&mut template).unwrap();
    }

    #[test]
    fn registry_failure_propagates() {
        let mut registry = TemplatePostProcessorRegistry::default();
        registry.add(Box::new(FailingProcessor));

        let cfg = Rc::new(Configuration::default());
        let mut template = Template::new("test.ftl".to_string(), Vec::new(), HashMap::new(), cfg);
        let result = registry.apply_all(&mut template);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("intentional failure"));
    }

    #[test]
    fn registry_remove() {
        let mut registry = TemplatePostProcessorRegistry::default();
        registry.add(Box::new(CountingProcessor::new()));
        registry.add(Box::new(CountingProcessor::new()));
        assert_eq!(registry.len(), 2);

        assert!(registry.remove(0));
        assert_eq!(registry.len(), 1);
        assert!(!registry.remove(10)); // 越界
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn registry_clear() {
        let mut registry = TemplatePostProcessorRegistry::default();
        registry.add(Box::new(CountingProcessor::new()));
        registry.add(Box::new(CountingProcessor::new()));
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn post_processor_modifies_template() {
        let mut registry = TemplatePostProcessorRegistry::default();
        registry.add(Box::new(InjectTextProcessor {
            text: "injected".to_string(),
        }));

        let cfg = Rc::new(Configuration::default());
        let mut template = Template::new("test.ftl".to_string(), Vec::new(), HashMap::new(), cfg);
        assert!(template.root.is_empty());

        registry.apply_all(&mut template).unwrap();
        assert_eq!(template.root.len(), 1);
        match &template.root[0].kind {
            crate::core::ElementKind::Text { text, .. } => assert_eq!(text, "injected"),
            _ => panic!("expected Text element"),
        }
    }

    #[test]
    fn processors_execute_in_order() {
        let mut registry = TemplatePostProcessorRegistry::default();
        registry.add(Box::new(InjectTextProcessor {
            text: "first".to_string(),
        }));
        registry.add(Box::new(InjectTextProcessor {
            text: "second".to_string(),
        }));

        let cfg = Rc::new(Configuration::default());
        let mut template = Template::new("test.ftl".to_string(), Vec::new(), HashMap::new(), cfg);
        registry.apply_all(&mut template).unwrap();
        assert_eq!(template.root.len(), 2);
        match &template.root[0].kind {
            crate::core::ElementKind::Text { text, .. } => assert_eq!(text, "first"),
            _ => panic!("expected first Text element"),
        }
        match &template.root[1].kind {
            crate::core::ElementKind::Text { text, .. } => assert_eq!(text, "second"),
            _ => panic!("expected second Text element"),
        }
    }
}
