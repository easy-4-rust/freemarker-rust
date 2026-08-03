//! cargo-fuzz target: freemarker 表达式解析器
//!
//! 随机输入下 parse_expression() 必须返回 Result 但不得 panic。
//! 运行: cargo fuzz run expression -- -max_total_time=60

use std::rc::Rc;

use freemarker::parser::parse_expression;
use freemarker::template::Configuration;

fuzz_target!(|data: &[u8]| {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };
    if text.len() > 4_096 {
        return;
    }
    let cfg = Rc::new(Configuration::default());
    let _ = parse_expression(&cfg, text);
});
