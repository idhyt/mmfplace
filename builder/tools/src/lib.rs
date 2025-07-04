use std::collections::HashSet;
use std::io::Result;
use std::path::Path;

mod metadata;

pub async fn metadata_extractor(file: &Path) -> Result<HashSet<String>> {
    metadata::METADATA.read(file).await
}
