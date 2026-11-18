//! Source-preserving edits of the simple subset of Handler Rules.
use crate::config_loader::BrowsifyEngine;
use browsify_core::{NativeRouter, Rule, RulesFile, TomlConfig};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Primary,
    Visual,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRequest {
    source: Source,
    revision: String,
    rows: Vec<Row>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Row {
    id: Option<usize>,
    rule: Option<Rule>,
}

struct File {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}
impl File {
    fn load(path: &Path) -> Result<Self, String> {
        let bytes = read_optional(path)?;
        // Replace the linked file, not the symlink itself.
        let path = if bytes.is_some() {
            fs::canonicalize(path).map_err(error)?
        } else {
            path.to_owned()
        };
        Ok(Self { path, bytes })
    }
    fn text(&self) -> Result<&str, String> {
        self.bytes
            .as_deref()
            .map(std::str::from_utf8)
            .transpose()
            .map_err(error)
            .map(|s| s.unwrap_or("{}"))
    }
    fn revision(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.path.as_os_str().as_encoded_bytes());
        h.update([0, self.bytes.is_some() as u8]);
        if let Some(bytes) = &self.bytes {
            h.update(bytes);
        }
        format!("{:x}", h.finalize())
    }
    fn replace(&self, bytes: &[u8]) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(error)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
        if let Ok(metadata) = fs::metadata(&self.path) {
            tmp.as_file()
                .set_permissions(metadata.permissions())
                .map_err(error)?;
        }
        tmp.write_all(bytes).map_err(error)?;
        tmp.as_file().sync_all().map_err(error)?;
        if read_optional(&self.path)? != self.bytes {
            return Err(conflict());
        }
        tmp.persist(&self.path).map_err(error)?;
        Ok(())
    }
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn conflict() -> String {
    "The rules file changed on disk. Your draft is still here. Reload before saving again.".into()
}
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error(e)),
    }
}

fn select(engine: &BrowsifyEngine, source: Source) -> Result<File, String> {
    match source {
        Source::Primary => File::load(
            engine
                .primary_config_path()
                .ok_or("There is no advanced rules file.")?,
        ),
        Source::Visual => File::load(&engine.rules_path()),
    }
}
fn json_rules(doc: &Value) -> Result<&[Value], String> {
    match doc.get("rules") {
        None => Ok(&[]),
        Some(Value::Array(rows)) => Ok(rows),
        _ => Err("Could not read the list of rules.".into()),
    }
}
fn json_rule(row: &Value) -> Option<Rule> {
    if row
        .as_object()?
        .keys()
        .any(|key| !matches!(key.as_str(), "match" | "browser" | "profile"))
    {
        return None;
    }
    let rule: Rule = serde_json::from_value(row.clone()).ok()?;
    if rule.browser.contains(':') {
        return None;
    }
    Some(rule)
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
fn json_group(file: &File, name: &str) -> Result<Value, String> {
    let doc: Value = serde_json::from_str(file.text()?).map_err(error)?;
    let rows: Vec<_> = json_rules(&doc)?
        .iter()
        .enumerate()
        .map(|(i, raw)| {
            let description = serde_json::from_value::<Rule>(raw.clone())
                .ok()
                .map(|rule| format!("{} → {}", rule.match_patterns.join(", "), rule.browser))
                .unwrap_or_else(|| format!("Advanced rule {}", i + 1));
            json!({"id": i, "rule": json_rule(raw), "description": description})
        })
        .collect();
    Ok(json!({"source": name, "revision": file.revision(), "rows": rows, "rewrites": []}))
}
pub fn snapshot(engine: &BrowsifyEngine) -> Result<Value, String> {
    let visual = select(engine, Source::Visual)?;
    let mut groups = Vec::new();
    if let Some(path) = engine.primary_config_path() {
        let file = File::load(path)?;
        match path
            .extension()
            .and_then(|x| x.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("toml") => {
                let doc: DocumentMut = file.text()?.parse().map_err(error)?;
                let supported = doc.get("handlers").is_none_or(|x| x.is_array_of_tables());
                let rows: Vec<_> = doc.get("handlers").and_then(Item::as_array_of_tables).into_iter().flat_map(|a| a.iter()).enumerate().map(|(i, table)| json!({
                    "id": i, "rule": toml_rule(table, doc.get("browsers")), "description": summary(table.get("match").or_else(|| table.get("match_hostnames")), &format!("Advanced rule {}", i + 1))
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
                groups.push(json!({"source": "primary", "revision": supported.then(|| file.revision()), "rows": rows, "rewrites": rewrites}));
            }
            Some("json") => groups.push(json_group(&file, "primary")?),
            _ => {
                let descriptions = engine.scripted_rules().unwrap_or_default();
                let rows: Vec<_> = descriptions["handlers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, label)| json!({"id": i, "description": label}))
                    .collect();
                groups.push(json!({"source": "primary", "rows": rows, "rewrites": descriptions["rewrites"]}));
            }
        }
        if file.path != visual.path {
            groups.push(json_group(&visual, "visual")?);
        }
    } else {
        groups.push(json_group(&visual, "visual")?);
    }
    Ok(json!(groups))
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

fn save_json(file: &File, rows: &[Row]) -> Result<Vec<u8>, String> {
    let mut doc: Value = serde_json::from_str(file.text()?).map_err(error)?;
    let originals = json_rules(&doc)?;
    let old: Vec<_> = originals.iter().map(json_rule).collect();
    validate_rows(rows, &old)?;
    let next: Vec<_> = rows
        .iter()
        .map(|row| {
            if let Some(i) = row.id {
                if old[i] == row.rule {
                    return originals[i].clone();
                }
            }
            json!(row.rule)
        })
        .collect();
    doc.as_object_mut()
        .ok_or("Could not read the rules file.")?
        .insert("rules".into(), json!(next));
    serde_json::from_value::<RulesFile>(doc.clone()).map_err(error)?;
    let mut bytes = serde_json::to_vec_pretty(&doc).map_err(error)?;
    bytes.push(b'\n');
    Ok(bytes)
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
fn save_toml(file: &File, rows: &[Row]) -> Result<Vec<u8>, String> {
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
    if engine.check_files_changed() {
        engine.reload().map_err(|_| conflict())?;
    }
    let file = select(engine, request.source)?;
    if file.revision() != request.revision {
        return Err(conflict());
    }
    let bytes = match request.source {
        Source::Visual => save_json(&file, &request.rows)?,
        Source::Primary => match engine
            .primary_config_path()
            .and_then(Path::extension)
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("toml") => save_toml(&file, &request.rows)?,
            Some("json") => save_json(&file, &request.rows)?,
            _ => return Err("Edit this rules file in TextEdit.".into()),
        },
    };
    file.replace(&bytes)?;
    if let Err(e) = engine.reload() {
        let written = File {
            path: file.path.clone(),
            bytes: Some(bytes),
        };
        if let Some(original) = &file.bytes {
            written.replace(original)?;
        } else if read_optional(&file.path)? == written.bytes {
            fs::remove_file(&file.path).map_err(error)?;
        }
        return Err(e);
    }
    snapshot(engine)
}
