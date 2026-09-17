//! 测试 —— 自 java_date_format.rs 拆出（#[cfg(test)] 模块；由主文件
//! #[cfg(test)] #[path] 声明，仅测试构建时编译）。

use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TzSetting;
    use chrono::NaiveDate;
    use chrono::TimeZone as _; // from_utc_datetime

    fn gmt() -> TzSetting {
        TzSetting::Fixed(FixedOffset::east_opt(0).unwrap())
    }

    fn d2002() -> DateValue {
        let naive = NaiveDate::from_ymd_opt(2002, 11, 15)
            .unwrap()
            .and_hms_opt(14, 54, 13)
            .unwrap();
        DateValue::new(
            Utc.from_utc_datetime(&naive)
                .with_timezone(&FixedOffset::east_opt(0).unwrap()),
            DateType::DateTime,
        )
    }

    #[test]
    fn named_styles_en_us() {
        // dateformat-java.txt 期望值（2002-11-15 14:54:13 GMT）
        let d = d2002();
        assert_eq!(
            format_java(
                &resolve_named_style("", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "Nov 15, 2002 2:54:13 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "11/15/02 2:54 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("medium", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "Nov 15, 2002 2:54:13 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("long", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "November 15, 2002 2:54:13 PM GMT"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short_medium", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "11/15/02 2:54:13 PM"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short_long", DateType::DateTime, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "11/15/02 2:54:13 PM GMT"
        );
        // date-only / time-only
        assert_eq!(
            format_java(
                &resolve_named_style("medium", DateType::Date, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "Nov 15, 2002"
        );
        assert_eq!(
            format_java(
                &resolve_named_style("short", DateType::Time, "en_US").unwrap(),
                &d,
                "en_US",
                &gmt()
            )
            .unwrap(),
            "2:54 PM"
        );
        // hu_hu（dateformat-java.txt：long_long）
        assert_eq!(
            format_java(
                &resolve_named_style("long_long", DateType::DateTime, "hu_hu").unwrap(),
                &d,
                "hu_hu",
                &gmt()
            )
            .unwrap(),
            "2002. november 15. 14:54:13 GMT"
        );
    }

    #[test]
    fn java_patterns() {
        let d = d2002();
        assert_eq!(
            format_java("EEE, dd MMM yyyyy HH:mm:ss z", &d, "en_US", &gmt()).unwrap(),
            "Fri, 15 Nov 02002 14:54:13 GMT"
        );
        assert_eq!(
            format_java("EEE, dd MMM yyyy HH:mm:ss z", &d, "en_US", &gmt()).unwrap(),
            "Fri, 15 Nov 2002 14:54:13 GMT"
        );
        assert_eq!(format_java("yyyy", &d, "en_US", &gmt()).unwrap(), "2002");
        assert_eq!(format_java("MM", &d, "en_US", &gmt()).unwrap(), "11");
    }

    #[test]
    fn parse_java_patterns() {
        // dateparsing：'AD 1998-10-30 19:30:44.512 +0400'?datetime（G yyyy-MM-dd HH:mm:ss.S Z）
        let d = parse_java(
            "G yyyy-MM-dd HH:mm:ss.S Z",
            "AD 1998-10-30 19:30:44.512 +0400",
            DateType::DateTime,
            "en_US",
            &gmt(),
        )
        .unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string(),
            "1998-10-30 15:30:44.512"
        );
        // '10/30/1998 19:30:44:512 GMT+04:00'?datetime("MM/dd/yyyy HH:mm:ss:S z")
        let d = parse_java(
            "MM/dd/yyyy HH:mm:ss:S z",
            "10/30/1998 19:30:44:512 GMT+04:00",
            DateType::DateTime,
            "en_US",
            &gmt(),
        )
        .unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string(),
            "1998-10-30 15:30:44.512"
        );
        // '2010-05-15 22:38:05:23 +0200'?datetime("yyyy-MM-dd HH:mm:ss:S Z")
        let d = parse_java(
            "yyyy-MM-dd HH:mm:ss:S Z",
            "2010-05-15 22:38:05:23 +0200",
            DateType::DateTime,
            "en_US",
            &gmt(),
        )
        .unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string(),
            "2010-05-15 20:38:05.023"
        );
        // date-only 解析（dateformat-iso-bi-common：GMT+02 时区）
        let tz2 = TzSetting::Fixed(FixedOffset::east_opt(7200).unwrap());
        let d = parse_java("yyyy-MM-dd", "2010-05-15", DateType::Date, "en_US", &tz2).unwrap();
        assert_eq!(
            d.dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            "2010-05-14 22:00:00"
        );
    }
}
