//! 版本 —— 对应 Java `freemarker.template.Version`
//! （incompatibleImprovements 设置使用；to_int 编码同 Java intValue）

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub micro: u32,
}

impl Version {
    pub const V2_3_34: Version = Version {
        major: 2,
        minor: 3,
        micro: 34,
    };
    pub const V2_3_0: Version = Version {
        major: 2,
        minor: 3,
        micro: 0,
    };

    pub fn to_int(&self) -> i64 {
        (self.major as i64) * 1_000_000 + (self.minor as i64) * 1_000 + self.micro as i64
    }

    pub fn parse(s: &str) -> std::result::Result<Version, String> {
        let parts: Vec<u32> = s.split('.').map(|p| p.parse().unwrap_or(0)).collect();
        if parts.len() < 3 {
            return Err(format!("Invalid version: {s}"));
        }
        Ok(Version {
            major: parts[0],
            minor: parts[1],
            micro: parts[2],
        })
    }
}
