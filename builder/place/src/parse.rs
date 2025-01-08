use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};

use super::FileDateTime;

use config::{Parser, CONFIG};

fn capture_from_string(value: &str, parsers: &Vec<Parser>) -> Option<String> {
    for parser in parsers {
        // ensure the string contains the parser name
        if value.contains(&parser.name) {
            match parser.capture(&value) {
                Ok(t) => {
                    log::info!("capture {} from metadata: {}", parser.name, t);
                    return Some(t);
                }
                Err(e) => {
                    log::error!("capture {} from metadata with error: {}", parser.name, e);
                }
            }
        }
    }
    None
}

pub fn capture_type(value: &str) -> Option<String> {
    // capture file extension from string
    capture_from_string(value, &CONFIG.typeparse)
}

pub fn capture_date(value: &str) -> Option<String> {
    capture_from_string(value, &CONFIG.dateparse)
}

pub fn get_datetime_from_string(value: &str) -> Option<FileDateTime> {
    let date_str = capture_date(value);
    if date_str.is_none() {
        return None;
    }
    let data_str = date_str.unwrap();
    for strip in &CONFIG.striptimes {
        // 如果包含非 ascii 字符，尝试替换为 ascii 字符并解析
        if !value.chars().all(|c| c.is_ascii()) {
            for c in vec![" ", "-", ":", "1", ""] {
                let repl_text = value.replace(|c: char| !c.is_ascii(), c);
                log::debug!("replace non-ascii {} with {}", value, repl_text);
                if let Some(dt) = fuzzy_strptime(&repl_text, &strip.fmt) {
                    return Some(dt);
                }
            }
        } else {
            if let Some(dt) = fuzzy_strptime(&data_str, &strip.fmt) {
                return Some(dt);
            }
        }
    }
    None
}
// https://stackoverflow.com/questions/61179070/rust-chrono-parse-date-string-parseerrornotenough-and-parseerrortooshort/61179071#61179071
fn fuzzy_strptime(value: &str, fmt: &str) -> Option<FileDateTime> {
    // like "2020-04-12" => Date = NaiveDate
    if value.len() == 10 {
        match NaiveDate::parse_from_str(&value, fmt) {
            Ok(date) => {
                return Some(FileDateTime {
                    year: date.year() as u16,
                    month: date.month() as u8,
                    day: date.day() as u8,
                    hour: 0,
                    minute: 0,
                    second: 0,
                    timestamp: date.and_hms_opt(0, 0, 0).unwrap().timestamp() as i64,
                });
            }
            Err(e) => log::debug!("try NaiveDate {} as {}, {}", value, fmt, e),
        }
    }

    // like "2020-04-12 22:10:57" => Date + Time = NaiveDateTime
    if value.len() == 19 {
        match NaiveDateTime::parse_from_str(&value, fmt) {
            Ok(date) => {
                return Some(FileDateTime {
                    year: date.year() as u16,
                    month: date.month() as u8,
                    day: date.day() as u8,
                    hour: date.hour() as u8,
                    minute: date.minute() as u8,
                    second: date.second() as u8,
                    timestamp: date.timestamp() as i64,
                });
            }
            Err(e) => log::debug!("try NaiveDateTime {} as {}, {}", value, fmt, e),
        }
    }

    // Date + Time + Timezone (other or non-standard)
    match DateTime::parse_from_str(&value, fmt) {
        Ok(date) => {
            return Some(FileDateTime {
                year: date.year() as u16,
                month: date.month() as u8,
                day: date.day() as u8,
                hour: date.hour() as u8,
                minute: date.minute() as u8,
                second: date.second() as u8,
                timestamp: date.timestamp() as i64,
            });
        }
        Err(e) => log::debug!("try DateTime {} as {}, {}", value, fmt, e),
    }

    match Utc.datetime_from_str(&value, fmt) {
        Ok(dt) => {
            return Some(FileDateTime {
                year: dt.year() as u16,
                month: dt.month() as u8,
                day: dt.day() as u8,
                hour: dt.hour() as u8,
                minute: dt.minute() as u8,
                second: dt.second() as u8,
                timestamp: dt.timestamp() as i64,
            });
        }
        Err(e) => log::debug!("try Utc {} as {}, {}", value, fmt, e),
    }

    None
}
