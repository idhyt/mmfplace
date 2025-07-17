use rusqlite::{Connection, Result};
use serde::Serialize;
use serde_json::json;
use std::borrow::Cow;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tracing::{info, warn};

// pub struct FileInfo<'a, 'b, T: AsRef<str> + 'a> {
//     pub parts: &'a [T],
//     pub hash: &'b str,
//     // the DateTime<Local> timestamp
//     pub earliest: i64,
// }

#[derive(Debug)]
pub struct FileInfo<'a, T: AsRef<str> + Clone + ToOwned + 'static> {
    // pub parts: Vec<Cow<'static, T>>,
    pub parts: Cow<'a, [T]>,
    pub hash: Cow<'a, str>,
    // the DateTime<Local> timestamp
    pub earliest: i64,
}

static DATABASE: OnceLock<Mutex<Connection>> = OnceLock::new();

pub fn get_connection() -> &'static Mutex<Connection> {
    DATABASE.get_or_init(|| {
        let path = config::CONFIG.database.as_ref().unwrap();
        Mutex::new(db_init(path).unwrap())
    })
}

pub fn db_init(p: &Path) -> Result<Connection> {
    if p.is_file() {
        info!(file=?p, "Loading Database exists and using it");
    } else {
        warn!(file=?p, "Loading Database not found, creating a new one");
    }
    let conn = Connection::open(p)?;
    // 创建表（如果不存在）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS data (
            id INTEGER PRIMARY KEY,
            parts TEXT NOT NULL,    -- json list
            earliest INTEGER NOT NULL,
            hash TEXT NOT NULL UNIQUE
        )",
        [], // 无参数
    )?;
    // 创建索引
    conn.execute("CREATE INDEX IF NOT EXISTS idx_hash ON data (hash)", [])?;
    Ok(conn)
}

fn insert(conn: &Connection, parts: &str, hash: &str, earliest: i64) -> Result<usize> {
    conn.execute(
        "INSERT INTO data (parts, hash, earliest) VALUES (?, ?, ?)",
        rusqlite::params![parts, hash, earliest],
    )
}

fn query<'a>(conn: &Connection, hash: &str) -> Result<Option<FileInfo<'a, String>>> {
    let mut stmt = conn.prepare("SELECT parts, hash, earliest FROM data WHERE hash = ?")?;
    let mut rows = stmt.query([hash])?;
    if let Some(row) = rows.next()? {
        let parts_json: String = row.get(0)?;
        let hash: String = row.get(1)?;
        let earliest: i64 = row.get(2)?;

        let parts: Vec<String> = serde_json::from_str(&parts_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Some(FileInfo {
            parts: Cow::Owned(parts),
            hash: Cow::Owned(hash),
            earliest,
        }))
    } else {
        Ok(None)
    }
}

// fn update<T>(conn: &Connection, hash: &str, parts: &[T], earliest: i64) -> Result<usize>
// where T:AsRef<str> + Clone + ToOwned + 'static + Serialize
fn update(conn: &Connection, hash: &str, parts: &str, earliest: i64) -> Result<usize> {
    // let parts = serde_json::to_string(parts).map_err(|e| {
    //             rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    //         })?;
    let mut stmt = conn.prepare("UPDATE data SET parts = ?, earliest = ? WHERE hash = ?")?;
    stmt.execute(rusqlite::params![parts, earliest, hash])
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

pub fn insert_finfo<T>(conn: &Connection, fh: &FileInfo<T>) -> Result<usize>
where
    T: AsRef<str> + 'static + Serialize + Clone,
{
    insert(
        conn,
        &json!(fh.parts).to_string(),
        fh.hash.as_ref(),
        fh.earliest,
    )
}

pub fn query_finfo<'a>(conn: &Connection, hash: &str) -> Result<Option<FileInfo<'a, String>>> {
    query(conn, hash)
}

pub fn update_finfo<T>(conn: &Connection, finfo: &FileInfo<T>) -> Result<usize>
where
    T: AsRef<str> + 'static + Serialize + Clone,
{
    let parts = serde_json::to_string(&finfo.parts).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    update(conn, &finfo.hash, &parts, finfo.earliest)
}

// test
#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use serde_json::json;

    fn get_db_path(n: &str) -> PathBuf {
        let p = PathBuf::from(n);
        if p.exists() {
            std::fs::remove_file(&p).unwrap();
        }
        p
    }

    #[test]
    fn test_insert() {
        let p = get_db_path("test_insert.db");
        let data = [
            (
                json!(["tmp", "test1", "file1"]).to_string(),
                "hash1".to_string(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
            (
                json!(["tmp", "test2", "file2"]).to_string(),
                "hash2".to_string(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 60 * 60 * 24,
            ),
        ];
        {
            let conn = db_init(&p).unwrap();
            println!("conn: {:#?}", conn);

            for (parts, hash, timestamp) in data.iter() {
                let r = insert(&conn, &parts, &hash, *timestamp as i64);
                println!("insert: {:#?}", r);
                assert!(r.is_ok());
                assert!(r.unwrap() == 1);
            }
            for (parts, hash, timestamp) in data.iter() {
                let r = insert(&conn, &parts, &hash, *timestamp as i64);
                println!("insert: {:#?}", r);
                assert!(r.is_err());
                assert!(
                    r.unwrap_err()
                        .to_string()
                        .contains("UNIQUE constraint failed: data.hash")
                );
            }
        }
        std::fs::remove_file(p).unwrap();
    }

    #[test]
    fn test_insert_finfo() {
        let (parts1, parts2) = (vec!["path", "to", "file1"], vec!["path", "to", "file2"]);
        let tests = [
            // json!({"parts": ["path", "to", "file1"], "hash": "hash1"}),
            // json!({"parts": ["path", "to", "file1"], "hash": "hash1"}),
            FileInfo {
                parts: Cow::Borrowed(&parts1),
                hash: Cow::Borrowed("hash1"),
                earliest: 0,
            },
            FileInfo {
                parts: Cow::Borrowed(&parts2),
                hash: Cow::Borrowed("hash2"),
                earliest: 0,
            },
        ];
        let p = get_db_path("test_insert_finfo.db");
        {
            let conn = db_init(&p).unwrap();
            for test in tests.iter() {
                let r = insert_finfo(&conn, &test);
                println!("insert: {:#?}", r);
                assert!(r.is_ok());
                assert!(r.unwrap() == 1);
            }
            for test in tests.iter() {
                let r = insert_finfo(&conn, &test);
                println!("insert: {:#?}", r);
                assert!(r.is_err());
                assert!(
                    r.unwrap_err()
                        .to_string()
                        .contains("UNIQUE constraint failed: data.hash")
                );
            }
        }

        std::fs::remove_file(p).unwrap();
    }

    #[test]
    fn test_query() {
        let p = get_db_path("test_query.db");
        let parts = vec!["path", "to", "file1"];
        let test = FileInfo {
            parts: Cow::Borrowed(&parts),
            hash: Cow::Borrowed("hash1"),
            earliest: 123,
        };
        {
            let conn = db_init(&p).unwrap();
            let r = insert_finfo(&conn, &test);
            println!("insert: {:#?}", r);
            assert!(r.is_ok());
            assert!(r.unwrap() == 1);

            let r = query_finfo(&conn, "hash1");
            println!("query: {:#?}", r);
            assert!(r.is_ok());
            let r = r.unwrap().unwrap();
            assert!(r.parts.len() == 3);
            assert!(r.hash == "hash1");
            assert!(r.earliest == 123);

            let r = query_finfo(&conn, "hash2");
            println!("query: {:#?}", r);
            assert!(r.is_ok());
            assert!(r.unwrap().is_none());
        }

        std::fs::remove_file(p).unwrap();
    }

    #[test]
    fn test_update() {
        let p = get_db_path("test_update.db");
        let parts = vec!["path", "to", "file1"];
        let hash = "hash1";
        let earliest = 123;
        {
            let conn = db_init(&p).unwrap();
            let test = FileInfo {
                parts: Cow::Borrowed(&parts),
                hash: Cow::Borrowed(&hash),
                earliest: earliest,
            };
            let r = insert_finfo(&conn, &test);
            println!("insert: {:#?}", r);
            assert!(r.is_ok());
            assert!(r.unwrap() == 1);

            let mut test = test;
            let parts2 = vec!["path", "to", "file2"];
            test.earliest = 456;
            test.parts = Cow::Borrowed(&parts2);
            let r = update_finfo(&conn, &test);
            println!("update_finfo: {:#?}", r);
            assert!(r.unwrap() == 1);

            let r = query_finfo(&conn, &test.hash);
            println!("query_finfo: {:#?}", r);
            assert!(r.is_ok());
            let r = r.unwrap().unwrap();
            assert!(r.parts == parts2);
            assert!(r.earliest == 456);
        }

        std::fs::remove_file(p).unwrap();
    }
}
