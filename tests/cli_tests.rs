use std::process::Command;
use tempfile::NamedTempFile;

fn trod(db: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trod"));
    cmd.arg("--db").arg(db);
    cmd
}

#[test]
fn test_add_and_list() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    let output = trod(db).args(["add", "/tmp"]).output().unwrap();
    assert!(output.status.success());

    let output = trod(db).arg("list").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("/tmp"));
}

#[test]
fn test_back() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    trod(db).args(["add", "/first"]).output().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    trod(db).args(["add", "/second"]).output().unwrap();

    let output = trod(db).args(["back", "1"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().contains("/first"));
}

#[test]
fn test_forget() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    trod(db).args(["add", "/tmp"]).output().unwrap();
    trod(db).args(["forget", "/tmp"]).output().unwrap();

    let output = trod(db).arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("/tmp"));
}

#[test]
fn test_list_json() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    trod(db).args(["add", "/tmp"]).output().unwrap();

    let output = trod(db).args(["list", "--json"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"path\":\"/tmp\""));
}

#[test]
fn test_init_bash() {
    let output = Command::new(env!("CARGO_BIN_EXE_trod"))
        .args(["init", "bash"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("PROMPT_COMMAND"));
    assert!(stdout.contains("td()"));
}

#[test]
fn test_init_zsh() {
    let output = Command::new(env!("CARGO_BIN_EXE_trod"))
        .args(["init", "zsh"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("chpwd"));
    assert!(stdout.contains("td()"));
}

#[test]
fn test_print_back() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    trod(db).args(["add", "/first"]).output().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    trod(db).args(["add", "/second"]).output().unwrap();

    let output = trod(db).args(["--print-back", "1"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().contains("/first"));
}

#[test]
fn test_init_shell_has_cd_wrapper() {
    let output = Command::new(env!("CARGO_BIN_EXE_trod"))
        .args(["init", "zsh"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--print-back"));
    assert!(stdout.contains("cd \"$dir\""));
}
