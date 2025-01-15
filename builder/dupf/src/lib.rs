use anyhow::{Ok, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

use place::check::Checker;
use utils::crypto::get_file_md5;

fn get_dup_files(input: &PathBuf) -> Result<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for entry in WalkDir::new(input) {
        let path = entry?.path().to_path_buf();
        let checker = Checker::new(&path);
        if checker.is_ignore() {
            log::debug!("🙈 skip file: {:?}", path);
            continue;
        }
        let md5 = get_file_md5(&path).unwrap();
        if map.contains_key(&md5) {
            log::info!("🔊 duplicate file {}: {:?}", md5, path);
            map.get_mut(&md5)
                .unwrap()
                .push(checker.path_str.to_string());
        } else {
            map.insert(md5, vec![checker.path_str.to_string()]);
        }
    }
    // only keep the vec with length > 1
    map.retain(|_, v| v.len() > 1);
    Ok(map)
}

pub fn process(input: &PathBuf, _output: &Option<PathBuf>) {
    let dups = get_dup_files(input).unwrap();
    log::info!("duplicate file: {:#?}", dups);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_root() -> PathBuf {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }
    #[test]
    fn test_get_dup_files() {
        let path = get_root().join("tests");
        let dup = get_dup_files(&path).unwrap();
        println!("dup: {:#?}", dup);
        // assert_eq!(total, 3);
    }
}
