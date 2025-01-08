use anyhow::Result;
use filetime::{set_file_times, FileTime};
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;

use config::CONFIG;
use meta::META;
use target::{Checker, Target};

mod meta;
// pub mod pick;
// mod entry;
mod target;

fn get_total_size(path: &PathBuf) -> usize {
    WalkDir::new(path)
        .into_iter()
        .filter(|e| {
            let p = e.as_ref().unwrap().path().to_path_buf();
            !Checker::new(&p).is_skip()
        })
        .count()
}

pub async fn do_place(input: &PathBuf, output: &PathBuf, test: bool) -> Result<()> {
    let total = get_total_size(input);

    log::info!(
        "start process with:\n  input: {:?}\n  output: {:?}\n  test: {}\n  total: {}",
        input,
        output,
        test,
        total
    );

    let mut index = 0;
    let mut handles = Vec::new();

    for entry in WalkDir::new(input) {
        let path = entry?.path().to_path_buf();
        let checker = Checker::new(&path);
        if checker.is_skip() {
            log::info!("skip file: {:?}", path);
            continue;
        }
        index += 1;

        handles.push(tokio::spawn(async move {
            Target::new(&path).process(index, total).await
        }));

        if handles.len() >= CONFIG.batch_size {
            for handle in handles.iter_mut() {
                let target = handle.await??;
            }
            handles.clear();
        }
    }

    //     let _config = Arc::clone(&config);
    //     let _reader = Arc::clone(&mreader);
    //     handles.push(tokio::spawn(async move {
    //         let pf = PickFile::new(&entry.path().to_path_buf(), index, total);
    //         return pf.create(&_config.parser, &_reader).await.unwrap();
    //         // process_one(pf, &_config, &_reader, test).await.unwrap();
    //     }));
    //     // if handles len is bigger than config.parser.batch_size, wait for all handles done and clear it
    //     if handles.len() as u32 >= config.parser.batch_size {
    //         for handle in handles.iter_mut() {
    //             let pf = handle.await?;
    //             do_copy(pf, &config.output, config.parser.dup_max, test)?;
    //         }
    //         handles.clear();
    //     }
    // }

    // // wait for all handles done
    // if handles.len() > 0 {
    //     for handle in handles {
    //         let pf = handle.await?;
    //         do_copy(pf, &config.output, config.parser.dup_max, test)?;
    //     }
    // }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_total_size() {
        let path = PathBuf::from("/tmp/123");
        let total = get_total_size(&path);
        println!("total: {}", total);
        // assert_eq!(total, 3);
    }
}
