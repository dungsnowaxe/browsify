use std::process::Command;

#[test]
fn zen_workspace_dry_run_and_missing_target_suppress_after_import() {
    let fixture = tempfile::tempdir().unwrap();
    let config = fixture.path().join("config.js");
    let imported = fixture.path().join("imported.toml");
    let target = serde_json::json!({
        "name": "Zen",
        "profile": "workspace:Uninstalled Workspace \"Δ\"",
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
        // Dry-run describes the action without validating the workspace against
        // a live profile, and never falls back to an untargeted launch.
        assert!(stdout.contains("--space"), "{stdout}");
        assert!(stdout.contains("Uninstalled Workspace \"Δ\""), "{stdout}");
        assert!(!stdout.contains("--profile-directory"), "{stdout}");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn zen_workspace_string_target_keeps_full_selector() {
    // The compact string form "Zen:workspace:Research" must keep the whole
    // workspace selector, not truncate it at the second colon.
    let fixture = tempfile::tempdir().unwrap();
    let config = fixture.path().join("config.js");
    let imported = fixture.path().join("imported.toml");
    std::fs::write(
        &config,
        "export default { defaultBrowser: 'Zen:workspace:Research' };",
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
        .args(["open", "https://example.invalid/", "--dry-run", "--config"])
        .arg(&imported)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--space"), "{stdout}");
    assert!(stdout.contains("Research"), "{stdout}");
}
