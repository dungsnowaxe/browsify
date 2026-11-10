use regex::Regex;

/// Preprocesses Finicky / Browsify `.js` and `.ts` configuration source code so it can execute
/// directly inside QuickJS without bundling `esbuild` or `babel`.
///
/// Handles:
/// - Single-line and multi-line `import type { ... } from "...";`
/// - `import ... from "finicky"` / `"/Applications/Finicky.app/..."`
/// - Rewriting `export default <expr>` -> `globalThis.__browsifyDefaultExport = <expr>`
/// - Supporting CommonJS `module.exports = <expr>`
/// - Stripping TypeScript variable annotations (`const x: SomeType =`), parameter annotations
///   (`(url: URL, options: OpenUrlOptions)`), `as <Type>`, and `satisfies <Type>`.
pub fn preprocess_js_ts_config(source: &str) -> String {
    // 1. Remove multi-line `import type ... from ...;`
    let import_type_re = Regex::new(r"(?ms)^\s*import\s+type\s+.*?from\s+['\x22].*?['\x22]\s*;?").unwrap();
    let without_import_type = import_type_re.replace_all(source, "");

    // 2. Remove single-line finicky imports
    let mut lines_out = Vec::new();
    for line in without_import_type.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") && (trimmed.contains("finicky") || trimmed.contains("Finicky.app")) {
            lines_out.push(String::new());
            continue;
        }
        lines_out.push(line.to_string());
    }

    let joined = lines_out.join("\n");
    let stripped_ts = strip_ts_syntax(&joined);

    // 3. Replace `export default` with assignment to `globalThis.__browsifyDefaultExport =`
    //    and wrap in an IIFE so top-level `const`/`let` declarations remain isolated.
    let mut out = String::with_capacity(stripped_ts.len() + 96);
    out.push_str("(function() {\n");
    for line in stripped_ts.lines() {
        let trimmed_start = line.trim_start();
        if let Some(rest) = trimmed_start.strip_prefix("export default ") {
            let indent = &line[..line.len() - trimmed_start.len()];
            out.push_str(indent);
            out.push_str("globalThis.__browsifyDefaultExport = ");
            out.push_str(rest);
            out.push('\n');
        } else if trimmed_start == "export default" {
            let indent = &line[..line.len() - trimmed_start.len()];
            out.push_str(indent);
            out.push_str("globalThis.__browsifyDefaultExport =\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("})();\n");
    out
}

fn strip_ts_syntax(input: &str) -> String {
    let mut s = input.to_string();

    // Strip `const/let/var ident: TypeName =` -> `const/let/var ident =`
    let var_type_re =
        Regex::new(r"\b(const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*[A-Za-z0-9_$.<>|\[\]\s]+\s*=").unwrap();
    s = var_type_re.replace_all(&s, "$1 $2 =").into_owned();

    // Strip `satisfies TypeName`
    let satisfies_re = Regex::new(r"\bsatisfies\s+[A-Za-z_$][A-Za-z0-9_$.<>]*").unwrap();
    s = satisfies_re.replace_all(&s, "").into_owned();

    // Strip common parameter & `as` annotations
    for pat in [
        ": FinickyURL",
        ": URL",
        ": OpenUrlOptions",
        ": ProcessInfo",
        ": Config",
        ": FinickyConfig",
        ": BrowserHandler",
        ": string",
        ": boolean",
        ": number",
        " as any",
        " as Config",
        " as FinickyConfig",
        " as const",
    ] {
        s = s.replace(pat, "");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_ts_and_esm() {
        let src = r#"
import type {
  BrowserHandler,
  FinickyConfig,
} from "/Applications/Finicky.app/Contents/Resources/finicky.d.ts";

const handler: BrowserHandler = {
  match: (url: URL, { opener }) => url.host.includes("workplace"),
  browser: "Firefox",
};

export default {
  defaultBrowser: "Safari",
  handlers: [handler],
} satisfies FinickyConfig;
"#;
        let out = preprocess_js_ts_config(src);
        assert!(out.contains("globalThis.__browsifyDefaultExport = {"));
        assert!(out.contains("const handler = {"));
        assert!(!out.contains("import type"));
        assert!(!out.contains(": URL"));
        assert!(!out.contains("satisfies FinickyConfig"));
    }
}
