use std::process::Command;

#[test]
fn dia_dry_run_preserves_native_target_after_import() {
    let fixture = tempfile::tempdir().unwrap();
    let config = fixture.path().join("config.js");
    let imported = fixture.path().join("imported.toml");
    let target = serde_json::json!({
        "name": "Dia",
        "profile": "Uninstalled test profile \"Δ\"",
        "openInBackground": true
    });
    {
        let expression = target.to_string();
        std::fs::write(
            &config,
            format!("export default {{ defaultBrowser: {expression} }};"),
        )
        .unwrap();
        let migration = Command::new(env!("CARGO_BIN_EXE_browsify"))
            .env("HOME", fixture.path())
            .args(["migrate", "--config"])
            .arg(&config)
            .arg("--output")
            .arg(&imported)
            .output()
            .unwrap();
        assert!(
            migration.status.success(),
            "{}",
            String::from_utf8_lossy(&migration.stderr)
        );
        let output = Command::new(env!("CARGO_BIN_EXE_browsify"))
            .env("HOME", fixture.path())
            .args([
                "open",
                "https://example.invalid/a?x=%22&y=1",
                "--dry-run",
                "--config",
            ])
            .arg(&imported)
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
