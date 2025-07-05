use anyhow::{Result, anyhow};
use chrono::prelude::*;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use once_cell::sync::Lazy;
use regex::Regex;

use config::CONFIG;

/// Parse struct has methods implemented parsers for accepted formats.
#[allow(dead_code)]
pub struct Parse<'z, Tz2> {
    tz: &'z Tz2,
    default_time: Option<NaiveTime>,
}

impl<'z, Tz2> Parse<'z, Tz2>
where
    Tz2: TimeZone,
{
    /// Create a new instrance of [`Parse`] with a custom parsing timezone that handles the
    /// datetime string without time offset.
    pub fn new(tz: &'z Tz2, default_time: Option<NaiveTime>) -> Self {
        Self { tz, default_time }
    }

    // https://stackoverflow.com/questions/61179070/rust-chrono-parse-date-string-parseerrornotenough-and-parseerrortooshort/61179071#61179071
    pub fn parse(&self, input: &str) -> Result<DateTime<Utc>> {
        self.ymd(input)
            .or_else(|_| self.ymd_hms(input))
            .or_else(|_| self.ymd_hms_tz(input))
            .or_else(|_| self.rfc3339(input))
            .or_else(|_| self.rfc2822(input))
            .or_else(|_| self.non_standard(input))
            .or_else(|_| self.force_ymd(input))
    }

    // 没有时区信息的日期 "2020-04-12" => Date = NaiveDate
    //  { "fmt" = "%Y-%m-%d", "test" = "2002-06-20" },
    // { "fmt" = "%Y:%m:%d", "test" = "2010:06:24" },
    fn ymd(&self, input: &str) -> Result<DateTime<Utc>> {
        if input.len() == 10 {
            for strip in CONFIG.dateparse.list.iter() {
                if strip.fmt.len() == 8 {
                    if let Ok(d) = NaiveDate::parse_from_str(input, &strip.fmt) {
                        // NaiveDate 转为 DateTime<Utc>
                        return Ok(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap()));
                    }
                }
            }
        }
        Err(anyhow!("NaiveDate::parse_from_str failed"))
    }

    // 没有时区信息的日期和时间 "2020-04-12 22:10:57" => Date + Time = NaiveDateTime
    // { "fmt" = "%Y-%m-%d %H:%M:%S", "test" = "2017-08-16 12:18:36" },
    // { "fmt" = "%Y:%m:%d %H:%M:%S", "test" = "2017:08:16 12:18:36" },
    // { "fmt" = "%Y/%m/%d %H:%M:%S", "test" = "2017/08/16 12:18:36" },
    pub fn ymd_hms(&self, input: &str) -> Result<DateTime<Utc>> {
        if input.len() == 19 {
            for strip in CONFIG.dateparse.list.iter() {
                if strip.fmt.len() == 17 {
                    if let Ok(dt) = NaiveDateTime::parse_from_str(input, &strip.fmt) {
                        // NaiveDateTime 转为 DateTime<Utc>
                        return Ok(Utc.from_utc_datetime(&dt));
                    }
                }
            }
        }
        Err(anyhow!("NaiveDateTime::parse_from_str failed"))
    }

    // 带有时区信息的日期和时间 "2020-04-12 22:10:57+08:00" => Date + Time + TimeZone = DateTime<Tz>
    // { "fmt" = "%Y-%m-%d %H:%M:%S%:z", "test" = "2017-08-16 12:18:36+02:00" },
    // { "fmt" = "%Y-%m-%d %H:%M:%S%%z", "test" = "2017-08-16 12:18:36+0200" },
    // { "fmt" = "%Y-%m-%d %H:%M:%S% %Z", "test" = "2017-08-16 12:18:36 UTC" },
    fn ymd_hms_tz(&self, input: &str) -> Result<DateTime<Utc>> {
        if input.len() > 19 {
            for strip in CONFIG.dateparse.list.iter() {
                if strip.fmt.len() > 17 {
                    if let Ok(dt) = DateTime::parse_from_str(input, &strip.fmt) {
                        // DateTime<Tz> 转为 DateTime<Utc>
                        return Ok(dt.with_timezone(&Utc));
                    }
                    // yyyy-mm-dd hh:mm:ss z
                    // - 2017-11-25 13:31:15 PST
                    // - 2017-11-25 13:31 PST
                    // - 2014-12-16 06:20:00 UTC
                    // - 2014-12-16 06:20:00 GMT
                    // - 2014-04-26 13:13:43 +0800
                    // - 2014-04-26 13:13:44 +09:00
                    // - 2012-08-03 18:31:59.257000000 +0000
                    // - 2015-09-30 18:48:56.35272715 UTC
                    if let Ok(t) = NaiveDateTime::parse_from_str(input, &strip.fmt) {
                        // NaiveDateTime 转为 DateTime<Utc>
                        return Ok(Utc.from_utc_datetime(&t));
                    }
                }
            }
        }

        Err(anyhow!("DateTime::parse_from_str failed"))
    }

    // RFC3339 = Date + Time + TimeZone, YYYY-MM-DDTHH:MM:SS[.ffffff]Z 或 YYYY-MM-DDTHH:MM:SS[.ffffff]±HH:MM
    // "2001-07-08T00:08:56+05:00";
    fn rfc3339(&self, input: &str) -> Result<DateTime<Utc>> {
        if input.len() > 20 {
            if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
                // DateTime<Tz> 转为 DateTime<Utc>
                return Ok(dt.with_timezone(&Utc));
            }
        }
        Err(anyhow!("DateTime::parse_from_rfc3339 failed"))
    }

    // RFC2822 = Date + Time + TimeZone, day-of-week, day month year hour:minute:second zone
    // "Tue, 1 Jul 2003 10:52:37 +0200";
    // "Wed, 30 Nov 2022 05:58:56 +0100"
    fn rfc2822(&self, input: &str) -> Result<DateTime<Utc>> {
        if input.len() > 20 {
            if let Ok(dt) = DateTime::parse_from_rfc2822(input) {
                // DateTime<Tz> 转为 DateTime<Utc>
                return Ok(dt.with_timezone(&Utc));
            }
        }
        Err(anyhow!("DateTime::parse_from_rfc2822 failed"))
    }

    // other or non-standard, must contain a timezone
    // "2020-04-12 22:10:57 +02:00" => Date + Time + Timezone
    //
    fn non_standard(&self, input: &str) -> Result<DateTime<Utc>> {
        // dbg!("non-standard: {}", input);
        for strip in CONFIG.dateparse.list.iter() {
            if strip.fmt.len() > 9 {
                if let Ok(dt) = DateTime::parse_from_str(input, &strip.fmt) {
                    // DateTime<Tz> 转为 DateTime<Utc>
                    return Ok(dt.with_timezone(&Utc));
                }
                // like 2018-06-30T17:11
                if let Ok(t) = NaiveDateTime::parse_from_str(input, &strip.fmt) {
                    // NaiveDateTime 转为 DateTime<Utc>
                    return Ok(Utc.from_utc_datetime(&t));
                }
            }
        }
        Err(anyhow!("other or non-standard failed"))
    }

    pub(crate) fn force_ymd(&self, input: &str) -> Result<DateTime<Utc>> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d{4}[-:/]\d{2}[-:/]\d{2})").unwrap());

        if let Some(caps) = RE.captures(input) {
            if let Some(c) = caps.get(0) {
                let date = c.as_str().replacen(":", "-", 2).replacen("/", "-", 2);

                if let Ok(d) = NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d") {
                    return Ok(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap()));
                }
            }
        }
        Err(anyhow!("Force parsed with ymd failed"))
    }
}
