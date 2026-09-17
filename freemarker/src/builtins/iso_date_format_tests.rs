//! ISO 日期格式化测试 —— 自 iso_date_format.rs 拆出（#[cfg(test)] 模块）。

use super::*;

#[cfg(test)]
mod tests {
    use super::iso_date_format_parse::fields_to_date;
    use super::*;
    use crate::core::TzSetting;
    use chrono::{NaiveDate, TimeZone};

    fn tz_gmt2() -> TzSetting {
        "GMT+02".parse().unwrap()
    }

    #[test]
    fn iso_params_parse() {
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(spec.accuracy, Accuracy::Milliseconds);
        assert_eq!(spec.show_zone_offset, None);
        let spec = parse_iso_params("iso ms nz", 3, false).unwrap();
        assert_eq!(spec.accuracy, Accuracy::MillisecondsForced);
        assert_eq!(spec.show_zone_offset, Some(false));
        let spec = parse_iso_params("xs_fz", 2, true).unwrap();
        assert_eq!(spec.show_zone_offset, Some(true));
        let spec = parse_iso_params("iso_s_u", 3, false).unwrap();
        assert_eq!(spec.accuracy, Accuracy::Seconds);
        assert_eq!(spec.force_utc, None);
        let spec = parse_iso_params("iso_h", 3, false).unwrap();
        assert_eq!(spec.accuracy, Accuracy::Hours);
        // XS 不允许 h/m 精度
        assert!(parse_iso_params("xs_h", 2, true).is_err());
        assert!(parse_iso_params("iso x", 3, false).is_err());
    }

    fn d(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, ms: u32) -> DateValue {
        let naive = NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_milli_opt(h, mi, s, ms)
            .unwrap();
        DateValue::new(
            Utc.from_utc_datetime(&naive)
                .with_timezone(&FixedOffset::east_opt(0).unwrap()),
            DateType::DateTime,
        )
    }

    #[test]
    fn format_iso_variants() {
        let dt = d(2010, 5, 15, 20, 38, 5, 23);
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dt, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.023+02:00"
        );
        let spec = parse_iso_params("iso", 3, false).unwrap();
        assert_eq!(
            format_iso_like(&dt, &spec, false, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.023+02:00"
        );
        // 毫秒为 0 → 不输出分数
        let dt0 = d(2010, 5, 15, 20, 38, 5, 0);
        assert_eq!(
            format_iso_like(&dt0, &spec, false, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05+02:00"
        );
        // ms 强制 3 位
        let spec_ms = parse_iso_params("iso ms", 3, false).unwrap();
        let dt10 = d(2010, 5, 15, 20, 38, 5, 10);
        assert_eq!(
            format_iso_like(&dt10, &spec_ms, false, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.010+02:00"
        );
        // 最小位数
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dt10, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.01+02:00"
        );
        let dt100 = d(2010, 5, 15, 20, 38, 5, 100);
        assert_eq!(
            format_iso_like(&dt100, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15T22:38:05.1+02:00"
        );
    }

    #[test]
    fn format_date_only_variants() {
        let dt = d(2010, 5, 15, 20, 38, 5, 23);
        let mut dd = dt.clone();
        dd.kind = DateType::Date;
        // XS date-only 默认带偏移（非 SQL），iso 恒不带
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15+02:00"
        );
        let spec_fz = parse_iso_params("xs_fz", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec_fz, true, &tz_gmt2()).unwrap(),
            "2010-05-15+02:00"
        );
        let spec_iso = parse_iso_params("iso", 3, false).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec_iso, false, &tz_gmt2()).unwrap(),
            "2010-05-15"
        );
        let spec_iso_fz = parse_iso_params("iso_fz", 3, false).unwrap();
        assert_eq!(
            format_iso_like(&dd, &spec_iso_fz, false, &tz_gmt2()).unwrap(),
            "2010-05-15"
        );
        // SQL date：xs 默认不带偏移，xs_fz 带
        dd.is_sql = true;
        assert_eq!(
            format_iso_like(&dd, &spec, true, &tz_gmt2()).unwrap(),
            "2010-05-15"
        );
        assert_eq!(
            format_iso_like(&dd, &spec_fz, true, &tz_gmt2()).unwrap(),
            "2010-05-15+02:00"
        );
    }

    #[test]
    fn format_time_only_variants() {
        let t = DateValue {
            dt: DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(1970, 1, 1)
                    .unwrap()
                    .and_hms_milli_opt(20, 38, 5, 23)
                    .unwrap(),
                Utc,
            )
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
            kind: DateType::Time,
            is_sql: true,
        };
        let spec = parse_iso_params("xs", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&t, &spec, true, &tz_gmt2()).unwrap(),
            "22:38:05.023"
        );
        let spec_fz = parse_iso_params("xs_fz", 2, true).unwrap();
        assert_eq!(
            format_iso_like(&t, &spec_fz, true, &tz_gmt2()).unwrap(),
            "22:38:05.023+02:00"
        );
        // 非 SQL 时间默认带偏移
        let mut t2 = t.clone();
        t2.is_sql = false;
        assert_eq!(
            format_iso_like(&t2, &spec, true, &tz_gmt2()).unwrap(),
            "22:38:05.023+02:00"
        );
    }

    #[test]
    fn parse_iso_and_xs() {
        let tz = tz_gmt2();
        let spec = parse_iso_params("xs", 2, true).unwrap();
        // 扩展
        let v = parse_iso_like(
            "2010-05-15T22:38:05.023+02:00",
            DateType::DateTime,
            &spec,
            &tz,
            true,
        )
        .unwrap();
        assert_eq!(v.dt, d(2010, 5, 15, 20, 38, 5, 23).dt);
        // 无时区 → 默认时区
        let v = parse_iso_like(
            "2010-05-15T22:38:05.023",
            DateType::DateTime,
            &spec,
            &tz,
            true,
        )
        .unwrap();
        assert_eq!(v.dt, d(2010, 5, 15, 20, 38, 5, 23).dt);
        // ISO 基本形式 + 逗号小数 + +01 偏移
        let spec_iso = parse_iso_params("iso", 3, false).unwrap();
        let v = parse_iso_like(
            "19981030T153044,512+01",
            DateType::DateTime,
            &spec_iso,
            &tz,
            false,
        )
        .unwrap();
        assert_eq!(v.dt, d(1998, 10, 30, 14, 30, 44, 512).dt);
        // 基本日期（GMT+02 默认时区 → 1998-10-29T22:00Z）
        let v = parse_iso_like("19981030", DateType::Date, &spec_iso, &tz, false).unwrap();
        assert_eq!(
            v.dt.with_timezone(&Utc),
            d(1998, 10, 29, 22, 0, 0, 0).dt.with_timezone(&Utc)
        );
        // XS 日期带 Z
        let v = parse_iso_like("1998-10-30Z", DateType::Date, &spec, &tz, true).unwrap();
        assert_eq!(v.dt, d(1998, 10, 30, 0, 0, 0, 0).dt);
        // 时间基本形式
        let v = parse_iso_like("153044,512Z", DateType::Time, &spec_iso, &tz, false).unwrap();
        assert_eq!(v.dt, d(1970, 1, 1, 15, 30, 44, 512).dt);
    }

    #[test]
    fn julian_year_600() {
        // Java GregorianCalendar：600-01-01（儒略历）→ 公历 0600-01-03
        let v = fields_to_date(
            600,
            1,
            1,
            23,
            59,
            59,
            123,
            FixedOffset::east_opt(0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            v.dt.format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
            "0600-01-03T23:59:59.123"
        );
        // 与 Java 基准一致：?iso_utc_ms → 0600-01-03T23:59:59.123Z（UTC）
        let spec = parse_iso_params("iso ms", 3, false).unwrap();
        let utc = TzSetting::Fixed(FixedOffset::east_opt(0).unwrap());
        assert_eq!(
            format_iso_like(&v, &spec, false, &utc).unwrap(),
            "0600-01-03T23:59:59.123Z"
        );
    }
}
