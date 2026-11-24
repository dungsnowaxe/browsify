//! Preview legacy inputs before publishing a single TOML configuration.
use crate::config_file::ConfigFile;
use crate::config_loader::ConfigPreferences;
use crate::rule_editor::edit_preferences;
use browsify_core::{rules_file_to_toml, NativeRouter, RulesFile, TomlConfig};
use browsify_js::HybridEngine;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

fn legacy_primary_path(home_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        // 1. Native Browsify paths
        ".config/browsify/config.toml",
        ".config/browsify/browsify.toml",
        ".browsify.toml",
        ".config/browsify/config.json",
        ".config/browsify/config.js",
        ".config/browsify/config.ts",
        ".browsify.js",
        ".browsify.ts",
        // Finicky v3/v4 import locations
        ".finicky.js",
        ".finicky.ts",
        ".config/finicky.js",
        ".config/finicky.ts",
        ".config/finicky/finicky.js",
        ".config/finicky/finicky.ts",
    ];
    for rel in candidates {
        let p = home_dir.join(rel);
        if p.is_file() {
            return Some(fs::canonicalize(&p).unwrap_or(p));
        }
    }
    None
}

const IMPORTED: &str = "# Legacy Browsify configuration imported";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub sources: Vec<PathBuf>,
    pub text: String,
    pub failures: Vec<String>,
    pub converted: usize,
    pub needs_default: bool,
    pub revision: String,
    #[serde(skip)]
    pub inputs: Vec<(PathBuf, String)>,
}

pub fn sources(
    home: &Path,
    dest: &Path,
    explicit: Option<&Path>,
    custom_rules: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let legacy = custom_rules
        .map(Path::to_owned)
        .unwrap_or_else(|| RulesFile::default_path(home));
    if custom_rules.is_some() && !legacy.is_file() {
        return Err(format!("Missing legacy rules file: {}", legacy.display()));
    }
    let settings = legacy.with_extension("settings.json");
    let candidate = explicit
        .map(Path::to_owned)
        .or_else(|| legacy_primary_path(home));
    let mut paths: Vec<_> = candidate.into_iter().collect();
    if explicit.is_none() {
        if dest.is_file() {
            if fs::read_to_string(dest)
                .map_err(|e| e.to_string())?
                .lines()
                .any(|s| s == IMPORTED)
            {
                return Ok(Vec::new());
            }
            paths = vec![dest.to_owned()];
        }
    }
    if explicit.is_none() || custom_rules.is_some() {
        for p in [legacy, settings] {
            if p.is_file() && !paths.contains(&p) {
                paths.push(p)
            }
        }
        if paths.len() == 1 && paths[0] == dest {
            paths.clear()
        }
    }
    Ok(paths)
}

fn json_document(
    raw: &str,
    path: &Path,
    failures: &mut Vec<String>,
) -> Result<DocumentMut, String> {
    let mut input: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let rows = input
        .as_object_mut()
        .ok_or("Expected a rules object")?
        .remove("rules")
        .unwrap_or(serde_json::json!([]));
    let mut file: RulesFile = serde_json::from_value(input).map_err(|e| e.to_string())?;
    for (i, row) in rows
        .as_array()
        .ok_or("Expected a rules array")?
        .iter()
        .enumerate()
    {
        let parsed = serde_json::from_value::<browsify_core::Rule>(row.clone());
        match parsed {
            Ok(mut rule)
                if rule.match_patterns.iter().any(|m| !m.is_empty())
                    && !rule.browser.is_empty() =>
            {
                rule.match_patterns.retain(|m| !m.is_empty());
                file.rules.push(rule);
            }
            _ => failures.push(format!(
                "{} · Handler {} · {} · missing or invalid match or browser",
                path.display(),
                i + 1,
                row.get("match").unwrap_or(&serde_json::Value::Null)
            )),
        }
    }
    let mut doc: DocumentMut = rules_file_to_toml(&file)
        .parse()
        .map_err(|e: toml_edit::TomlError| e.to_string())?;
    if file.default_browser.is_empty() && file.default_profile.is_empty() {
        doc.remove("default_browser");
    }
    Ok(doc)
}

fn append_fallback(doc: &mut DocumentMut, fallback: &DocumentMut) -> Result<(), String> {
    if !doc.contains_key("default_browser") {
        if let Some(item) = fallback.get("default_browser") {
            doc.insert("default_browser", item.clone());
        }
    }
    if let Some(opts) = fallback.get("options").and_then(Item::as_table_like) {
        if !doc.contains_key("options") {
            doc.insert("options", Item::Table(Table::new()));
        }
        let dest = doc["options"]
            .as_table_like_mut()
            .ok_or("Invalid options table")?;
        for (key, v) in opts.iter() {
            if !dest.contains_key(key) {
                dest.insert(key, v.clone());
            }
        }
    }
    if let Some(rows) = fallback.get("handlers").and_then(Item::as_array_of_tables) {
        if !doc.contains_key("handlers") {
            doc.insert("handlers", Item::ArrayOfTables(ArrayOfTables::new()));
        }
        match doc["handlers"].as_array_mut() {
            Some(inline) => {
                for row in rows.iter() {
                    inline.push(row.clone().into_inline_table());
                }
            }
            None => {
                let dest = doc["handlers"]
                    .as_array_of_tables_mut()
                    .ok_or("Invalid handlers list")?;
                for row in rows.iter() {
                    dest.push(row.clone());
                }
            }
        }
    }
    Ok(())
}

pub fn preview(
    home: &Path,
    dest: &Path,
    explicit: Option<&Path>,
    custom_rules: Option<&Path>,
) -> Result<Preview, String> {
    let revision = ConfigFile::load(dest)?.revision();
    let paths = sources(home, dest, explicit, custom_rules)?;
    if paths.is_empty() {
        return Err("No configuration to import.".into());
    }
    let mut doc = DocumentMut::new();
    let mut failures = Vec::new();
    let mut needs_default = false;
    let mut inputs = Vec::new();
    let settings_path = custom_rules
        .map(Path::to_owned)
        .unwrap_or_else(|| RulesFile::default_path(home))
        .with_extension("settings.json");
    for (idx, path) in paths.iter().enumerate() {
        let file = ConfigFile::load(path)?;
        inputs.push((path.to_owned(), file.revision()));
        let raw = file.text()?;
        if path == &settings_path {
            let overrides: ConfigPreferences =
                serde_json::from_str(raw).map_err(|e| e.to_string())?;
            let mut current =
                crate::rule_editor::preferences(&TomlConfig::from_toml_str(&doc.to_string())?);
            if overrides.default_browser.is_some() {
                needs_default = false;
                current.default_browser = overrides.default_browser;
                current.default_profile = overrides.default_profile;
            }
            let opt = &mut current.options;
            let v = overrides.options;
            opt.keep_running = v.keep_running.or(opt.keep_running);
            opt.hide_icon = v.hide_icon.or(opt.hide_icon);
            opt.log_requests = v.log_requests.or(opt.log_requests);
            opt.check_for_updates = v.check_for_updates.or(opt.check_for_updates);
            edit_preferences(&mut doc, &current)?;
            continue;
        }
        let part = match path
            .extension()
            .and_then(|x| x.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("toml") => raw.parse::<DocumentMut>().map_err(|e| e.to_string())?,
            Some("json") => json_document(raw, path, &mut failures)?,
            Some("js" | "ts" | "mjs") => {
                let js = HybridEngine::from_js_source(raw, &[])?;
                needs_default = js.default_browser_is_dynamic();
                let (text, issues) = js.import_toml();
                failures.extend(
                    issues
                        .into_iter()
                        .map(|v| format!("{} · {v}", path.display())),
                );
                text.parse::<DocumentMut>().map_err(|e| e.to_string())?
            }
            _ => return Err(format!("Unsupported import format: {}", path.display())),
        };
        if idx == 0 {
            doc = part
        } else {
            append_fallback(&mut doc, &part)?;
        }
    }
    let text = format!("{IMPORTED}\n{doc}");
    let cfg = TomlConfig::from_toml_str(&text)?;
    NativeRouter::from_toml_config(&cfg)?;
    Ok(Preview {
        sources: paths,
        converted: cfg.handlers.len() + cfg.rewrite.len(),
        failures,
        text,
        needs_default,
        revision,
        inputs,
    })
}

impl Preview {
    pub fn finish_cli(
        &self,
        home: &Path,
        check: bool,
        approved: Option<&str>,
        browser: Option<&str>,
        output: Option<&Path>,
    ) -> Result<(), String> {
        let canonical = home.join(".config/browsify/config.toml");
        eprintln!("{}", self.report());
        if check {
            println!("Review ID: {}\n\n{}", self.id(), self.text);
            return Ok(());
        }
        let text = self.approval(browser)?;
        if let Some(id) = approved {
            if output.is_some() {
                return Err("Choose --apply for activation or --output for export.".into());
            }
            if id != self.id() {
                return Err("This review is stale. Run migrate --check again.".into());
            }
            let mut engine = crate::config_loader::BrowsifyEngine::load(home, None, None, false)?;
            engine.commit(&self.revision, &text)?;
            println!("Activated {}", canonical.display());
        } else if let Some(path) = output {
            if path
                .extension()
                .is_none_or(|s| !s.eq_ignore_ascii_case("toml"))
            {
                return Err("Choose a .toml output file.".into());
            }
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            if path.file_name() == canonical.file_name()
                && fs::canonicalize(parent).ok()
                    == fs::canonicalize(canonical.parent().unwrap()).ok()
            {
                return Err(
                    "Review with migrate --check and activate with --apply REVIEW-ID.".into(),
                );
            }
            let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
            std::io::Write::write_all(&mut temp, text.as_bytes()).map_err(|e| e.to_string())?;
            temp.persist_noclobber(path)
                .map_err(|e| format!("Could not create {}: {}", path.display(), e.error))?;
        } else {
            print!("{text}");
        }
        Ok(())
    }

    pub fn id(&self) -> String {
        use sha2::{Digest, Sha256};
        format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(self, &self.inputs)).expect("serializable preview")
            )
        )
    }
    pub fn approval(&self, browser: Option<&str>) -> Result<String, String> {
        for (path, rev) in &self.inputs {
            if ConfigFile::load(path)?.revision() != *rev {
                return Err("An import source changed. Review the import again.".into());
            }
        }
        if !self.needs_default {
            return Ok(self.text.clone());
        }
        let choice = browser
            .filter(|s| !s.trim().is_empty())
            .ok_or("Choose a replacement for the JavaScript default browser.")?;
        let mut doc = self
            .text
            .parse::<DocumentMut>()
            .map_err(|e| e.to_string())?;
        doc["default_browser"] = value(choice);
        Ok(doc.to_string())
    }

    pub fn report(&self) -> String {
        let mut lines = vec![format!(
            "{} {} imported, {} failed.",
            self.converted,
            if self.converted == 1 { "rule" } else { "rules" },
            self.failures.len()
        )];
        lines.extend(self.failures.clone());
        if self.needs_default {
            lines.push("Choose a replacement for the JavaScript default browser.".into());
        }
        lines.join("\n")
    }
}
