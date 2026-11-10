use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppType {
    AppName,
    BundleId,
    Path,
    None,
}

impl AppType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppType::AppName => "appName",
            AppType::BundleId => "bundleId",
            AppType::Path => "path",
            AppType::None => "none",
        }
    }
}

impl Default for AppType {
    fn default() -> Self {
        AppType::AppName
    }
}

/// Autodetects whether a string refers to an application name, bundle ID, or `.app` path.
/// Matches `autodetectAppStringType` in `packages/config-api/src/index.ts`.
pub fn autodetect_app_string_type(app: Option<&str>) -> AppType {
    let Some(s) = app else {
        return AppType::None;
    };
    if s.is_empty() {
        return AppType::None;
    }

    // Alphanumeric + spaces -> appName
    if s.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ') {
        return AppType::AppName;
    }

    // Alphanumeric + '-' + '.' -> bundleId
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return AppType::BundleId;
    }

    // App path: `~?(/...)+/....app`
    if s.ends_with(".app") {
        let rest = s.strip_prefix('~').unwrap_or(s);
        if rest.starts_with('/') && !rest.contains('\n') {
            let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
            if parts.len() >= 2 {
                return AppType::Path;
            }
        }
    }

    AppType::AppName
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserConfig {
    pub name: String,
    #[serde(default)]
    pub app_type: AppType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_in_background: Option<bool>,
    #[serde(default)]
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_mode: Option<bool>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: String,
}

impl BrowserConfig {
    pub fn none(url: impl Into<String>) -> Self {
        Self {
            name: String::new(),
            app_type: AppType::None,
            open_in_background: None,
            profile: String::new(),
            container: None,
            app_id: None,
            app_mode: None,
            args: Vec::new(),
            url: url.into(),
        }
    }

    pub fn from_spec_str(spec: &str, url: impl Into<String>) -> Self {
        let (name_part, profile_part) = match spec.split_once(':') {
            Some((n, p)) if !s_is_path(spec) => (n, p),
            _ => (spec, ""),
        };
        let app_type = autodetect_app_string_type(Some(name_part));
        let (profile, container) = if let Some(c) = profile_part.strip_prefix("container:") {
            (String::new(), (!c.is_empty()).then(|| c.to_string()))
        } else {
            (profile_part.to_string(), None)
        };
        Self {
            name: name_part.to_string(),
            app_type,
            open_in_background: None,
            profile,
            container,
            app_id: None,
            app_mode: None,
            args: Vec::new(),
            url: url.into(),
        }
    }

    pub fn safari_default(url: impl Into<String>, open_in_background: bool) -> Self {
        Self {
            name: "com.apple.Safari".to_string(),
            app_type: AppType::BundleId,
            open_in_background: Some(open_in_background),
            profile: String::new(),
            container: None,
            app_id: None,
            app_mode: None,
            args: Vec::new(),
            url: url.into(),
        }
    }
}

fn s_is_path(s: &str) -> bool {
    s.starts_with('/') || s.starts_with("~/")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserFamily {
    Chromium,
    Firefox,
    Safari,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserMeta {
    pub id: &'static str,
    pub app_name: &'static str,
    pub config_dir_relative: &'static str,
    pub family: BrowserFamily,
}

pub const BROWSERS_REGISTRY: &[BrowserMeta] = &[
    // Chromium family
    BrowserMeta {
        id: "com.google.Chrome",
        app_name: "Google Chrome",
        config_dir_relative: "Google/Chrome",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.google.Chrome.beta",
        app_name: "Google Chrome Beta",
        config_dir_relative: "Google/Chrome Beta",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.google.Chrome.dev",
        app_name: "Google Chrome Dev",
        config_dir_relative: "Google/Chrome Dev",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.google.Chrome.canary",
        app_name: "Google Chrome Canary",
        config_dir_relative: "Google/Chrome Canary",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.brave.Browser",
        app_name: "Brave Browser",
        config_dir_relative: "BraveSoftware/Brave-Browser",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.brave.Browser.beta",
        app_name: "Brave Browser Beta",
        config_dir_relative: "BraveSoftware/Brave-Browser-Beta",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.brave.Browser.nightly",
        app_name: "Brave Browser Nightly",
        config_dir_relative: "BraveSoftware/Brave-Browser-Nightly",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.microsoft.edgemac",
        app_name: "Microsoft Edge",
        config_dir_relative: "Microsoft Edge",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.microsoft.edgemac.Beta",
        app_name: "Microsoft Edge Beta",
        config_dir_relative: "Microsoft Edge Beta",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.microsoft.edgemac.Dev",
        app_name: "Microsoft Edge Dev",
        config_dir_relative: "Microsoft Edge Dev",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.vivaldi.Vivaldi",
        app_name: "Vivaldi",
        config_dir_relative: "Vivaldi",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "org.chromium.Chromium",
        app_name: "Chromium",
        config_dir_relative: "Chromium",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.bookry.wavebox",
        app_name: "Wavebox",
        config_dir_relative: "WaveboxApp",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "net.imput.helium",
        app_name: "Helium",
        config_dir_relative: "net.imput.helium",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "ai.perplexity.comet",
        app_name: "Comet",
        config_dir_relative: "Comet",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "ru.yandex.desktop.yandex-browser",
        app_name: "Yandex",
        config_dir_relative: "Yandex/YandexBrowser",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.operasoftware.Opera",
        app_name: "Opera",
        config_dir_relative: "com.operasoftware.Opera",
        family: BrowserFamily::Chromium,
    },
    BrowserMeta {
        id: "com.operasoftware.OperaGX",
        app_name: "Opera GX",
        config_dir_relative: "com.operasoftware.OperaGX",
        family: BrowserFamily::Chromium,
    },
    // Dia (Chromium family)
    BrowserMeta {
        id: "company.thebrowser.dia",
        app_name: "Dia",
        config_dir_relative: "Dia/User Data",
        family: BrowserFamily::Chromium,
    },
    // Firefox family
    BrowserMeta {
        id: "org.mozilla.firefox",
        app_name: "Firefox",
        config_dir_relative: "Firefox",
        family: BrowserFamily::Firefox,
    },
    BrowserMeta {
        id: "org.mozilla.firefoxdeveloperedition",
        app_name: "Firefox Developer Edition",
        config_dir_relative: "Firefox",
        family: BrowserFamily::Firefox,
    },
    BrowserMeta {
        id: "org.mozilla.nightly",
        app_name: "Firefox Nightly",
        config_dir_relative: "Firefox",
        family: BrowserFamily::Firefox,
    },
    BrowserMeta {
        id: "app.zen-browser.zen",
        app_name: "Zen",
        config_dir_relative: "zen",
        family: BrowserFamily::Firefox,
    },
    BrowserMeta {
        id: "net.waterfox.waterfox",
        app_name: "Waterfox",
        config_dir_relative: "Waterfox",
        family: BrowserFamily::Firefox,
    },
    BrowserMeta {
        id: "io.gitlab.librewolf-community",
        app_name: "LibreWolf",
        config_dir_relative: "LibreWolf",
        family: BrowserFamily::Firefox,
    },
    // Safari
    BrowserMeta {
        id: "com.apple.Safari",
        app_name: "Safari",
        config_dir_relative: "",
        family: BrowserFamily::Safari,
    },
];

pub fn lookup_browser_meta(identifier: &str) -> Option<&'static BrowserMeta> {
    BROWSERS_REGISTRY.iter().find(|b| {
        b.id.eq_ignore_ascii_case(identifier) || b.app_name.eq_ignore_ascii_case(identifier)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autodetect_app_string_type() {
        assert_eq!(
            autodetect_app_string_type(Some("Google Chrome")),
            AppType::AppName
        );
        assert_eq!(
            autodetect_app_string_type(Some("Firefox")),
            AppType::AppName
        );
        assert_eq!(autodetect_app_string_type(Some("Safari")), AppType::AppName);
        assert_eq!(
            autodetect_app_string_type(Some("Chrome 123")),
            AppType::AppName
        );

        assert_eq!(
            autodetect_app_string_type(Some("com.google.Chrome")),
            AppType::BundleId
        );
        assert_eq!(
            autodetect_app_string_type(Some("org.mozilla.firefox")),
            AppType::BundleId
        );
        assert_eq!(
            autodetect_app_string_type(Some("com.apple.Safari")),
            AppType::BundleId
        );

        assert_eq!(
            autodetect_app_string_type(Some("/Applications/Google Chrome.app")),
            AppType::Path
        );
        assert_eq!(
            autodetect_app_string_type(Some("~/Applications/Firefox.app")),
            AppType::Path
        );
        assert_eq!(
            autodetect_app_string_type(Some("/Users/username/Applications/Custom Browser.app")),
            AppType::Path
        );

        assert_eq!(autodetect_app_string_type(None), AppType::None);
        assert_eq!(
            autodetect_app_string_type(Some("Invalid/Path")),
            AppType::AppName
        );
        assert_eq!(
            autodetect_app_string_type(Some("com.example!invalid")),
            AppType::AppName
        );
    }

    #[test]
    fn test_colon_profile_spec() {
        let cfg = BrowserConfig::from_spec_str("Chrome:Work", "https://example.com/");
        assert_eq!(cfg.name, "Chrome");
        assert_eq!(cfg.profile, "Work");
        assert_eq!(cfg.app_type, AppType::AppName);
    }
}
