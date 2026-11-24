use crate::{compile_json_rules, BrowserConfig, NativeRouter, RulesFile};

/// Render a literal destination so an alias in the receiving TOML cannot change it.
pub fn toml_browser_target(target: &BrowserConfig) -> String {
    if target.name.is_empty() {
        return "\"none\"".into();
    }
    let mut fields = toml::map::Map::new();
    fields.insert("name".into(), target.name.clone().into());
    if !target.profile.is_empty() {
        fields.insert("profile".into(), target.profile.clone().into());
    }
    for (key, val) in [("container", &target.container), ("app_id", &target.app_id)] {
        if let Some(val) = val {
            fields.insert(key.into(), val.clone().into());
        }
    }
    for (key, val) in [
        ("open_in_background", target.open_in_background),
        ("app_mode", target.app_mode),
    ] {
        if let Some(val) = val {
            fields.insert(key.into(), val.into());
        }
    }
    if !target.args.is_empty() {
        fields.insert("args".into(), toml::Value::from(target.args.clone()));
    }
    toml::Value::Table(fields).to_string()
}

/// Converts legacy rules with the same target parsing and match ordering used at runtime.
pub fn rules_file_to_toml(rules: &RulesFile) -> String {
    let router = NativeRouter::from_rules_file(rules);
    let mut out = format!(
        "default_browser = {}\n",
        toml_browser_target(&router.default_browser)
    );
    if let Some(opts) = &rules.options {
        out.push_str("\n[options]\n");
        for (key, setting) in [
            ("keep_running", opts.keep_running),
            ("hide_icon", opts.hide_icon),
            ("log_requests", opts.log_requests),
            ("check_for_updates", opts.check_for_updates),
        ] {
            if let Some(setting) = setting {
                out.push_str(&format!("{key} = {setting}\n"));
            }
        }
    }
    for rule in &rules.rules {
        let Some(compiled) = compile_json_rules(std::slice::from_ref(rule)).pop() else {
            continue;
        };
        let patterns: Vec<_> = rule
            .match_patterns
            .iter()
            .filter(|p| !p.is_empty())
            .cloned()
            .collect();
        let matcher = match patterns.as_slice() {
            [single] => toml::Value::String(single.clone()),
            _ => toml::Value::from(patterns),
        };
        out.push_str(&format!(
            "\n[[handlers]]\nmatch = {matcher}\nbrowser = {}\n",
            toml_browser_target(&compiled.target)
        ));
    }
    out
}
