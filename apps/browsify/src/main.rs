mod config_loader;
mod launcher;
mod macos;
mod ui_server;
mod update;

use browsify_core::{OpenerInfo, ProfileResolver, BROWSERS_REGISTRY};
use config_loader::{migrate_config, BrowsifyEngine, MigrateTarget};
use launcher::{execute_launch_plan, format_open_command};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn print_usage() {
    println!(
        "Browsify v{} — Ultra-lean Rust browser & profile router

USAGE:
  browsify <COMMAND> [OPTIONS]
  browsify [FINICKY_FLAGS]

COMMANDS:
  test <URL>               Dry-run a URL through rewrites, rules, and profile lookup
  open <URL>               Resolve and launch the target browser & profile for <URL>
  validate                 Validate configuration and check all referenced browser profiles
  migrate                  Convert existing .finicky.js/.ts or rules.json to native config.toml
  list-browsers            List all 22+ supported browser identifiers and families
  list-profiles [BROWSER]  List discovered profiles for a browser (or all installed browsers)
  ui                       Start the on-demand 127.0.0.1 UI server (0 MB idle RAM when closed)

OPTIONS:
  --config <PATH>          Custom configuration file (.toml, .js, .ts, .json)
  --rules <PATH>           Custom rules.json path
  --no-config              Skip primary config file and use rules.json only
  --dry-run                Print the resolved `open` command without launching the browser
  --background             Open the browser in the background
  --opener-bundle-id <ID>  Simulate an opener application bundle ID (for `test`)
  --opener-name <NAME>     Simulate an opener application name (for `test`)
  --output <PATH>          Write migrated config.toml to <PATH> (for `migrate`)
  --port <PORT>            Port for `ui` server (default: 0 / ephemeral)
  --no-open                Do not automatically open the browser when starting `ui`",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    macos::init_host_hooks();

    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help" || a == "help") {
        print_usage();
        return;
    }

    let home_dir = env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    let mut custom_config: Option<PathBuf> = None;
    let mut custom_rules: Option<PathBuf> = None;
    let mut skip_config = false;
    let mut dry_run = false;
    let mut background = false;
    let mut force_window = false;
    let mut opener_bundle_id: Option<String> = None;
    let mut opener_name: Option<String> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut migrate_target = MigrateTarget::ToToml;
    let mut ui_port: u16 = 0;
    let mut no_open = false;

    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with("-psn_") {
            i += 1;
            continue;
        }
        match arg.as_str() {
            "--config" | "-config" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    custom_config = Some(PathBuf::from(v));
                }
            }
            "--rules" | "-rules" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    custom_rules = Some(PathBuf::from(v));
                }
            }
            "--no-config" | "-no-config" => skip_config = true,
            "--dry-run" | "-dry-run" => dry_run = true,
            "--background" | "-g" => background = true,
            "--window" | "-window" => force_window = true,
            "--check" => migrate_target = MigrateTarget::Check,
            "--to-toml" => migrate_target = MigrateTarget::ToToml,
            "--to-json" => migrate_target = MigrateTarget::ToJson,
            "--opener-bundle-id" => {
                i += 1;
                opener_bundle_id = args.get(i).cloned();
            }
            "--opener-name" => {
                i += 1;
                opener_name = args.get(i).cloned();
            }
            "--output" | "-o" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    output_path = Some(PathBuf::from(v));
                }
            }
            "--port" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    ui_port = v.parse().unwrap_or(0);
                }
            }
            "--no-open" => no_open = true,
            _ => positional.push(arg.clone()),
        }
        i += 1;
    }

    let cmd = positional.first().map(|s| s.as_str()).unwrap_or("daemon");

    match cmd {
        "daemon" => {
            let engine = load_engine_or_exit(
                &home_dir,
                custom_config.as_deref(),
                custom_rules.as_deref(),
                skip_config,
            );
            let resolver = Arc::new(ProfileResolver::new(&home_dir));
            macos::run_daemon(
                Arc::new(Mutex::new(engine)),
                resolver,
                force_window,
                dry_run,
            );
        }
        "list-browsers" => {
            println!("{:<36} {:<28} {:<10}", "BUNDLE ID", "APP NAME", "FAMILY");
            println!("{}", "-".repeat(76));
            for b in BROWSERS_REGISTRY {
                println!("{:<36} {:<28} {:?}", b.id, b.app_name, b.family);
            }
        }
        "list-profiles" => {
            let resolver = ProfileResolver::new(&home_dir);
            if let Some(browser) = positional.get(1) {
                let profiles = resolver.list_profiles(browser);
                if profiles.is_empty() {
                    println!("No profiles found for '{}'", browser);
                } else {
                    for p in profiles {
                        println!("{}", p);
                    }
                }
            } else {
                for b in BROWSERS_REGISTRY {
                    let profiles = resolver.list_profiles(b.id);
                    if !profiles.is_empty() {
                        println!("{} ({}): {}", b.app_name, b.id, profiles.join(", "));
                    }
                }
            }
        }
        "migrate" => {
            match migrate_config(&home_dir, custom_config.as_deref(), migrate_target) {
                Ok((src_path, out_text)) => {
                    if let Some(out) = output_path {
                        if let Some(parent) = out.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Err(e) = fs::write(&out, &out_text) {
                            eprintln!("Failed writing {}: {}", out.display(), e);
                            std::process::exit(1);
                        }
                        println!("Migrated {} -> {}", src_path, out.display());
                    } else {
                        print!("{}", out_text);
                    }
                }
                Err(e) => {
                    eprintln!("Migration error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "validate" => {
            let engine = load_engine_or_exit(
                &home_dir,
                custom_config.as_deref(),
                custom_rules.as_deref(),
                skip_config,
            );
            let resolver = ProfileResolver::new(&home_dir);
            let sum = engine.summary();
            println!("Configuration valid!");
            println!(
                "  Source:          {}",
                sum.config_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "None (Default)".to_string())
            );
            println!("  Format / Engine: {}", sum.format);
            println!(
                "  Fast Path:       {}",
                if sum.pure_native_fast_path {
                    "Pure Native Rust (~2.3 µs)"
                } else {
                    "Hybrid QuickJS (~42 µs)"
                }
            );
            println!("  JS VM Active:    {}", sum.js_vm_active);
            println!("  Default Browser: {}", sum.default_browser);
            println!(
                "  Rewrites:        {}/{} static",
                sum.static_rewrites_count, sum.rewrites_count
            );
            println!(
                "  Handlers:        {}/{} static",
                sum.static_handlers_count, sum.handlers_count
            );
            for warn in engine.validate_profiles(&resolver) {
                println!("  Profile Note:    {}", warn);
            }
        }
        "test" | "open" => {
            let Some(target_url) = positional.get(1) else {
                eprintln!("Usage: browsify {} <URL>", cmd);
                std::process::exit(1);
            };
            let engine = load_engine_or_exit(
                &home_dir,
                custom_config.as_deref(),
                custom_rules.as_deref(),
                skip_config,
            );
            let resolver = ProfileResolver::new(&home_dir);
            let opener = if opener_bundle_id.is_some() || opener_name.is_some() {
                Some(OpenerInfo {
                    name: opener_name.unwrap_or_default(),
                    bundle_id: opener_bundle_id.unwrap_or_default(),
                    path: String::new(),
                    window_title: None,
                    pid: 0,
                })
            } else {
                None
            };

            let resolved = match engine.resolve(target_url, opener.as_ref(), background) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("Error resolving URL: {}", e);
                    std::process::exit(1);
                }
            };

            let plan = resolver.build_launch_plan(
                &resolved,
                background,
                engine.summary().options.on_missing_profile,
            );

            if cmd == "test" {
                println!("Input URL:   {}", target_url);
                println!("Final URL:   {}", plan.final_url);
                println!("Browser:     {} ({})", plan.browser_name, plan.app_type.as_str());
                if !resolved.profile.is_empty() {
                    println!("Profile:     {}", resolved.profile);
                }
                if !plan.args.is_empty() {
                    println!("Args:        {}", plan.args.join(" "));
                }
                println!("Command:     {}", format_open_command(&plan));
                if let Some(w) = &plan.warning {
                    println!("Warning:     {}", w);
                }
            } else if let Err(e) = execute_launch_plan(&plan, dry_run) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        "ui" => {
            let _lock = match macos::acquire_singleton_lock() {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Browsify is already running ({}). Exiting.", e);
                    std::process::exit(1);
                }
            };
            let engine = load_engine_or_exit(
                &home_dir,
                custom_config.as_deref(),
                custom_rules.as_deref(),
                skip_config,
            );
            let resolver = Arc::new(ProfileResolver::new(&home_dir));
            if let Err(e) =
                ui_server::run_ui_server(Arc::new(Mutex::new(engine)), resolver, ui_port, !no_open)
            {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        other => {
            // If invoked directly with a URL (e.g. `browsify https://example.com`)
            if other.contains("://") {
                let engine = load_engine_or_exit(
                    &home_dir,
                    custom_config.as_deref(),
                    custom_rules.as_deref(),
                    skip_config,
                );
                let resolver = ProfileResolver::new(&home_dir);
                match engine.resolve(other, None, background) {
                    Ok(cfg) => {
                        let plan = resolver.build_launch_plan(
                            &cfg,
                            background,
                            engine.summary().options.on_missing_profile,
                        );
                        if let Err(e) = execute_launch_plan(&plan, dry_run) {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error resolving URL: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Unknown command '{}'. Run `browsify --help` for usage.", other);
                std::process::exit(1);
            }
        }
    }
}

fn load_engine_or_exit(
    home_dir: &Path,
    custom_config: Option<&Path>,
    custom_rules: Option<&Path>,
    skip_config: bool,
) -> BrowsifyEngine {
    match BrowsifyEngine::load(home_dir, custom_config, custom_rules, skip_config) {
        Ok(eng) => eng,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    }
}
