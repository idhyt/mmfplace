pub mod metadata;
pub mod parser;

// #[cfg(test)]
// mod tests {
//     use super::parser::{FileDateTime, FileMeta};
//     use config::config;

//     async fn strptime(text: &str, fmt: &str) -> Option<FileDateTime> {
//         let fm = FileMeta::new(std::path::PathBuf::from("tests/none.jpeg"));
//         return fm.fuzzy_strptime(text, fmt).await.unwrap();
//     }

//     #[test]
//     fn test_parse() {
//         let config = config::Config::new();
//         // cargo test -- --nocapture
//         println!("config: {:#?}", config);
//         let dateparse = &config.stripes[0].strptimes;
//         for dp in dateparse {
//             println!("dp: {:#?}", dp);
//             let result = futures::executor::block_on(strptime(&dp.test, &dp.fmt)).unwrap();
//             println!("result: {:#?}", result);
//         }
//     }
// }
