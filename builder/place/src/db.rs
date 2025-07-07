use rusqlite::{Connection, Result};
use serde::Serialize;
use serde_json::json;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tracing::info;

pub struct FileHash<'a, 'b, T: AsRef<str> + 'a> {
    pub parts: &'a [T],
    pub hash: &'b str,
}

static DATABASE: OnceLock<Mutex<Connection>> = OnceLock::new();

pub fn get_connection() -> &'static Mutex<Connection> {
    DATABASE.get_or_init(|| {
        let mut work_dir = std::env::current_exe().unwrap();
        work_dir.pop();
        let path = work_dir.join("place.db");
        Mutex::new(db_init(&path).unwrap())
    })
}

pub fn db_init(p: &Path) -> Result<Connection> {
    info!(path = ?p, "Loading database once");
    let conn = Connection::open(p)?;
    // 创建表（如果不存在）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS data (
            id INTEGER PRIMARY KEY,
            parts TEXT NOT NULL,    -- json list
            hash TEXT NOT NULL UNIQUE
        )",
        [], // 无参数
    )?;
    // 创建索引
    conn.execute("CREATE INDEX IF NOT EXISTS idx_hash ON data (hash)", [])?;
    Ok(conn)
}

pub fn insert(conn: &Connection, parts: &str, hash: &str) -> Result<usize> {
    conn.execute(
        "INSERT INTO data (parts, hash) VALUES (?, ?)",
        [parts, hash],
    )
}

pub fn insert_hash<'a, T>(conn: &Connection, fh: &FileHash<'a, '_, T>) -> Result<usize>
where
    T: AsRef<str> + 'a + Serialize,
{
    insert(conn, &json!(fh.parts).to_string(), fh.hash)
}

pub fn query_parts(conn: &Connection, hash: &str) -> Result<Option<Vec<String>>> {
    let mut stmt = conn.prepare("SELECT parts FROM data WHERE hash = ?")?;
    let mut rows = stmt.query([hash])?;
    if let Some(row) = rows.next()? {
        let parts: String = row.get(0)?;
        return Ok(Some(serde_json::from_str(&parts).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?));
    }
    Ok(None)
}

// test
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
            ),
            (
                json!(["tmp", "test2", "file2"]).to_string(),
                "hash2".to_string(),
            ),
        ];
        {
            let conn = db_init(&p).unwrap();
            println!("conn: {:#?}", conn);

            for (parts, hash) in data.iter() {
                let r = conn.execute(
                    "INSERT INTO data (parts, hash) VALUES (?, ?)",
                    [parts, hash],
                );
                println!("insert: {:#?}", r);
                assert!(r.is_ok());
                assert!(r.unwrap() == 1);
            }
            for (parts, hash) in data.iter() {
                let r = conn.execute(
                    "INSERT INTO data (parts, hash) VALUES (?, ?)",
                    [parts, hash],
                );
                println!("insert: {:#?}", r);
                assert!(r.is_err());
                assert!(r
                    .unwrap_err()
                    .to_string()
                    .contains("UNIQUE constraint failed: data.hash"));
            }
        }
        std::fs::remove_file(p).unwrap();
    }

    #[test]
    fn test_insert_hash() {
        let tests = [
            // json!({"parts": ["path", "to", "file1"], "hash": "hash1"}),
            // json!({"parts": ["path", "to", "file1"], "hash": "hash1"}),
            FileHash {
                parts: &vec!["path", "to", "file1"],
                hash: "hash1",
            },
            FileHash {
                parts: &vec!["path", "to", "file2"],
                hash: "hash2",
            },
        ];
        let p = get_db_path("test_insert_hash.db");
        {
            let conn = db_init(&p).unwrap();
            for test in tests.iter() {
                let r = insert_hash(&conn, &test);
                println!("insert: {:#?}", r);
                assert!(r.is_ok());
                assert!(r.unwrap() == 1);
            }
            for test in tests.iter() {
                let r = insert_hash(&conn, &test);
                println!("insert: {:#?}", r);
                assert!(r.is_err());
                assert!(r
                    .unwrap_err()
                    .to_string()
                    .contains("UNIQUE constraint failed: data.hash"));
            }
        }

        std::fs::remove_file(p).unwrap();
    }

    #[test]
    fn test_query_parts() {
        let p = get_db_path("test_query_parts.db");

        let test = FileHash {
            parts: &vec!["path", "to", "file1"],
            hash: "hash1",
        };
        {
            let conn = db_init(&p).unwrap();
            let r = insert_hash(&conn, &test);
            println!("insert: {:#?}", r);
            assert!(r.is_ok());
            assert!(r.unwrap() == 1);

            let r = query_parts(&conn, "hash1");
            println!("query: {:#?}", r);
            assert!(r.is_ok());
            assert!(r.unwrap().unwrap().len() == 3);

            let r = query_parts(&conn, "hash2");
            println!("query: {:#?}", r);
            assert!(r.is_ok());
            assert!(r.unwrap().is_none());
        }

        std::fs::remove_file(p).unwrap();
    }
}
