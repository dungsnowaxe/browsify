use std::{fs, process::Command};

#[test]
fn legacy_rules_are_import_inputs_only() {
    let home = tempfile::tempdir().unwrap();
    let primary = home.path().join(".config/browsify/config.toml");
    fs::create_dir_all(primary.parent().unwrap()).unwrap();
    fs::write(&primary, "default_browser = 'Safari'\n").unwrap();
    let legacy = browsify_core::RulesFile::default_path(home.path());
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(legacy, r#"{"rules":[{"match":"*","browser":"Firefox"}]}"#).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_browsify"))
        .env("HOME", home.path())
        .args(["test", "https://example.invalid/"])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Safari"));
}

#[test]
fn migration_reports_individual_failures_and_requires_a_static_default() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".finicky.js");
    let source = r#"export default {
        defaultBrowser: () => 'Safari',
        handlers: [
            {match: '*github.com/*', browser: 'Firefox'},
            {match: '*work.test/*', browser: () => 'Safari'},
            {match: /(?<=foo)bar/, browser: 'Safari'}
        ]
    }"#;
    fs::write(&config, source).unwrap();
    let output = home.path().join("imported.toml");
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_browsify"))
            .env("HOME", home.path())
            .args(["migrate", "--output"])
            .arg(&output)
            .args(extra)
            .output()
            .unwrap()
    };
    let check = run(&["--check"]);
    assert!(check.status.success());
    let report = String::from_utf8(check.stderr).unwrap();
    assert!(report.contains("1 rule imported, 2 failed"), "{report}");
    assert!(
        report.contains("Handler 2")
            && report.contains("*work.test/*")
            && report.contains("Handler 3"),
        "{report}"
    );
    assert!(!run(&[]).status.success());
    assert!(!output.exists());
    assert!(run(&["--default-browser", "Safari"]).status.success());
    let toml =
        browsify_core::TomlConfig::from_toml_str(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(toml.handlers.len(), 1);
    assert_eq!(fs::read_to_string(config).unwrap(), source);
}
