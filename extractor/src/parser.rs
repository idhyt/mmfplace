use crate::metadata::MetadataReader;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use config::config;
use filetime::{set_file_times, FileTime};
use regex::Regex;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct FileDateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    timestamp: i64,
}

impl FileDateTime {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            year: now.year() as u16,
            month: now.month() as u8,
            day: now.day() as u8,
            hour: now.hour() as u8,
            minute: now.minute() as u8,
            second: now.second() as u8,
            timestamp: now.timestamp() as i64,
        }
    }

    pub fn get_year(&self) -> u16 {
        self.year
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn to_string(&self) -> String {
        format!(
            "{:04}:{:02}:{:02} {:02}:{:02}:{:02}, {}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.timestamp
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileMeta {
    pub file_path: PathBuf,
    pub suffix: String,
    pub datetime: FileDateTime,
}

impl FileMeta {
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        // let file_path = file_path.as_ref().to_path_buf();
        // // let suffix = file_path.extension()?.to_str()?.to_owned().to_lowercase();
        // let suffix = match file_path.extension() {
        //     Some(s) => s.to_str().unwrap().to_owned().to_lowercase(),
        //     None => "".to_string(),
        // };

        Self {
            file_path: file_path.as_ref().to_path_buf(),
            suffix: "".to_string(),
            datetime: FileDateTime::new(),
        }
    }

    pub fn set_datetime(&mut self, datetime: FileDateTime) {
        self.datetime = datetime;
    }

    pub fn set_suffix(&mut self, suffix: &str) {
        self.suffix = suffix.to_owned();
    }

    pub fn get_name(&self, index: u16) -> String {
        if index == 0 {
            return format!(
                "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}.{}",
                self.datetime.year,
                self.datetime.month,
                self.datetime.day,
                self.datetime.hour,
                self.datetime.minute,
                self.datetime.second,
                self.suffix
            );
        } else {
            return format!(
                "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}_{:02}.{}",
                self.datetime.year,
                self.datetime.month,
                self.datetime.day,
                self.datetime.hour,
                self.datetime.minute,
                self.datetime.second,
                index,
                self.suffix
            );
        }
    }

    pub async fn copy_to<T>(&self, dst: T) -> Result<bool>
    where
        T: AsRef<Path>,
    {
        std::fs::copy(&self.file_path, &dst)?;

        let metadata = std::fs::metadata(&self.file_path)?;
        // log::info!("src file time: {:#?}", metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        let mtime = FileTime::from_last_modification_time(&metadata);
        set_file_times(&dst, atime, mtime)?;
        // let metadata = std::fs::metadata(&dst)?;
        // log::info!("dst file time: {:#?}", metadata);
        Ok(true)
    }

    fn regex_text_value(&self, text: &String, restr: &str, index: usize) -> Result<String> {
        let re = Regex::new(restr)?;
        match re.captures(text) {
            Some(caps) => match caps.get(index) {
                Some(cap) => Ok(cap.as_str().trim().to_owned()),
                None => Err(anyhow!("get match failed from from {} by {}", text, restr)),
            },
            None => Err(anyhow!("regex captures failed from {} by {}", text, restr)),
        }
    }

    // https://stackoverflow.com/questions/61179070/rust-chrono-parse-date-string-parseerrornotenough-and-parseerrortooshort/61179071#61179071
    async fn fuzzy_strptime(&self, date_str: &str, fmt: &str) -> Result<Option<FileDateTime>> {
        // like "2020-04-12" => Date = NaiveDate
        if date_str.len() == 10 {
            match NaiveDate::parse_from_str(&date_str, fmt) {
                Ok(date) => {
                    return Ok(Some(FileDateTime {
                        year: date.year() as u16,
                        month: date.month() as u8,
                        day: date.day() as u8,
                        hour: 0,
                        minute: 0,
                        second: 0,
                        timestamp: date.and_hms_opt(0, 0, 0).unwrap().timestamp() as i64,
                    }));
                }
                Err(e) => log::debug!("NaiveDate try {} as {}, {}", date_str, fmt, e),
            }
        }

        // like "2020-04-12 22:10:57" => Date + Time = NaiveDateTime
        if date_str.len() == 19 {
            match NaiveDateTime::parse_from_str(&date_str, fmt) {
                Ok(date) => {
                    return Ok(Some(FileDateTime {
                        year: date.year() as u16,
                        month: date.month() as u8,
                        day: date.day() as u8,
                        hour: date.hour() as u8,
                        minute: date.minute() as u8,
                        second: date.second() as u8,
                        timestamp: date.timestamp() as i64,
                    }));
                }
                Err(e) => log::debug!("NaiveDateTime try {} as {}, {}", date_str, fmt, e),
            }
        }

        // Date + Time + Timezone (other or non-standard)
        match DateTime::parse_from_str(&date_str, fmt) {
            Ok(date) => {
                return Ok(Some(FileDateTime {
                    year: date.year() as u16,
                    month: date.month() as u8,
                    day: date.day() as u8,
                    hour: date.hour() as u8,
                    minute: date.minute() as u8,
                    second: date.second() as u8,
                    timestamp: date.timestamp() as i64,
                }));
            }
            Err(e) => log::debug!("DateTime try {} as {}, {}", date_str, fmt, e),
        }

        match Utc.datetime_from_str(&date_str, fmt) {
            Ok(dt) => {
                return Ok(Some(FileDateTime {
                    year: dt.year() as u16,
                    month: dt.month() as u8,
                    day: dt.day() as u8,
                    hour: dt.hour() as u8,
                    minute: dt.minute() as u8,
                    second: dt.second() as u8,
                    timestamp: dt.timestamp() as i64,
                }));
            }
            Err(e) => log::debug!("Utc try {} as {}, {}", date_str, fmt, e),
        }

        Ok(None)
    }

    async fn date_from_strptime(
        &self,
        date_str: &str,
        strptimes: &Vec<config::Strptime>,
    ) -> Result<FileDateTime> {
        for strptime in strptimes {
            match self.fuzzy_strptime(&date_str, &strptime.fmt).await {
                Ok(Some(dt)) => {
                    return Ok(dt);
                }
                _ => (),
            }
        }
        Err(anyhow!("parse {} failed", date_str))
    }

    async fn date_from_string(
        &self,
        value: &String,
        stripes: &Vec<config::Stripe>,
    ) -> Result<Option<FileDateTime>> {
        for stripe in stripes {
            if !value.contains(&stripe.name) {
                continue;
            }

            let date_string = self.regex_text_value(value, &stripe.regex, 1)?;
            // found date string, parse it
            let fdt = self
                .date_from_strptime(&date_string, &stripe.strptimes)
                .await?;
            log::debug!("[+] {} -> {}", date_string, fdt.to_string());
            return Ok(Some(fdt));
        }

        Ok(None)
    }

    async fn date_from_metedata(
        &mut self,
        texts: &HashSet<String>,
        config: &config::Config,
    ) -> Result<Vec<FileDateTime>> {
        let mut file_dts: Vec<FileDateTime> = Vec::new();

        if texts.len() == 0 {
            log::error!("no metadata found for {}", self.file_path.display());
            return Ok(file_dts);
        }

        'outer: for value in texts {
            log::debug!("{}", value);
            for black_str in &config.blacklist {
                if value.contains(black_str) {
                    log::debug!("[!] {} contains black string {}, skip...", value, black_str);
                    continue 'outer;
                }
            }

            // get file type from metadata
            if self.suffix.is_empty() {
                if let Some(file_type) = self.type_from_metadata(value).await? {
                    log::info!("[+] extractor {} from file metadata file type.", file_type);
                    self.set_suffix(&file_type.to_lowercase());
                }
            }
            // get date from metadata
            let parsed = match self.date_from_string(value, &config.stripes).await? {
                Some(dt) => dt,
                None => continue 'outer,
            };
            log::info!("[+] extractor {} from file metadata.", parsed.to_string());
            if parsed.get_year() < 1975 {
                log::warn!("[!] {} < 1975, skip...", parsed.get_year());
            } else {
                file_dts.push(parsed);
            }
        }
        Ok(file_dts)
    }

    fn earliest_from_attributes(&self) -> Result<FileDateTime> {
        let metadata = std::fs::metadata(&self.file_path)?;
        let atime = FileTime::from_last_access_time(&metadata).seconds();
        let mtime = FileTime::from_last_modification_time(&metadata).seconds();
        let ctime = match FileTime::from_creation_time(&metadata) {
            Some(v) => v.seconds(),
            None => {
                // log::debug!("not all Unix platforms have this field available");
                mtime
            }
        };

        let min_timestamp = match vec![atime, mtime, ctime].iter().min() {
            Some(t) => *t,
            None => return Err(anyhow!("get min timestamp failed")),
        };
        let dt = Utc.timestamp_opt(min_timestamp, 0).unwrap();

        Ok(FileDateTime {
            year: dt.year() as u16,
            month: dt.month() as u8,
            day: dt.day() as u8,
            hour: dt.hour() as u8,
            minute: dt.minute() as u8,
            second: dt.second() as u8,
            timestamp: dt.timestamp() as i64,
        })
    }

    async fn date_from_filename(&self, stripe: &config::Stripe) -> Result<Option<FileDateTime>> {
        let file_name = match self.file_path.file_name() {
            Some(s) => match s.to_os_string().into_string() {
                Ok(s) => s,
                Err(_) => {
                    log::warn!(
                        "{} file name is not utf8 string, ignore it!",
                        self.file_path.display()
                    );
                    return Ok(None);
                }
            },
            None => return Ok(None),
        };

        let date_string = self.regex_text_value(&file_name, &stripe.regex, 1);
        match date_string {
            // found date string, parse it
            Ok(s) => {
                let fdt = self.date_from_strptime(&s, &stripe.strptimes).await?;
                log::debug!("[+] {} -> {}", s, fdt.to_string());
                return Ok(Some(fdt));
            }
            _ => {} // Err(e) => {
                    //     log::debug!("try {} as {}, {}", file_name, stripe.regex, e);
                    // }
        }

        Ok(None)
    }

    async fn date_from_additional(
        &self,
        additional: &Vec<config::Stripe>,
    ) -> Result<Option<FileDateTime>> {
        for stripe in additional {
            if stripe.name == "filename" {
                return self.date_from_filename(stripe).await;
            }
        }
        Ok(None)
    }

    async fn type_from_metadata(&self, text: &String) -> Result<Option<String>> {
        // for check_str in vec!["Expected File Name Extension", "Detected File Type Name"] {
        for check_str in vec!["Expected File Name Extension"] {
            if text.contains(check_str) {
                return Ok(Some(self.regex_text_value(
                    text,
                    &format!("{} = (.*)", check_str),
                    1,
                )?));
            }
        }
        Ok(None)
    }

    pub async fn process(
        mut self,
        config: &config::Config,
        extractor: &MetadataReader,
    ) -> Result<FileMeta> {
        let mut file_dts: Vec<FileDateTime> = Vec::new();

        let readers = extractor.read(&self.file_path).await?;
        let dts = self.date_from_metedata(&readers, &config).await?;
        file_dts.extend(dts);

        let earliest = self.earliest_from_attributes()?;
        log::info!(
            "[+] extractor {} from file attributes.",
            earliest.to_string()
        );
        file_dts.push(earliest);

        if !config.additionals.is_empty() {
            match self.date_from_additional(&config.additionals).await? {
                Some(dt) => {
                    log::info!("[+] extractor {} from additional.", dt.to_string());
                    file_dts.push(dt);
                }
                None => {}
            }
        }

        let min_datetime = match file_dts.iter().min_by_key(|o| o.get_timestamp()) {
            Some(dt) => dt.to_owned(),
            None => {
                return Err(anyhow!(
                    "minimum datetime not found in {}",
                    self.file_path.display()
                ));
            }
        };

        if self.suffix.is_empty() {
            let suffix = match self.file_path.extension() {
                Some(s) => s.to_str().unwrap().to_owned().to_lowercase(),
                None => "".to_string(),
            };
            self.set_suffix(&suffix);
            log::debug!(
                "file type not found, set it to {} from file name.",
                self.suffix
            );
        }

        log::info!(
            "[+] minimum datetime {} found in {}",
            min_datetime.to_string(),
            self.file_path.display()
        );

        self.set_datetime(min_datetime);

        return Ok(self);
    }
}
