use anyhow::Result;
use sha2::{Digest, Sha256};
use std::io::{BufReader, Read};
use std::path::Path;

/// calculates sha256 digest as lowercase hex string
pub fn sha256_digest(path: impl AsRef<Path>) -> Result<String> {
    let input = std::fs::File::open(path)?;
    let mut reader = BufReader::new(input);

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    let hex_hash = base16ct::lower::encode_string(&digest);
    Ok(hex_hash)
}
