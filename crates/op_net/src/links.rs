use crate::{LoadError, has_uri_scheme, looks_like_windows_path};

/// Resolve the initial link-navigation subset. Network pages cannot reference
/// local files, and unsupported schemes never launch another application.
pub fn resolve_link(base: Option<&str>, href: &str) -> Result<String, LoadError> {
    let href = href.trim();
    if has_uri_scheme(href) {
        let scheme = href.split_once(':').unwrap().0.to_ascii_lowercase();
        if scheme == "http" || scheme == "https" {
            super::http::validate_url(href)?;
            return Ok(href.to_owned());
        }
        return Err(LoadError::UnsupportedScheme(scheme));
    }
    let base =
        base.ok_or_else(|| LoadError::InvalidLink("relative link has no document base".into()))?;
    if base
        .get(..7)
        .is_some_and(|p| p.eq_ignore_ascii_case("http://"))
        || base
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
    {
        let (scheme, rest) = base.split_once("://").unwrap();
        if href.starts_with("//") {
            let result = format!("{scheme}:{href}");
            super::http::validate_url(&result)?;
            return Ok(result);
        }
        if href.contains('\\') || href.chars().any(|c| c.is_control()) {
            return Err(LoadError::InvalidLink(href.to_owned()));
        }
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let origin = format!("{scheme}://{}", &rest[..end]);
        let base_target = &rest[end..];
        let base_target = if base_target.starts_with('/') {
            base_target.to_owned()
        } else {
            format!("/{base_target}")
        };
        let base_without_fragment = base_target.split('#').next().unwrap_or_default();
        let result = if href.is_empty() || href.starts_with('#') {
            format!("{origin}{base_without_fragment}{href}")
        } else if href.starts_with('?') {
            let path = base_without_fragment.split('?').next().unwrap_or("/");
            format!("{origin}{path}{href}")
        } else {
            let path = base_without_fragment.split('?').next().unwrap_or("/");
            let joined = if href.starts_with('/') {
                href.to_owned()
            } else {
                format!(
                    "{}{href}",
                    path.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned() + "/"
                )
            };
            let suffix_start = joined.find(['?', '#']).unwrap_or(joined.len());
            format!(
                "{origin}{}{}",
                normalize_path(&joined[..suffix_start]),
                &joined[suffix_start..]
            )
        };
        super::http::validate_url(&result)?;
        return Ok(result);
    }
    if looks_like_windows_path(base) || !has_uri_scheme(base) {
        if href.starts_with(['/', '\\']) || href.chars().any(|c| c.is_control()) {
            return Err(LoadError::InvalidLink(
                "absolute local/UNC link is unsupported".into(),
            ));
        }
        let href = href.split(['?', '#']).next().unwrap_or_default();
        if href.is_empty() {
            return Ok(base.to_owned());
        }
        let decoded = super::percent_decode(href)
            .map_err(|message| LoadError::InvalidLink(message.into()))?;
        // Decoding must not turn a relative href into an absolute/UNC path.
        if decoded.starts_with(['/', '\\'])
            || looks_like_windows_path(&decoded)
            || decoded.contains(':')
            || decoded.contains('\0')
        {
            return Err(LoadError::InvalidLink(href.to_owned()));
        }
        let path = std::path::Path::new(base)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        return Ok(path.join(decoded).display().to_string());
    }
    Err(LoadError::InvalidLink(
        "this document source has no relative link base".into(),
    ))
}

fn normalize_path(path: &str) -> String {
    let mut segments = Vec::new();
    let mut trailing = path.ends_with('/');
    for part in path.strip_prefix('/').unwrap_or(path).split('/') {
        // Recognize percent-encoded dot segments without decoding escaped slashes.
        let dot = part.to_ascii_lowercase().replace("%2e", ".");
        match dot.as_str() {
            "." => trailing = true,
            ".." => {
                segments.pop();
                trailing = true;
            }
            _ => {
                segments.push(part);
                trailing = part.is_empty();
            }
        }
    }
    let mut result = format!("/{}", segments.join("/"));
    if trailing && !result.ends_with('/') {
        result.push('/');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_http_reference_forms() {
        let base = Some("https://example.com:8443/a/b/index.html?old=1");
        for (href, expected) in [
            ("next.html", "https://example.com:8443/a/b/next.html"),
            ("../page?x=1#id", "https://example.com:8443/a/page?x=1#id"),
            ("/root", "https://example.com:8443/root"),
            ("//other.example/a", "https://other.example/a"),
            ("?new=2", "https://example.com:8443/a/b/index.html?new=2"),
            ("#id", "https://example.com:8443/a/b/index.html?old=1#id"),
            ("", "https://example.com:8443/a/b/index.html?old=1"),
            ("%2e%2e/next", "https://example.com:8443/a/next"),
            ("../../../next", "https://example.com:8443/next"),
            ("./", "https://example.com:8443/a/b/"),
        ] {
            assert_eq!(resolve_link(base, href).unwrap(), expected, "{href}");
        }
        assert_eq!(
            resolve_link(None, "https://example.com/").unwrap(),
            "https://example.com/"
        );
        assert_eq!(
            resolve_link(Some("https://example.com?old=1"), "next").unwrap(),
            "https://example.com/next"
        );
        assert_eq!(
            resolve_link(Some("http://[::1]:8080/a/b"), "../c").unwrap(),
            "http://[::1]:8080/c"
        );
        assert_eq!(
            resolve_link(base, "../%2Fname?x=..").unwrap(),
            "https://example.com:8443/a/%2Fname?x=.."
        );
    }

    #[test]
    fn rejects_unsupported_links_and_missing_bases() {
        for href in [
            "javascript:alert(1)",
            "file:///C:/private.html",
            "mailto:test@example.com",
            "data:text/html,test",
            "ftp://example.com",
        ] {
            assert!(matches!(
                resolve_link(Some("https://example.com/"), href),
                Err(LoadError::UnsupportedScheme(_))
            ));
        }
        assert!(resolve_link(None, "relative").is_err());
        assert!(resolve_link(Some("data:text/html"), "relative").is_err());
        assert!(resolve_link(Some("https://example.com/"), "..\\private").is_err());
    }

    #[test]
    fn resolves_relative_local_files_without_decoding_into_absolute_paths() {
        let base = std::env::temp_dir()
            .join("opbrowser-links")
            .join("index.html");
        let expected = base.parent().unwrap().join("next page.html");
        assert_eq!(
            resolve_link(base.to_str(), "next%20page.html#section").unwrap(),
            expected.display().to_string()
        );
        for href in [
            "%2Fabsolute.html",
            "%5C%5Cserver/path",
            "%43%3A/private",
            "bad%00name",
        ] {
            assert!(resolve_link(base.to_str(), href).is_err());
        }
    }
}
