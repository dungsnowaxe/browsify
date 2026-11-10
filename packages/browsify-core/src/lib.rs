pub mod browser;
pub mod migrate;
pub mod profile;
pub mod router;
pub mod rules;
pub mod shorturl;
pub mod sqlite_profiles;
pub mod wildcard;

use serde::{Deserialize, Serialize};

pub use browser::{
    autodetect_app_string_type, lookup_browser_meta, AppType, BrowserConfig, BrowserFamily,
    BrowserMeta, ProfileSelector, BROWSERS_REGISTRY,
};
pub use migrate::rules_file_to_toml;
pub use profile::{MissingProfilePolicy, ProfileResolver, ResolvedLaunchPlan};
pub use router::{
    compile_json_rules, decode_protocol_url, ExplicitOptions, inherit_rules_file_options,
    rules_file_default_browser, CompiledHandler, CompiledMatcher, CompiledRewrite, CompiledTransform,
    ConfigOptions, HostnamePattern, NativeRouter,
};
pub use rules::{
    Rule, RulesFile, RulesOptions, TomlBrowserSpec, TomlBrowserTarget, TomlConfig, TomlHandlerRule,
    TomlOptions, TomlRewriteRule,
};
pub use shorturl::{is_short_url, resolve_short_url, SHORTENER_DOMAINS};
pub use wildcard::{compile_wildcard, match_wildcard};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenerInfo {
    pub name: String,
    pub bundle_id: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    /// Process id of the opener, used to lazily fetch `windowTitle` on demand.
    #[serde(default)]
    pub pid: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierKeys {
    pub shift: bool,
    pub option: bool,
    pub command: bool,
    pub control: bool,
    pub caps_lock: bool,
    #[serde(rename = "fn")]
    pub fn_key: bool,
    pub function: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub localized_name: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerInfo {
    pub is_charging: bool,
    pub is_connected: bool,
    pub percentage: Option<i32>,
}
