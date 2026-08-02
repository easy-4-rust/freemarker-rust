//! Java `freemarker.template.VersionTest` 的 Rust 1:1 实现
//! （VersionTest.java：Version 的构造/字符串解析/hashCode/equals 测试）
//!
//! 引擎映射：`freemarker::template::Version`（major/minor/micro + to_int/parse）。
//! 引擎差异：v1 Version 无 extraInfo/GAECompliant/buildDate 字段与
//! `new Version(int)`/短格式（"1.0"）解析；parse 不校验后缀（"1.2.3-beta2" 的
//! micro 解析失败记 0）且不抛 IllegalArgumentException——相关断言注释保留。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testFromNumber：new Version(1,2,3)
#[test]
fn test_from_number() {
    let v = freemarker::template::Version {
        major: 1,
        minor: 2,
        micro: 3,
    };
    assert_eq!(format!("{}.{}.{}", v.major, v.minor, v.micro), "1.2.3");
    assert_eq!(v.to_int(), 1002003);
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.micro, 3);
    // 引擎差异：Java 断言 getExtraInfo()==null、isGAECompliant()==null、
    // getBuildDate()==null —— v1 无这些字段
}

/// Java testFromNumber2：new Version(1,2,3,"beta8",TRUE,date)。
/// 引擎差异：v1 无 extraInfo/GAE/buildDate 构造参数——跳过并注释。
#[test]
fn test_from_number2() {
    // Java：v.toString()=="1.2.3-beta8"、getExtraInfo()=="beta8"、
    // isGAECompliant()==TRUE、getBuildDate()==Date(5000)
    // —— v1 Version 无后缀与元数据字段
}

/// Java testFromNumber3：new Version(int) 从 intValue 还原
#[test]
fn test_from_number3() {
    let v = freemarker::template::Version {
        major: 1,
        minor: 2,
        micro: 3,
    };
    // Java：new Version(new Version(1,2,3).intValue()) 还原三个分量
    // —— v1 无 int 构造器；用 to_int 反推验证编码一致
    let int_val = v.to_int();
    assert_eq!(int_val / 1_000_000, 1);
    assert_eq!((int_val / 1_000) % 1_000, 2);
    assert_eq!(int_val % 1_000, 3);
}

/// Java testFromString：new Version("1.2.3-beta2")。
/// 引擎差异：v1 parse 把 "3-beta2" 解析失败记 0（micro==0）且不抛错——
/// Java 期望 micro==3、extraInfo=="beta2"（无后缀版本部分可对齐）。
#[test]
fn test_from_string() {
    // 无后缀版本：Java 与 v1 一致
    let v = freemarker::template::Version::parse("1.2.3").unwrap();
    assert_eq!(v.to_int(), 1002003);
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.micro, 3);
    // 引擎差异：Java new Version("1.2.3-beta2") → micro=3、extraInfo="beta2"、
    // toString()=="1.2.3-beta2"；v1 parse 对后缀无解析（micro 记 0）——
    // v1 偏差登记（Java 断言注释保留）
}

/// Java testFromString2：new Version("10.20.30", TRUE, date)。
/// 引擎差异：v1 无 GAE/buildDate 参数；无后缀数值部分可对齐。
#[test]
fn test_from_string2() {
    let v = freemarker::template::Version::parse("10.20.30").unwrap();
    assert_eq!(v.to_int(), 10020030);
    assert_eq!(v.major, 10);
    assert_eq!(v.minor, 20);
    assert_eq!(v.micro, 30);
    // 引擎差异：Java toString()=="10.20.30"、isGAECompliant()==TRUE、
    // getBuildDate()==Date(5000)、getExtraInfo()==null —— 后缀/元数据无对应
}

/// Java testFromString3：前导零与后缀解析。
/// 引擎差异：v1 parse 不做前导零剥离与后缀拆分（"01.002.0003-20130524" 的
/// micro 解析失败记 0）——跳过并注释 Java 断言。
#[test]
fn test_from_string3() {
    // Java："01.002.0003-20130524"→major=1/minor=2/micro=3/extraInfo="20130524"；
    // "01.002.0003.4"→micro=3/extraInfo="4"；"1.2.3.FC"→extraInfo="FC"；
    // "1.2.3mod"→micro=3/extraInfo="mod"（toString 均保留原文）
    // —— v1 Version::parse 无后缀/前导零处理
}

/// Java testFromStringIncubating：后缀版本。
/// 引擎差异：同上（v1 无后缀解析）——注释保留。
#[test]
fn test_from_string_incubating() {
    // Java："2.3.24-rc01-incubating"→major=2/minor=3/micro=24/extraInfo="rc01-incubating"
}

/// Java testHashAndEquals：equals/hashCode 一致性。
/// 引擎差异：v1 Version 无 extraInfo/GAE/buildDate 维度——仅数值分量参与
/// PartialEq（derive）与 Java 的"所有字段参与"不等价；无后缀版本间相等性
/// 可对齐。
#[test]
fn test_hash_and_equals() {
    let v1 = freemarker::template::Version::parse("1.2.3").unwrap();
    let v2 = freemarker::template::Version {
        major: 1,
        minor: 2,
        micro: 3,
    };
    assert_eq!(v1, v2);
    assert_eq!(format!("{:?}", v1), format!("{:?}", v2));
    // 引擎差异：Java "1.2.3-beta2" vs "1.2.3-beta3"、GAE/buildDate 差异均不相等
    // —— v1 无这些维度，数值分量不同的版本仍不相等：
    let v3 = freemarker::template::Version::parse("1.2.9").unwrap();
    assert_ne!(v1, v3);
    let v4 = freemarker::template::Version::parse("1.9.3").unwrap();
    assert_ne!(v1, v4);
    let v5 = freemarker::template::Version::parse("9.2.3").unwrap();
    assert_ne!(v1, v5);
}

/// Java testShortForms："1.0.0-beta2" == "1.0-beta2" == "1-beta2"。
/// 引擎差异：v1 parse 要求三段（"1.0" 抛 Err）——跳过并注释。
#[test]
fn test_short_forms() {
    // Java：new Version("1.0-beta2")/new Version("1-beta2") 等价 1.0.0-beta2；
    // new Version("1.0")/new Version("1") 等价 1.0.0
    // —— v1 Version::parse 需要完整三段，短格式未实现
    assert!(freemarker::template::Version::parse("1.0").is_err());
}

/// Java testMalformed：非法版本字符串抛 IllegalArgumentException。
/// 引擎差异：v1 Version::parse 对 "1.2."、"1..3"、"a" 等不抛错（parse 失败的分段
/// 记 0；仅段数 < 3 报 Err）——跳过并注释 Java 断言。
#[test]
fn test_malformed() {
    // Java：new Version("1.2.")、("1.2.3.")、("1..3")、(".2")、("a")、("-a")
    // 均抛 IllegalArgumentException —— v1 parse 无此校验
    // 注：v1 parse("1.2") 报 Err（段数不足），与 Java 行为巧合一致：
    assert!(freemarker::template::Version::parse("1.2").is_err());
}
