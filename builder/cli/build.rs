use std::path::PathBuf;
use std::process::Command;

const VERSION: &str = "0.3.0";

fn get_version() -> String {
    // 获取 commit hash 前10位
    let commit_hash = Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        //.unwrap_or_else(|_| "unknown".to_string());
        .expect("Failed to get commit hash");
    assert!(!commit_hash.is_empty(), "commit hash is empty");

    // 获取最后一次提交日期 (ISO 格式 YYYY-MM-DD)
    let commit_date = Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=short"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .expect("Failed to get commit date");
    assert!(!commit_date.is_empty(), "commit date is empty");

    format!("{} ({} {})", VERSION, commit_hash, commit_date)
}

fn main() {
    // if cross compiling, unsupport get version because of no git
    if std::env::var("CROSS_SYSROOT").is_ok() {
        println!("warning: cross compiling, unsupport get version");
        return;
    }

    let version = get_version();
    println!("build version: {version}");

    let build_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let cli_path = build_dir.join("src").join("main.rs");
    assert!(cli_path.is_file());

    let version_code = format!("    version = \"{}\",", version);
    let content = std::fs::read_to_string(&cli_path).unwrap();
    let mut lines: Vec<&str> = content.lines().map(|s| s).collect();
    for i in 0..lines.len() {
        if lines[i].contains("version") {
            lines[i] = &version_code;
            break;
        }
    }
    std::fs::write(cli_path, lines.join("\n") + "\n").unwrap();
    println!("success patch consts version to {version}");
}
