use anyhow::Result;
use chrono::prelude::*;

use datetime::Parse;

pub mod datetime;

pub fn parse(input: &str) -> Result<DateTime<Utc>> {
    Parse::new(&Local, None).parse(input)
}

pub fn parse_with_timezone<Tz2: TimeZone>(input: &str, tz: &Tz2) -> Result<DateTime<Utc>> {
    Parse::new(tz, None).parse(input)
}

pub fn parse_with<Tz2: TimeZone>(
    input: &str,
    tz: &Tz2,
    default_time: NaiveTime,
) -> Result<DateTime<Utc>> {
    Parse::new(tz, Some(default_time)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::CONFIG;

    #[test]
    fn test_parse() {
        for strip in CONFIG.striptimes.iter() {
            dbg!(strip);
            let t = parse(&strip.test).unwrap();
            assert!(
                strip.test.contains(t.year().to_string().as_str())
                    || strip.test.contains(t.month().to_string().as_str())
            );
        }
    }
}
