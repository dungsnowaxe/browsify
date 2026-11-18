use std::process::Command;

#[test]
fn dia_dry_run_preserves_native_target_through_both_js_paths() {
    let fixture = tempfile::tempdir().unwrap();
    let config = fixture.path().join("config.js");
    let rules = fixture.path().join("rules.json");
    std::fs::write(&rules, "{}").unwrap();
    let target = serde_json::json!({
        "name": "Dia",
        "profile": "Uninstalled test profile \"Δ\"",
        "openInBackground": true
    });
    for expression in [target.to_string(), format!("() => ({target})")] {
        std::fs::write(
            &config,
            format!("export default {{ defaultBrowser: {expression} }};"),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_browsify"))
            .args([
                "open",
                "https://example.invalid/a?x=%22&y=1",
                "--dry-run",
                "--config",
            ])
            .arg(&config)
            .arg("--rules")
            .arg(&rules)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("Dia native profile"), "{stdout}");
        assert!(
            stdout.contains("Uninstalled test profile \"Δ\""),
            "{stdout}"
        );
        assert!(!stdout.contains("--profile-directory"), "{stdout}");
        assert!(output.stderr.is_empty());
    }
}
