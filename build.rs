use std::process::Command;
fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let commit_hash = String::from_utf8(output.stdout).unwrap();
    let output = Command::new("git")
        .args(&["show", "-s", "--format=%ci", "HEAD"])
        .output()
        .unwrap();
    let commit_time = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=HEN_COMMIT_HASH={}", commit_hash);
    println!("cargo:rustc-env=HEN_COMMIT_TIME={}", commit_time);
}
