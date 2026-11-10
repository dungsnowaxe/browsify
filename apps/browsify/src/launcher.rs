use browsify_core::{AppType, ResolvedLaunchPlan};
use std::process::Command;

/// Formats the macOS `open` command equivalent for dry-run and diagnostic logging.
pub fn format_open_command(plan: &ResolvedLaunchPlan) -> String {
    if plan.app_type == AppType::None || plan.browser_name.is_empty() {
        return "# [Suppressed: AppType is 'none']".to_string();
    }

    let mut parts = vec!["open".to_string()];
    match plan.app_type {
        AppType::BundleId => {
            parts.push("-b".to_string());
            parts.push(shell_quote(&plan.browser_name));
        }
        _ => {
            parts.push("-a".to_string());
            parts.push(shell_quote(&plan.browser_name));
        }
    }

    if plan.open_in_background {
        parts.push("-g".to_string());
    }
    if plan.creates_new_instance {
        parts.push("-n".to_string());
    }

    if !plan.args.is_empty() {
        parts.push("--args".to_string());
        for a in &plan.args {
            parts.push(shell_quote(a));
        }
        // Only append the URL if `--app=<url>` did not already embed it
        if !plan.args.iter().any(|a| a.starts_with("--app=")) {
            parts.push(shell_quote(&plan.final_url));
        }
    } else {
        parts.push(shell_quote(&plan.final_url));
    }

    parts.join(" ")
}

/// Executes a [`ResolvedLaunchPlan`].
pub fn execute_launch_plan(plan: &ResolvedLaunchPlan, dry_run: bool) -> Result<(), String> {
    if let Some(warn) = &plan.warning {
        eprintln!("browsify warning: {}", warn);
    }

    if plan.app_type == AppType::None || plan.browser_name.is_empty() {
        return Ok(());
    }

    if dry_run {
        println!("{}", format_open_command(plan));
        return Ok(());
    }

    let mut open_args = Vec::new();
    match plan.app_type {
        AppType::BundleId => {
            open_args.push("-b".to_string());
            open_args.push(plan.browser_name.clone());
        }
        _ => {
            open_args.push("-a".to_string());
            open_args.push(plan.browser_name.clone());
        }
    }

    if plan.open_in_background {
        open_args.push("-g".to_string());
    }
    if plan.creates_new_instance {
        open_args.push("-n".to_string());
    }

    if !plan.args.is_empty() {
        open_args.push("--args".to_string());
        open_args.extend(plan.args.clone());
        if !plan.args.iter().any(|a| a.starts_with("--app=")) {
            open_args.push(plan.final_url.clone());
        }
    } else {
        open_args.push(plan.final_url.clone());
    }

    // Spawn non-blocking on macOS (`open`), and if `activate_after_handoff` is set
    // (fixes Finicky #501 & #550 where `-n` helper exits and drops foreground focus),
    // activate the target browser after the helper completes handoff.
    let mut child = Command::new("open")
        .args(&open_args)
        .spawn()
        .map_err(|e| format!("Failed to launch browser '{}': {}", plan.browser_name, e))?;

    if plan.activate_after_handoff {
        let _ = child.wait();
        #[cfg(target_os = "macos")]
        {
            // Explicitly bring the running browser process to the foreground across Spaces
            let script = if plan.app_type == AppType::BundleId {
                format!(
                    "tell application id {:?} to activate",
                    plan.browser_name
                )
            } else {
                format!("tell application {:?} to activate", plan.browser_name)
            };
            let _ = Command::new("osascript").args(["-e", &script]).spawn();
        }
    }

    Ok(())
}

fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/' | b':' | b'=')
        })
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}
