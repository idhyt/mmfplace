use digest::DynDigest;
use std::io::{BufReader, Read};
use std::path::PathBuf;

// DynDigest needs to be boxed here, since function return should be sized.
fn select_hasher(s: &str) -> Box<dyn DynDigest> {
    match s {
        // cargo add md-5
        "md5" => Box::new(md5::Md5::default()),
        // "sha1" => Box::new(sha1::Sha1::default()),
        // "sha224" => Box::new(sha2::Sha224::default()),
        // cargo add sha2
        // "sha256" => Box::new(sha2::Sha256::default()),
        // "sha384" => Box::new(sha2::Sha384::default()),
        // "sha512" => Box::new(sha2::Sha512::default()),
        _ => unimplemented!("unsupported digest: {}", s),
    }
}

type HasherRet<T> = std::result::Result<T, std::io::Error>;

fn file_hasher(hasher: &str, path: &PathBuf) -> HasherRet<String> {
    let input = std::fs::File::open(path)?;
    let mut reader = BufReader::new(input);

    let digest = {
        let mut hasher = select_hasher(hasher);
        let mut buffer = [0; 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize_reset()
    };
    Ok(base16ct::lower::encode_string(&digest))
}

pub fn get_file_md5(path: &PathBuf) -> HasherRet<String> {
    file_hasher("md5", path)
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
    fn test_md5() {
        let path = get_root().join("tests/2002/11/simple.jpg");
        let md5 = get_file_md5(&path).unwrap();
        println!("md5: {}", md5);
        assert_eq!(md5, "a18932e314dbb4c81c6fd0e282d81d16");
    }
}
