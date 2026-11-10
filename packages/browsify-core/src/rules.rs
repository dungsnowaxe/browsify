use crate::browser::{autodetect_app_string_type, BrowserConfig, ProfileSelector};
use crate::profile::MissingProfilePolicy;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_running: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_icon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_requests: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_for_updates: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub match_patterns: Vec<String>,
    pub browser: String,
    pub profile: String,
}

impl<'de> Deserialize<'de> for Rule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum MatchField {
            Single(String),
            Multiple(Vec<String>),
        }

        #[derive(Deserialize)]
        struct RawRule {
            #[serde(rename = "match")]
            match_field: Option<MatchField>,
            #[serde(default)]
            browser: String,
            #[serde(default)]
            profile: String,
        }

        let raw = RawRule::deserialize(deserializer)?;
        let match_patterns = match raw.match_field {
            Some(MatchField::Single(s)) => vec![s],
            Some(MatchField::Multiple(v)) => v,
            None => Vec::new(),
        };
        Ok(Rule {
            match_patterns,
            browser: raw.browser,
            profile: raw.profile,
        })
    }
}

impl Serialize for Rule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct RuleOut<'a> {
            #[serde(rename = "match")]
            match_field: MatchOut<'a>,
            browser: &'a str,
            #[serde(skip_serializing_if = "str::is_empty")]
            profile: &'a str,
        }

        #[derive(Serialize)]
        #[serde(untagged)]
        enum MatchOut<'a> {
            Single(&'a str),
            Multiple(&'a [String]),
        }

        let match_field = if self.match_patterns.len() == 1 {
            MatchOut::Single(&self.match_patterns[0])
        } else {
            MatchOut::Multiple(&self.match_patterns)
        };

        RuleOut {
            match_field,
            browser: &self.browser,
            profile: &self.profile,
        }
        .serialize(serializer)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesFile {
    #[serde(default)]
    pub default_browser: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub default_profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<RulesOptions>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl RulesFile {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(RulesFile::default());
        }
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn default_path(home_dir: &Path) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            let browsify_path = home_dir.join("Library/Application Support/Browsify/rules.json");
            if browsify_path.exists() {
                return browsify_path;
            }
            let finicky_path = home_dir.join("Library/Application Support/Finicky/rules.json");
            if finicky_path.exists() {
                return finicky_path;
            }
            browsify_path
        }
        #[cfg(not(target_os = "macos"))]
        {
            let xdg = home_dir.join(".config/browsify/rules.json");
            if xdg.exists() {
                return xdg;
            }
            let finicky = home_dir.join(".config/Finicky/rules.json");
            if finicky.exists() {
                return finicky;
            }
            xdg
        }
    }
}

// ============================================================================
// Declarative TOML Config (`~/.config/browsify/config.toml`)
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TomlConfig {
    #[serde(default)]
    pub default_browser: Option<TomlBrowserTarget>,
    #[serde(default)]
    pub options: TomlOptions,
    #[serde(default)]
    pub browsers: HashMap<String, TomlBrowserSpec>,
    #[serde(default)]
    pub rewrite: Vec<TomlRewriteRule>,
    #[serde(default)]
    pub handlers: Vec<TomlHandlerRule>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TomlOptions {
    pub keep_running: Option<bool>,
    pub hide_icon: Option<bool>,
    pub log_requests: Option<bool>,
    pub check_for_updates: Option<bool>,
    pub resolve_short_urls: Option<bool>,
    #[serde(default)]
    pub on_missing_profile: MissingProfilePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TomlBrowserTarget {
    NameOrAlias(String),
    Spec(TomlBrowserSpec),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TomlBrowserSpec {
    pub name: String,
    #[serde(default)]
    pub profile: String,
    pub open_in_background: Option<bool>,
    pub container: Option<String>,
    pub app_id: Option<String>,
    pub app_mode: Option<bool>,
    #[serde(default)]
    pub args: Vec<String>,
}

impl TomlBrowserTarget {
    pub fn to_browser_config(
        &self,
        aliases: &HashMap<String, TomlBrowserSpec>,
        url: &str,
    ) -> BrowserConfig {
        match self {
            TomlBrowserTarget::NameOrAlias(s) => {
                if s.eq_ignore_ascii_case("none") || s.is_empty() {
                    return BrowserConfig::none(url);
                }
                if let Some(preset) = aliases.get(s) {
                    return preset.to_browser_config(url);
                }
                BrowserConfig::from_spec_str(s, url)
            }
            TomlBrowserTarget::Spec(spec) => spec.to_browser_config(url),
        }
    }
}

impl TomlBrowserSpec {
    pub fn to_browser_config(&self, url: &str) -> BrowserConfig {
        let app_type = autodetect_app_string_type(if self.name.is_empty() {
            None
        } else {
            Some(&self.name)
        });
        let (profile, container) = if self.container.is_some() {
            (self.profile.clone(), self.container.clone())
        } else {
            ProfileSelector::parse(&self.profile).split_owned()
        };
        BrowserConfig {
            name: self.name.clone(),
            app_type,
            open_in_background: self.open_in_background,
            profile,
            container,
            app_id: self.app_id.clone(),
            app_mode: self.app_mode,
            args: self.args.clone(),
            url: url.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrVec {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            StringOrVec::Single(s) => vec![s],
            StringOrVec::Multiple(v) => v,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TomlRewriteRule {
    #[serde(rename = "match")]
    pub match_patterns: Option<StringOrVec>,
    pub match_regex: Option<String>,
    #[serde(default)]
    pub strip_params: Vec<String>,
    pub force_https: Option<bool>,
    pub replace_host: Option<String>,
    pub replace_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlHandlerRule {
    #[serde(rename = "match")]
    pub match_patterns: Option<StringOrVec>,
    pub match_regex: Option<String>,
    pub match_hostnames: Option<StringOrVec>,
    pub opener_bundle_id: Option<StringOrVec>,
    pub opener_window_title: Option<StringOrVec>,
    pub browser: TomlBrowserTarget,
}

impl TomlConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| format!("Failed to parse TOML config: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_rules_file_load_save_roundtrip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("browsify_rules_test_{}.json", nanos));

        let original = RulesFile {
            default_browser: "Firefox".to_string(),
            default_profile: "Work".to_string(),
            options: None,
            rules: vec![
                Rule {
                    match_patterns: vec!["*github.com/*".to_string()],
                    browser: "Google Chrome".to_string(),
                    profile: "Personal".to_string(),
                },
                Rule {
                    match_patterns: vec![
                        "https://linear.app/*".to_string(),
                        "*.linear.app/*".to_string(),
                    ],
                    browser: "Safari".to_string(),
                    profile: String::new(),
                },
            ],
        };

        original.save_to_path(&path).unwrap();
        let raw_json = fs::read_to_string(&path).unwrap();
        // Single-entry match serializes as a plain string
        assert!(raw_json.contains(r#""match": "*github.com/*""#));

        let loaded = RulesFile::load_from_path(&path).unwrap();
        assert_eq!(loaded, original);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_missing_rules_file() {
        let loaded = RulesFile::load_from_path("/tmp/nonexistent_browsify_rules_12345.json").unwrap();
        assert!(loaded.rules.is_empty());
        assert_eq!(loaded.default_browser, "");
    }
}
