//! 鲁棒性 fuzz：解析器/渲染对随机输入不得 panic（阶段 A8，proptest）。

use std::rc::Rc;

use proptest::prelude::*;
use indexmap::IndexMap;

use freemarker::parser::parse;
use freemarker::template::Configuration;
use freemarker::template::SimpleHash;
use freemarker::template::SimpleScalar;
use freemarker::template::TModel;

fn fresh_cfg() -> Rc<Configuration> {
    Rc::new(Configuration::default())
}

fn parse_no_panic(template: &str) {
    let cfg = fresh_cfg();
    let _ = parse(&cfg, "fuzz", template);
}

fn expression_rich_template(prefix: &str, middle: &str, suffix: &str) -> String {
    format!(
        "<html><body><p th:text=\"${{{middle}}}\" th:if=\"${{{middle}}}\">{prefix}</p>\
         <span th:if=\"${{{middle}}} == 'x'\">{suffix}</span></body></html>"
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn html_parser_never_panics(template in "\\PC{0,512}") {
        parse_no_panic(&template);
    }

    #[test]
    fn template_render_smoke_never_panics(
        prefix in "\\PC{0,64}",
        middle in "\\PC{0,64}",
        suffix in "\\PC{0,64}",
    ) {
        let template = expression_rich_template(&prefix, &middle, &suffix);
        let cfg = fresh_cfg();
        if let Ok(tpl) = parse(&cfg, "fuzz", &template) {
            let mut map: indexmap::IndexMap<String, TModel> = IndexMap::default();
            map.insert("value".to_owned(), TModel::from_scalar("v".to_owned()));
            let root = TModel::from_hash(map);
            let mut out: Vec<u8> = Vec::new();
            let _ = tpl.process(root, &mut out);
        }
    }
}