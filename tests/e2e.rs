use std::process::Command;

fn run_check_test(file_path: &str, expected_counts: &str) {
    let output = Command::new("cargo")
        .args(["run", "--", "check", file_path])
        .output()
        .expect("Failed to run omnitype check");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that it found functions and classes
    assert!(stdout.contains(expected_counts), "Expected function/class count in output");

    // Check for warnings about missing annotations
    assert!(stdout.contains("warning Missing"), "Expected warnings about missing annotations");

    // Should exit with code 1 due to diagnostics
    assert!(!output.status.success(), "Expected non-zero exit code for diagnostics");
}

#[test]

fn test_check_sample_py() {
    run_check_test("tests/sample.py", "functions=8, classes=1");
}

#[test]

fn test_check_classes_py() {
    run_check_test("tests/classes.py", "functions=7, classes=1");
}
