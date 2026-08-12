//! 模板源 —— 对应 Java `freemarker.cache.TemplateLoader` 的
//! `findTemplateSource` 返回对象（Java 中 URLTemplateSource/StringTemplateSource
//! 等；本 trait 为 Rust 侧统一抽象）

use std::any::Any;

/// 模板源（对应 findTemplateSource 返回对象）
pub trait TemplateSource {
    fn name(&self) -> String;

    /// 按具体类型还原（对应 Java 内部按 instanceof 分发，如 MultiSource 委托；
    /// 默认不参与还原，实现者按需覆写）
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试实现者：模拟 Java StringTemplateSource（不覆写 as_any）
    struct TestSource(String);
    impl TemplateSource for TestSource {
        fn name(&self) -> String {
            self.0.clone()
        }
    }

    /// 覆写 as_any 的实现者：模拟 MultiSource 委托按 instanceof 还原
    struct DowncastingSource(String);
    impl TemplateSource for DowncastingSource {
        fn name(&self) -> String {
            self.0.clone()
        }
        fn as_any(&self) -> Option<&dyn Any> {
            Some(self)
        }
    }

    #[test]
    fn default_as_any_returns_none() {
        // 默认实现者：as_any 不参与还原（Java 未覆写 instanceof 分发的源）
        let src: Box<dyn TemplateSource> = Box::new(TestSource("a.ftl".to_string()));
        assert_eq!(src.name(), "a.ftl");
        assert!(src.as_any().is_none());
    }

    #[test]
    fn overridden_as_any_downcasts() {
        // 覆写实现者：可按具体类型还原（Java MultiSource 委托语义）
        let src: Box<dyn TemplateSource> = Box::new(DowncastingSource("b.ftl".to_string()));
        let down: &DowncastingSource = src
            .as_any()
            .expect("覆写后 as_any 应返回 Some(self)")
            .downcast_ref()
            .expect("可还原为具体类型");
        assert_eq!(down.0, "b.ftl");
    }
}
