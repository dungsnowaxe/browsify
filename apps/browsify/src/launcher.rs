use browsify_core::{AppType, LaunchAction, ResolvedLaunchPlan};
use std::process::Command;

/// Formats the macOS `open` command equivalent for dry-run and diagnostic logging.
pub fn format_open_command(plan: &ResolvedLaunchPlan) -> String {
    if plan.app_type == AppType::None || plan.browser_name.is_empty() {
        return "# [Suppressed: AppType is 'none']".to_string();
    }

    if let LaunchAction::DiaProfile { profile } = &plan.action {
        return format!(
            "Dia native profile {} (requires an open Dia window; checked at dispatch)",
            shell_quote(profile)
        );
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

/// Executes a [`ResolvedLaunchPlan`] via the macOS `open` command (LaunchServices)
/// with post-handoff `NSRunningApplication` activation (fixing Finicky Issues #501 & #550).
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

    if let LaunchAction::DiaProfile { profile } = &plan.action {
        return browsify_core::dia::open_profile(profile, &plan.final_url, plan.open_in_background);
    }

    // A Zen `--space <selector>` argument is validated against the workspace
    // store right before dispatch and rewritten to the stable workspace UUID.
    // A missing workspace suppresses the launch: Zen would navigate anyway,
    // leaking the URL into the active workspace.
    let mut resolved_plan;
    let plan = if is_zen_plan(plan) {
        match plan.args.iter().position(|a| a == "--space") {
            Some(space_index) => {
                let Some(selector) = plan.args.get(space_index + 1).cloned() else {
                    return Err("Zen --space argument is missing its workspace selector.".to_string());
                };
                let home = std::env::var("HOME").unwrap_or_default();
                let zen_root = std::path::Path::new(&home)
                    .join("Library/Application Support")
                    .join("zen");
                let workspace = browsify_core::zen::profile_dir(&zen_root, &plan.args)
                    .and_then(|dir| browsify_core::zen::resolve_workspace(&dir, &selector));
                match workspace {
                    Ok(ws) => {
                        resolved_plan = plan.clone();
                        resolved_plan.args[space_index + 1] = ws.uuid;
                        &resolved_plan
                    }
                    Err(err) => return Err(err),
                }
            }
            None => plan,
        }
    } else {
        plan
    };

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

    let mut child = Command::new("open")
        .args(&open_args)
        .spawn()
        .map_err(|e| format!("Failed to launch browser '{}': {}", plan.browser_name, e))?;

    if plan.activate_after_handoff {
        let _ = child.wait();
        crate::macos::activate_running_browser(plan.app_type, &plan.browser_name);
    }

    Ok(())
}

fn is_zen_plan(plan: &ResolvedLaunchPlan) -> bool {
    browsify_core::lookup_browser_meta(&plan.browser_name)
        .is_some_and(|m| m.id == browsify_core::zen::ZEN_BUNDLE_ID)
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
