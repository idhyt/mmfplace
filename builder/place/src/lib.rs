use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

mod db;
mod process;
mod target;

pub async fn process(input: &PathBuf, output: &Option<PathBuf>, test: bool) -> Result<()> {
    let output = if let Some(o) = output {
        o.canonicalize()?
    } else {
        input.with_extension("mmfplace").canonicalize()?
    };
    if !output.is_dir() {
        std::fs::create_dir_all(&output)?;
    }
    let total = walkdir::WalkDir::new(&input)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();
    info!(input=?input, total=total, output=?output, test=test, "start process");
    // init temp data
    process::temp_init(input.to_path_buf(), output, test);
    process::do_process().await
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
}

/*
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
    fn test_target_new() {
        let path = get_root().join("tests/simple.jpg");
        println!("path: {:?}", path);
        let target = Target::new(&path, 1, 1);
        println!("{}", target);
        assert_eq!(target.path, path);
        assert_eq!(target.extension, "jpg");
    }

    #[test]
    fn test_target_get_name() {
        let path = get_root().join("tests/simple.jpg");
        let target = Target::new(&path, 1, 1);
        let name = target.get_name();
        println!("target: {}, name: {}", target, name);
        let check = format!("{}.{}.jpg", target.datetime.second, target.hash);
        println!("check: {}", check);
        assert!(name.contains(&check));
        let name = target.get_name();
        println!("target: {}, name: {}", target, name);
        assert!(name.contains("a18932e314dbb4c81c6fd0e282d81d16.jpg"));
    }

    #[tokio::test]
    async fn test_date_from_metedata() {
        let path = get_root().join("tests/simple.jpg.png");
        let mut target = Target::new(&path, 1, 1);
        let dts = target.datetime_from_metedata().await.unwrap();
        println!("dts: {:?}", dts);
        assert!(dts.len() == 4);
        println!("target: {:#?}", target);
        assert!(target.extension == "png");
        assert!(target.suffix == Some("jpg".to_string()));
    }

    #[tokio::test]
    async fn test_get_all_datetime() {
        let path = get_root().join("tests/simple.jpg.png");
        let mut target = Target::new(&path, 1, 1);
        let dts = target.get_all_datetime(false).await;
        println!("dts: {:#?}", dts);
        assert!(dts.len() == 5);
        let mut sorts = vec![];
        for index in 0..dts.len() - 1 {
            if dts[index].timestamp < dts[index + 1].timestamp {
                sorts.push(true);
            } else {
                sorts.push(false);
            }
        }
        println!("sorts: {:?}", sorts);
        assert!(!sorts.iter().all(|x| *x));

        let dts = target.get_all_datetime(true).await;
        println!("dts: {:#?}", dts);
        assert!(dts.len() == 3);
        assert!(dts[0].timestamp < dts[1].timestamp);
        assert!(dts[1].timestamp < dts[2].timestamp);
    }

    #[tokio::test]
    async fn test_process() {
        let path = get_root().join("tests/simple.jpg");
        let output = get_root().join("tests/output");
        let target = Target::new(&path, 1, 1).process(None).await.unwrap();
        println!("target: {:#?}", target);
        assert!(target.datetime.timestamp == 1037460421);

        let dst = target.copy(&output);
        println!("copy from {:?} to {:?}", &path, &dst);
        assert!(dst.is_file());

        let src_meta = std::fs::metadata(&path).unwrap();
        let dst_meta = std::fs::metadata(&dst).unwrap();
        println!("src_meta: {:#?}", src_meta);
        println!("dst_meta: {:#?}", dst_meta);

        // std::fs::remove_dir_all(&output).unwrap();
    }
}
*/
