use std::process::Command;
use url::Url;

pub const SHORTENER_DOMAINS: &[&str] = &[
    "adf.ly",
    "b.link",
    "bit.do",
    "bit.ly",
    "bl.ink",
    "buff.ly",
    "cutt.ly",
    "ClickMeter.com",
    "db.tt",
    "dub.co",
    "dub.sh",
    "goo.gl",
    "is.gd",
    "ity.im",
    "j.mp",
    "kutt.it",
    "lnkd.in",
    "ow.ly",
    "po.st",
    "qlink.me",
    "rb.gy",
    "rebrand.ly",
    "short.cm",
    "short.io",
    "shorturl.at",
    "s.id",
    "t.co",
    "t.ly",
    "tiny.cc",
    "tiny.pl",
    "tinyurl.com",
    "tr.im",
    "u.to",
    "v.gd",
    "vzturl.com",
    "x.co",
    "yourls.org",
    "zpr.io",
];

/// Checks whether `url_str` belongs to a known URL shortener domain (or any custom `extra_domains`).
pub fn is_short_url(url_str: &str, extra_domains: &[String]) -> bool {
    let Ok(parsed) = Url::parse(url_str) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let matches_domain = |domain: &str| {
        host.eq_ignore_ascii_case(domain)
            || (host.len() > domain.len()
                && host
                    .to_ascii_lowercase()
                    .ends_with(&format!(".{}", domain.to_ascii_lowercase())))
    };
    SHORTENER_DOMAINS.iter().any(|d| matches_domain(d))
        || extra_domains.iter().any(|d| matches_domain(d))
}

/// Resolves a shortened URL by following up to 3 HTTP redirects with a 750ms timeout.
/// Falls back to `original_url` on any network error or if the domain is not a known shortener.
pub fn resolve_short_url(original_url: &str, extra_domains: &[String]) -> String {
    if !is_short_url(original_url, extra_domains) {
        return original_url.to_string();
    }

    let output = Command::new("curl")
        .args([
            "-sIL",
            "--max-redirs",
            "3",
            "--max-time",
            "0.75",
            "-A",
            "Browsify/0.1",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            original_url,
        ])
        .output();

    if let Ok(out) = output {
        let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !resolved.is_empty() && Url::parse(&resolved).is_ok() {
            return resolved;
        }
    }
    original_url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_url_detection() {
        assert!(is_short_url("https://t.co/abc123", &[]));
        assert!(is_short_url("https://bit.ly/xyz", &[]));
        assert!(is_short_url(
            "https://go.internal.corp/link",
            &["go.internal.corp".to_string()]
        ));
        assert!(!is_short_url("https://github.com/snowaxe/browsify", &[]));
    }
}
