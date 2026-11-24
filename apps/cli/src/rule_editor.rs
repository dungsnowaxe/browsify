//! Source-preserving edits of the simple subset of Handler Rules.
use crate::config_file::{conflict, ConfigFile};
use crate::config_loader::{BrowsifyEngine, ConfigPreferences};
use browsify_core::{NativeRouter, Rule, TomlConfig};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRequest {
    settings: Option<ConfigPreferences>,
    revision: String,
    rows: Option<Vec<Row>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Row {
    id: Option<usize>,
    rule: Option<Rule>,
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn toml_rule(table: &Table, aliases: Option<&Item>) -> Option<Rule> {
    if table
        .iter()
        .any(|(key, _)| !matches!(key, "match" | "browser"))
    {
        return None;
    }
    let patterns = table.get("match")?.as_value()?;
    let match_patterns = if let Some(p) = patterns.as_str() {
        vec![p.to_owned()]
    } else {
        patterns
            .as_array()?
            .iter()
            .map(|p| p.as_str().map(str::to_owned))
            .collect::<Option<_>>()?
    };
    let target = table.get("browser")?;
    let (browser, profile) = if let Some(name) = target.as_str() {
        if name.contains(':') || aliases.and_then(|a| a.get(name)).is_some() {
            return None;
        }
        (name, "")
    } else {
        let table = target.as_inline_table()?;
        if table
            .iter()
            .any(|(key, _)| !matches!(key, "name" | "profile"))
        {
            return None;
        }
        (
            table.get("name")?.as_str()?,
            table.get("profile").map_or(Some(""), |p| p.as_str())?,
        )
    };
    Some(Rule {
        match_patterns,
        browser: browser.to_owned(),
        profile: profile.to_owned(),
    })
}

fn summary(patterns: Option<&Item>, fallback: &str) -> String {
    patterns
        .and_then(Item::as_value)
        .and_then(|v| {
            v.as_str().map(str::to_owned).or_else(|| {
                v.as_array().map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
            })
        })
        .unwrap_or_else(|| fallback.to_owned())
}
pub fn snapshot(engine: &BrowsifyEngine) -> Result<Value, String> {
    let path = engine
        .primary_config_path()
        .ok_or("Editing disabled by --no-config")?;
    let file = ConfigFile::load(path)?;
    let doc: DocumentMut = file.text()?.parse().map_err(error)?;
    let supported = doc.get("handlers").is_none_or(|x| x.is_array_of_tables());
    let rows: Vec<_> = doc.get("handlers").and_then(Item::as_array_of_tables).into_iter().flat_map(|a| a.iter()).enumerate().map(|(i, table)| json!({
                    "id": i, "fingerprint": format!("{:x}",Sha256::digest(table.to_string().as_bytes())), "rule": toml_rule(table, doc.get("browsers")), "description": summary(table.get("match").or_else(|| table.get("match_hostnames")), &format!("Advanced rule {}", i + 1))
                })).collect();
    let parsed = TomlConfig::from_toml_str(file.text()?)?;
    let rewrites: Vec<_> = parsed
        .rewrite
        .iter()
        .enumerate()
        .map(|(i, rewrite)| {
            rewrite
                .match_patterns
                .clone()
                .map(|p| p.into_vec().join(", "))
                .or_else(|| rewrite.match_regex.clone())
                .unwrap_or_else(|| format!("Rewrite {}", i + 1))
        })
        .collect();
    let rows = if supported {
        rows
    } else {
        parsed.handlers.iter().enumerate().map(|(i, h)| json!({"id": i, "description": h.match_patterns.clone().map(|p| p.into_vec().join(", ")).unwrap_or_else(|| format!("Advanced rule {}", i + 1))})).collect()
    };
    let settings = preferences(&parsed);
    Ok(
        json!([{"source":"primary", "revision":file.revision(), "editableRows":supported, "rows":rows, "rewrites":rewrites, "settings":settings}]),
    )
}

pub fn preferences(cfg: &TomlConfig) -> ConfigPreferences {
    let target = cfg
        .default_browser
        .as_ref()
        .map(|v| v.to_browser_config(&cfg.browsers, ""));
    ConfigPreferences {
        default_browser: target.as_ref().map(|v| {
            if v.name.is_empty() {
                "none".into()
            } else {
                v.name.clone()
            }
        }),
        default_profile: target.as_ref().and_then(|v| {
            v.container
                .as_ref()
                .map(|c| format!("container:{c}"))
                .or_else(|| (!v.profile.is_empty()).then(|| v.profile.clone()))
        }),
        options: browsify_core::RulesOptions {
            keep_running: cfg.options.keep_running,
            hide_icon: cfg.options.hide_icon,
            log_requests: cfg.options.log_requests,
            check_for_updates: cfg.options.check_for_updates,
        },
    }
}

pub fn edit_preferences(doc: &mut DocumentMut, settings: &ConfigPreferences) -> Result<(), String> {
    let old = preferences(&TomlConfig::from_toml_str(&doc.to_string())?);
    if old.default_browser != settings.default_browser
        || old.default_profile != settings.default_profile
    {
        match &settings.default_browser {
            None if settings.default_profile.as_deref().unwrap_or("").is_empty() => {
                doc.remove("default_browser");
            }
            Some(browser) if !browser.trim().is_empty() => {
                if browser.eq_ignore_ascii_case("none") {
                    change_item(doc.as_table_mut(), "default_browser", value("none"));
                } else {
                    // A profile-only edit must retain custom launch arguments and window policy.
                    let mut target = if old.default_browser == settings.default_browser {
                        let original = &doc["default_browser"];
                        let spec = original
                            .as_str()
                            .and_then(|name| doc.get("browsers")?.get(name))
                            .unwrap_or(original);
                        spec.clone()
                            .into_value()
                            .ok()
                            .and_then(|v| v.as_inline_table().cloned())
                            .unwrap_or_default()
                    } else {
                        InlineTable::new()
                    };
                    target.remove("container");
                    target.remove("profile");
                    target.insert("name", browser.as_str().into());
                    if let Some(profile) = &settings.default_profile {
                        target.insert("profile", profile.as_str().into());
                    }
                    change_item(doc.as_table_mut(), "default_browser", value(target));
                }
            }
            _ => return Err("Choose a default browser.".into()),
        }
    }
    let opts = &settings.options;
    for (key, before, after) in [
        ("keep_running", old.options.keep_running, opts.keep_running),
        ("hide_icon", old.options.hide_icon, opts.hide_icon),
        ("log_requests", old.options.log_requests, opts.log_requests),
        (
            "check_for_updates",
            old.options.check_for_updates,
            opts.check_for_updates,
        ),
    ] {
        if before == after {
            continue;
        }
        if !doc.contains_key("options") {
            doc.insert("options", Item::Table(Table::new()));
        }
        let options = doc["options"]
            .as_table_like_mut()
            .ok_or("Invalid TOML options table")?;
        if let Some(v) = after {
            options.insert(key, value(v));
        } else {
            options.remove(key);
        }
    }
    Ok(())
}

fn validate_rows(rows: &[Row], old: &[Option<Rule>]) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut locked = Vec::new();
    for row in rows {
        let original = if let Some(id) = row.id {
            if !seen.insert(id) {
                return Err("Duplicate rule.".into());
            }
            old.get(id).ok_or_else(conflict)?.as_ref()
        } else {
            None
        };
        match (&row.rule, original, row.id) {
            (None, None, Some(id)) => locked.push(id),
            (Some(rule), previous, id) if id.is_none() || previous.is_some() => {
                if previous != Some(rule)
                    && (rule.browser.trim().is_empty()
                        || rule.browser.contains(':')
                        || rule.match_patterns.is_empty()
                        || rule.match_patterns.iter().any(|x| x.trim().is_empty()))
                {
                    return Err("Each rule needs a URL pattern and a browser.".into());
                }
            }
            _ => return Err("Edit advanced rules in TextEdit.".into()),
        }
    }
    if locked.into_iter().ne(old
        .iter()
        .enumerate()
        .filter_map(|(i, x)| x.is_none().then_some(i)))
    {
        return Err("Advanced rules must stay in their original relative order.".into());
    }
    Ok(())
}

fn change_item(table: &mut Table, key: &str, mut next: Item) {
    if let Some(decor) = table
        .get(key)
        .and_then(Item::as_value)
        .map(|x| x.decor().clone())
    {
        if let Some(v) = next.as_value_mut() {
            *v.decor_mut() = decor;
        }
    }
    table[key] = next;
}
fn save_toml(file: &ConfigFile, rows: &[Row]) -> Result<Vec<u8>, String> {
    let mut doc: DocumentMut = file.text()?.parse().map_err(error)?;
    let original = match doc.get("handlers") {
        Some(item) => item
            .as_array_of_tables()
            .ok_or("Edit this rules file in TextEdit.")?
            .clone(),
        None => ArrayOfTables::new(),
    };
    let old: Vec<_> = original
        .iter()
        .map(|t| toml_rule(t, doc.get("browsers")))
        .collect();
    validate_rows(rows, &old)?;
    let mut next = ArrayOfTables::new();
    for (slot, row) in rows.iter().enumerate() {
        let mut table = row
            .id
            .and_then(|i| original.get(i))
            .cloned()
            .unwrap_or_default();
        let before = row.id.and_then(|i| old[i].as_ref());
        if let Some(rule) = &row.rule {
            if before.is_none_or(|b| b.match_patterns != rule.match_patterns) {
                let patterns = if let [one] = rule.match_patterns.as_slice() {
                    value(one.as_str())
                } else {
                    value(
                        rule.match_patterns
                            .iter()
                            .map(String::as_str)
                            .collect::<Array>(),
                    )
                };
                change_item(&mut table, "match", patterns);
            }
            if before.is_none_or(|b| b.browser != rule.browser || b.profile != rule.profile) {
                let target = if rule.profile.is_empty()
                    && doc
                        .get("browsers")
                        .and_then(|a| a.get(&rule.browser))
                        .is_none()
                {
                    value(rule.browser.as_str())
                } else {
                    let mut spec = InlineTable::new();
                    spec.insert("name", rule.browser.as_str().into());
                    spec.insert("profile", rule.profile.as_str().into());
                    value(spec)
                };
                change_item(&mut table, "browser", target);
            }
        }
        if let Some(pos) = original.get(slot).and_then(Table::position) {
            table.set_position(pos);
        } else {
            table.set_position(usize::MAX);
        }
        next.push(table);
    }
    if next.is_empty() {
        doc.remove("handlers");
    } else {
        doc["handlers"] = Item::ArrayOfTables(next);
    }
    let rendered = doc.to_string();
    // Refuse edits to unusual layouts if rendering changes any TOML meaning.
    let parsed: toml::Value = toml::from_str(&rendered).map_err(error)?;
    let expected: toml::Value = toml_edit::de::from_document(doc).map_err(error)?;
    if parsed != expected {
        return Err("This layout cannot be edited safely. Open it in TextEdit.".into());
    }
    NativeRouter::from_toml_config(&TomlConfig::from_toml_str(&rendered)?)?;
    Ok(rendered.into_bytes())
}

pub fn save(engine: &mut BrowsifyEngine, request: SaveRequest) -> Result<Value, String> {
    let path = engine
        .primary_config_path()
        .ok_or("Editing disabled by --no-config")?;
    let file = ConfigFile::load(path)?;
    if file.revision() != request.revision {
        return Err(conflict());
    }
    let text = match request.rows {
        Some(rows) => String::from_utf8(save_toml(&file, &rows)?).map_err(error)?,
        None => file.text()?.to_owned(),
    };
    let mut doc = text.parse::<DocumentMut>().map_err(error)?;
    if let Some(settings) = request.settings {
        edit_preferences(&mut doc, &settings)?;
    }
    engine.commit(&request.revision, &doc.to_string())?;
    snapshot(engine)
}
