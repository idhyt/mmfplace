use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
// use filetime::FileTime;
use std::{path::PathBuf, time::SystemTime};

use super::{panic_with_test, FileDateTime};

use config::{Parser, Strptime, CONFIG};

fn capture_from_string(value: &str, parsers: &Vec<Parser>, check: bool) -> Option<String> {
    for parser in parsers {
        // ensure the string contains the parser name
        if check && !value.contains(&parser.check) {
            continue;
        }
        match parser.capture(&value) {
            Ok(t) => {
                log::debug!("capture {} from {}", t, value);
                return Some(t);
            }
            Err(e) => {
                log::debug!("capture {} from {} with error: {}", parser.regex, value, e);
            }
        }
    }
    None
}

pub fn capture_type(value: &str) -> Option<String> {
    // capture file extension from string
    capture_from_string(value, &CONFIG.typeparse, true)
}

pub fn capture_date(value: &str) -> Option<String> {
    capture_from_string(value, &CONFIG.dateparse, true)
}

/// 解析所有可能的时间格式，格式见配置文件中的 `striptimes`
// https://stackoverflow.com/questions/61179070/rust-chrono-parse-date-string-parseerrornotenough-and-parseerrortooshort/61179071#61179071
#[warn(deprecated)]
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

fn get_datetime_with_striptimes(value: &str, striptimes: &Vec<Strptime>) -> Option<FileDateTime> {
    for strip in striptimes {
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
            if let Some(dt) = fuzzy_strptime(&value, &strip.fmt) {
                return Some(dt);
            }
        }
    }

    // warning!!!
    // 如果没有解析出时间字符串，说明在配置文件中 `striptimes` 缺失时间格式，需要强制处理！
    log::error!(
        "💥 Unrecognized time string format: {}, must add parsing format `striptimes` in config.toml`",
        value
    );
    panic!();
}

/// 从给定字符串中获取时间
pub fn get_datetime_from_string(value: &str) -> Option<FileDateTime> {
    let date_str = capture_date(value);
    if date_str.is_none() {
        return None;
    }
    let data_str = date_str.unwrap();
    get_datetime_with_striptimes(&data_str, &CONFIG.striptimes)
}

pub fn get_datatime_from_metadata(file: &PathBuf) -> Option<Vec<SystemTime>> {
    let metadata = std::fs::metadata(file);
    if metadata.is_err() {
        log::error!(
            "get metadata {} failed with error {:?}",
            file.display(),
            metadata.err()
        );
        return None;
    }
    let metadata = metadata.unwrap();
    let mut times = vec![];

    if let Ok(atime) = metadata.accessed() {
        times.push(atime);
    } else {
        log::warn!("💡 last access time Not supported on this platform!");
    }
    if let Ok(mtime) = metadata.modified() {
        times.push(mtime);
    } else {
        log::warn!("💡 last modified time Not supported on this platform!");
    }
    if let Ok(ctime) = metadata.created() {
        times.push(ctime);
    } else {
        log::warn!("💡 creation time Not supported on this platform!");
    }

    if times.is_empty() {
        None
    } else {
        Some(times)
    }
}

/// 从文件属性中获取访问时间、创建时间、修改时间，并返回最早的时间
pub fn get_earliest_datetime_from_attributes(file: &PathBuf) -> Option<FileDateTime> {
    if let Some(times) = get_datatime_from_metadata(&file) {
        if times.len() != 3 {
            panic_with_test();
        }

        if let Some(dt) = times.into_iter().min() {
            // let dt: DateTime<Utc> = dt.into();
            let dt: DateTime<Local> = dt.into();
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
    }
    log::error!("get attributes min timestamp failed for {}", file.display());
    None
}

/// 从文件名中获取时间
/// 1. 从文件名中捕获时间字符串
/// 2. 通过时间字符串解析时间
fn get_datetime_from_filename(
    file: &PathBuf,
    dateparse: &Vec<Parser>,
    striptimes: &Vec<Strptime>,
) -> Option<FileDateTime> {
    let name = file.file_name();
    if name.is_none() {
        return None;
    }
    let name = name.unwrap().to_string_lossy();
    // 从文件名中捕获时间字符串
    if let Some(value) = capture_from_string(&name, dateparse, false) {
        // 尝试解析时间
        if let Some(dt) = get_datetime_with_striptimes(&value, striptimes) {
            return Some(dt);
        }
    }
    None
}

pub fn get_datetime_from_additional(file: &PathBuf) -> Option<FileDateTime> {
    if let Some(additionals) = &CONFIG.additionals {
        for additional in additionals.iter() {
            if additional.name == "filename" {
                return get_datetime_from_filename(
                    file,
                    &additional.dateparse,
                    &additional.striptimes,
                );
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_capture_date() {
        let test = "[Exif SubIFD] Date/Time Digitized = 2002:11:16 15:27:01";
        let dt = capture_date(test);
        println!("dt: {:?}", dt);
        assert!(dt.is_some());
    }

    #[test]
    fn test_capture_type() {
        let test = "> [File Type] Expected File Name Extension = jpg";
        let dt = capture_type(test);
        println!("dt: {:?}", dt);
        assert_eq!(dt, Some("jpg".to_string()));
    }

    #[test]
    fn test_get_datetime_from_string() {
        let test = "[Exif SubIFD] Date/Time Digitized = 2002:11:16 15:27:01";
        let dt = capture_date(test);
        println!("dt: {:?}", dt);
        assert!(dt.is_some());

        let dt = get_datetime_from_string(&test);
        println!("dt: {:?}", dt);
        assert!(dt.is_some());
    }

    #[test]
    fn test_get_earliest_datetime_from_attributes() {
        let file = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/simple.jpg");
        println!("test file: {}", file.display());
        assert!(file.is_file());

        let dt = get_earliest_datetime_from_attributes(&file);
        println!("dt: {:?}", dt);
        assert!(dt.is_some());
    }

    #[test]
    fn test_get_datetime_from_additional() {
        let file = PathBuf::from("./tests/test.jpg");
        let dt = get_datetime_from_additional(&file);
        println!("dt: {:?}", dt);
        assert!(dt.is_none());
        let file = PathBuf::from("./tests/IMG_2018-05-02-13-13-39-01-0001.sha.md5.xxx.jpg.jpg");
        let dt = get_datetime_from_additional(&file);
        println!("dt: {:?}", dt);
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.year, 2018);
        assert_eq!(dt.second, 39);
    }
}
