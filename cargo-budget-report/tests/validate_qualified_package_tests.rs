use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mock_workspace_fixture() -> PathBuf {
    manifest_dir().join("tests/fixtures/mock_workspace")
}

fn fake_bin_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures/fake_bin")
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("failed to create destination directory");
    for entry in fs::read_dir(src).expect("failed to read source directory") {
        let entry = entry.expect("failed to read directory entry");
        let file_type = entry.file_type().expect("failed to read file type");
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dst_path);
        } else {
            fs::copy(entry.path(), &dst_path).expect("failed to copy fixture file");
        }
    }
}

fn setup_mock_workspace() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    copy_dir_all(&mock_workspace_fixture(), tmp.path());
    tmp
}

fn mocked_path() -> String {
    let real_path = std::env::var("PATH").unwrap_or_default();
    format!("{}:{}", fake_bin_dir().display(), real_path)
}

#[test]
fn validate_qualified_package_mismatch_is_rejected() {
    let workspace = setup_mock_workspace();

    // 'swap' is exported by mock-contract-b, but we configure it under wrong-pkg::swap and mock-contract-a::swap
    fs::write(
        workspace.path().join("budget.toml"),
        "[functions.\"wrong-pkg::swap\"]\ncpu_limit = 1000\n\n[functions.\"mock-contract-a::swap\"]\ncpu_limit = 1000\n",
    )
    .expect("failed to write budget.toml");

    let mut cmd = Command::cargo_bin("cargo-budget-report").expect("binary should be built");
    cmd.current_dir(workspace.path()).env("PATH", mocked_path());
    let assert = cmd
        .args(["budget-report", "--network", "local", "--source", "alice"])
        .assert();

    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}");

    assert!(
        output.status.code() == Some(3) || !output.status.success(),
        "mismatched package qualification should fail validation: {combined}"
    );
    assert!(
        combined.contains("wrong-pkg") || combined.contains("mock-contract-a"),
        "error message should mention the mismatched or unknown package: {combined}"
    );
}
