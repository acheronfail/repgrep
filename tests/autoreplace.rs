// Cross runs this test binary under QEMU, but child processes are not automatically wrapped in
// QEMU. The unit suite still exercises the implementation on ARM.
#![cfg(not(target_arch = "arm"))]

use std::fs;
use std::process::{Command, Output, Stdio};

use tempfile::tempdir;

fn run_rgr(args: &[&str], current_dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rgr"))
        .args(args)
        .current_dir(current_dir)
        .stdin(Stdio::null())
        .output()
        .expect("failed to start repgrep")
}

fn ripgrep_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn autoreplaces_all_matches_without_a_terminal() {
    // Cross-compiled tests run inside minimal containers without ripgrep. Native CI installs it.
    if !ripgrep_available() {
        return;
    }

    let directory = tempdir().unwrap();
    let first = directory.path().join("first.txt");
    let second = directory.path().join("second.txt");
    fs::write(&first, "old_name old_name\n").unwrap();
    fs::write(&second, "old_name\n").unwrap();

    let output = run_rgr(
        &[
            "-y",
            "-r",
            "$first-$second",
            "(?P<first>old)_(?P<second>name)",
            "first.txt",
            "second.txt",
        ],
        directory.path(),
    );

    assert!(
        output.status.success(),
        "autoreplace failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(first).unwrap(), "old-name old-name\n");
    assert_eq!(fs::read_to_string(second).unwrap(), "old-name\n");
}

#[test]
fn autoreplace_supports_cached_json_results() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("input.txt");
    let json_path = directory.path().join("matches.json");
    fs::write(&path, "remove me\n").unwrap();

    fs::write(
        &json_path,
        r#"{"type":"match","data":{"path":{"text":"input.txt"},"lines":{"text":"remove me\n"},"line_number":1,"absolute_offset":0,"submatches":[{"match":{"text":"remove"},"start":0,"end":6}]}}
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_rgr"))
        .args(["--autoreplace", "--replace=", "remove"])
        .env("RGR_JSON_FILE", &json_path)
        .current_dir(directory.path())
        .stdin(Stdio::null())
        .output()
        .expect("failed to start repgrep");

    assert!(
        output.status.success(),
        "autoreplace failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(path).unwrap(), " me\n");
}

#[test]
fn autoreplace_requires_an_explicit_replacement() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("input.txt");
    fs::write(&path, "old_name\n").unwrap();

    let output = run_rgr(&["-y", "old_name", "input.txt"], directory.path());

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--autoreplace requires -r/--replace")
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "old_name\n");
}

#[test]
fn autoreplace_does_not_partially_modify_a_file_with_stale_cached_results() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("input.txt");
    let json_path = directory.path().join("matches.json");
    fs::write(&path, "old old\n").unwrap();

    fs::write(
        &json_path,
        r#"{"type":"match","data":{"path":{"text":"input.txt"},"lines":{"text":"old old\n"},"line_number":1,"absolute_offset":0,"submatches":[{"match":{"text":"old"},"start":0,"end":3},{"match":{"text":"old"},"start":4,"end":7}]}}
"#,
    )
    .unwrap();
    fs::write(&path, "old new\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_rgr"))
        .args(["-y", "-r", "updated", "old"])
        .env("RGR_JSON_FILE", &json_path)
        .current_dir(directory.path())
        .stdin(Stdio::null())
        .output()
        .expect("failed to start repgrep");

    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(path).unwrap(), "old new\n");
}

#[test]
fn help_documents_rgr_autoreplace_option() {
    let directory = tempdir().unwrap();
    let output = run_rgr(&["--help"], directory.path());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains("-y, --autoreplace"));
    assert!(stdout.contains("requires -r/--replace"));
}
