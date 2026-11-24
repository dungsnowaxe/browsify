//! Route Zen workspaces through the browser's native `--space` command-line flag.
//! Workspace discovery reads only `zen-sessions.jsonlz4` (name and UUID per
//! workspace); no tabs, history or cookies are touched.

use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const ZEN_BUNDLE_ID: &str = "app.zen-browser.zen";

/// The Zen profile directory a `workspace:` selector applies to: an explicit
/// `--profile <dir>` or `-P <name>` argument when present, otherwise the
/// default profile from `profiles.ini` in the Zen configuration root.
pub fn profile_dir(zen_root: &Path, args: &[String]) -> Result<PathBuf, String> {
    for (i, arg) in args.iter().enumerate() {
        let value = if arg == "--profile" {
            args.get(i + 1).map(|s| s.as_str())
        } else if let Some(v) = arg.strip_prefix("--profile=") {
            Some(v)
        } else {
            None
        };
        if let Some(value) = value {
            let expanded = expand_home(value, zen_root);
            if expanded.is_dir() {
                return Ok(expanded);
            }
            return Err(format!(
                "Custom --profile argument '{}' is not an existing directory.",
                value
            ));
        }
    }
    let named = args
        .iter()
        .position(|a| a.as_str() == "-P")
        .and_then(|i| args.get(i + 1));
    if let Some(name) = named {
        return ini_profile_dir(zen_root, Some(name));
    }
    ini_profile_dir(zen_root, None)
}

fn expand_home(value: &str, zen_root: &Path) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/") {
        zen_root
            .ancestors()
            .nth(3)
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(value))
    } else {
        PathBuf::from(value)
    }
}

#[derive(Debug, Deserialize)]
struct ZenSessionFile {
    #[serde(default)]
    spaces: Vec<ZenSpace>,
}

#[derive(Debug, Deserialize)]
struct ZenSpace {
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    name: String,
}

/// A workspace as persisted by Zen, with the stable UUID Zen resolves first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenWorkspace {
    pub uuid: String,
    pub name: String,
}

pub fn list_workspaces(profile_dir: &Path) -> Result<Vec<ZenWorkspace>, String> {
    let bytes = std::fs::read(profile_dir.join("zen-sessions.jsonlz4"))
        .map_err(|_| "Could not read Zen workspaces. Open Zen once so it creates its session store.".to_string())?;
    let json = decompress_mozlz4(&bytes)?;
    let parsed: ZenSessionFile = serde_json::from_slice(&json)
        .map_err(|_| "Zen session store has an unexpected format".to_string())?;
    Ok(parsed
        .spaces
        .into_iter()
        .map(|s| ZenWorkspace { uuid: s.uuid, name: s.name })
        .collect())
}

/// Resolves a `workspace:` selector the same way Zen's command line does:
/// exact UUID first, then case-insensitive unique name.
pub fn resolve_workspace(
    profile_dir: &Path,
    selector: &str,
) -> Result<ZenWorkspace, String> {
    let spaces = list_workspaces(profile_dir)?;
    if let Some(hit) = spaces.iter().find(|s| s.uuid == selector) {
        return Ok(hit.clone());
    }
    let matches: Vec<&ZenWorkspace> = spaces
        .iter()
        .filter(|s| s.name.eq_ignore_ascii_case(selector))
        .collect();
    match matches.as_slice() {
        [only] => Ok((*only).clone()),
        [] => Err(format!(
            "Zen workspace '{}' not found. Available workspaces: {}",
            selector,
            available_names(&spaces)
        )),
        _ => Err(format!(
            "Zen workspace name '{}' matches several workspaces; use its exact name or UUID.",
            selector
        )),
    }
}

fn available_names(spaces: &[ZenWorkspace]) -> String {
    let mut names: Vec<&str> = spaces
        .iter()
        .map(|s| {
            if s.name.is_empty() {
                s.uuid.as_str()
            } else {
                s.name.as_str()
            }
        })
        .collect();
    names.sort_unstable();
    names.dedup();
    names.join(", ")
}

/// Maps a Zen profile name (or the default) to its directory using
/// `profiles.ini` in the Zen configuration root.
fn ini_profile_dir(zen_root: &Path, name: Option<&str>) -> Result<PathBuf, String> {
    let ini_path = zen_root.join("profiles.ini");
    let text = std::fs::read_to_string(&ini_path).map_err(|_| {
        "Could not read Zen's profiles.ini. Open Zen once so it creates a profile, or add an explicit --profile <path> argument."
            .to_string()
    })?;
    let (profiles, install_default) = parse_profiles_ini(&text);
    if let Some(name) = name {
        if let Some((_, path, relative, _)) = profiles
            .iter()
            .find(|(n, _, _, _)| !n.is_empty() && n.eq_ignore_ascii_case(name))
        {
            return Ok(if *relative {
                zen_root.join(path)
            } else {
                PathBuf::from(path)
            });
        }
        return Err(format!("Zen profile '{}' not found in profiles.ini.", name));
    }
    if let Some(default) = install_default {
        return Ok(zen_root.join(default));
    }
    if let Some((_, path, relative, _)) = profiles.iter().find(|(_, _, _, d)| *d) {
        return Ok(if *relative {
            zen_root.join(path)
        } else {
            PathBuf::from(path)
        });
    }
    if profiles.len() == 1 {
        let (_, path, relative, _) = &profiles[0];
        return Ok(if *relative {
            zen_root.join(path)
        } else {
            PathBuf::from(path)
        });
    }
    Err("Could not determine Zen's default profile. Add an explicit --profile <path> argument.".into())
}

/// Parses `profiles.ini` into `(name, path, is_relative, is_default)` tuples
/// plus the install-section `Default=` path when present.
fn parse_profiles_ini(text: &str) -> (Vec<(String, String, bool, bool)>, Option<String>) {
    #[derive(Default)]
    struct Entry {
        name: String,
        path: String,
        relative: bool,
        default: bool,
    }
    let mut profiles: Vec<(String, String, bool, bool)> = Vec::new();
    let mut current: Option<Entry> = None;
    let mut install_default = None;
    let mut in_install_section = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(entry) = current.take() {
                if !entry.path.is_empty() {
                    profiles.push((entry.name, entry.path, entry.relative, entry.default));
                }
            }
            in_install_section = section.trim().to_ascii_lowercase().starts_with("install");
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();
        match (in_install_section, key) {
            (true, "Default") => install_default = Some(value.to_string()),
            (false, "Name") => current.get_or_insert_with(Default::default).name = value.to_string(),
            (false, "Path") => current.get_or_insert_with(Default::default).path = value.to_string(),
            (false, "IsRelative") => {
                current.get_or_insert_with(Default::default).relative = value == "1";
            }
            (false, "Default") => {
                if value == "1" {
                    current.get_or_insert_with(Default::default).default = true;
                }
            }
            _ => {}
        }
    }
    if let Some(entry) = current.take() {
        if !entry.path.is_empty() {
            profiles.push((entry.name, entry.path, entry.relative, entry.default));
        }
    }
    (profiles, install_default)
}

/// Decompresses a Mozilla `mozLz40\0` archive (8-byte magic, u32 LE
/// decompressed size, LZ4 block).
fn decompress_mozlz4(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 12 || &bytes[..8] != b"mozLz40\0" {
        return Err("Zen session store is not in the expected mozLz4 format".into());
    }
    let size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    lz4_flex::decompress(&bytes[12..], size)
        .map_err(|_| "Could not decompress the Zen session store".to_string())
}
