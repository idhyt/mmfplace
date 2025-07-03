use rusqlite::{Connection, Result};
use serde_json::json;
use std::path::Path;

struct FileHash<'a, 'b> {
    parts: Vec<&'a str>,
    hash: &'b str,
}

pub fn db_init(p: &Path) -> Result<Connection> {
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

pub fn insert_hash(conn: &Connection, fh: &FileHash) -> Result<usize> {
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

    fn get_db_path() -> PathBuf {
        let p = PathBuf::from("test.db");
        if p.exists() {
            std::fs::remove_file(&p).unwrap();
        }
        p
    }

    #[test]
    fn test_insert() {
        let p = get_db_path();
        let conn = db_init(&p).unwrap();
        println!("conn: {:#?}", conn);

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

    #[test]
    fn test_insert_hash() {
        let tests = [
            // json!({"parts": ["path", "to", "file1"], "hash": "hash1"}),
            // json!({"parts": ["path", "to", "file1"], "hash": "hash1"}),
            FileHash {
                parts: vec!["path", "to", "file1"],
                hash: "hash1",
            },
            FileHash {
                parts: vec!["path", "to", "file2"],
                hash: "hash2",
            },
        ];
        let p = get_db_path();
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

    #[test]
    fn test_query_parts() {
        let p = get_db_path();
        let conn = db_init(&p).unwrap();
        let test = FileHash {
            parts: vec!["path", "to", "file1"],
            hash: "hash1",
        };
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
}
