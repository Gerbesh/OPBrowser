//! Bounded classic-script source loading with script-specific filtering.
//! External script URLs intentionally use a same-origin subset for M4.2.
use crate::{LoadError, has_uri_scheme, looks_like_windows_path, resolve_link};
use std::io::Read;

const MAX_SCRIPT_BYTES: usize = 128 * 1024;

fn starts(source: &str, prefix: &str) -> bool {
    source
        .get(..prefix.len())
        .is_some_and(|v| v.eq_ignore_ascii_case(prefix))
}

pub fn resolve_script_source(base: &str, src: &str) -> Result<String, LoadError> {
    let src = src.trim();
    if src.is_empty()
        || src.starts_with('#')
        || starts(src, "data:")
        || starts(src, "file:")
        || looks_like_windows_path(src)
    {
        return Err(LoadError::InvalidLink(
            "unsupported external script URL".into(),
        ));
    }
    if src.contains('\\') || src.chars().any(char::is_control) {
        return Err(LoadError::InvalidLink("invalid script source".into()));
    }
    let resolved = resolve_link(Some(base), src)?;
    enforce_same_origin(base, &resolved)?;
    Ok(resolved)
}

pub(super) fn enforce_same_origin(page: &str, script: &str) -> Result<(), LoadError> {
    let page_http = starts(page, "https://") || starts(page, "http://");
    let script_http = starts(script, "https://") || starts(script, "http://");
    if page_http || script_http {
        if page_http && script_http && crate::http::same_origin(page, script) {
            return Ok(());
        }
    } else if (looks_like_windows_path(page) || !has_uri_scheme(page))
        && (looks_like_windows_path(script) || !has_uri_scheme(script))
    {
        return Ok(());
    }
    Err(LoadError::InvalidLink(
        "external script must be same-origin".into(),
    ))
}

pub(super) fn load(source: &str, page: &str, limit: usize) -> Result<String, LoadError> {
    let limit = limit.min(MAX_SCRIPT_BYTES);
    if starts(source, "http://") || starts(source, "https://") {
        // WinHTTP follows redirects; reject cross-origin final URLs too.
        let (final_url, text) = crate::http::load_script(source, limit)?;
        enforce_same_origin(page, &final_url)?;
        return Ok(text);
    }
    if has_uri_scheme(source) && !looks_like_windows_path(source) {
        return Err(LoadError::UnsupportedScheme(
            source.split(':').next().unwrap_or_default().into(),
        ));
    }
    let path = std::path::PathBuf::from(source);
    let file = std::fs::File::open(&path).map_err(|error| LoadError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| LoadError::Io {
            path: path.clone(),
            message: error.to_string(),
        })?;
    if bytes.len() > limit {
        return Err(LoadError::ScriptTooLarge);
    }
    crate::encoding::decode_script(&bytes, None, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_urls_resolve_relative_paths_without_origin_escalation() {
        assert_eq!(
            resolve_script_source("https://example.test/a/index.html", "./code.js").unwrap(),
            "https://example.test/a/code.js"
        );
        assert_eq!(
            resolve_script_source(
                "https://example.test:443/page",
                "https://EXAMPLE.test/test.js"
            )
            .unwrap(),
            "https://EXAMPLE.test/test.js"
        );
        for candidate in [
            "http://example.test/a.js",
            "https://other.test/a.js",
            "file:///C:/secrets.js",
            "data:text/javascript,1",
            "C:\\secrets.js",
            "javascript:alert(1)",
        ] {
            assert!(
                resolve_script_source("https://example.test/page", candidate).is_err(),
                "{candidate}"
            );
        }
        assert!(
            resolve_script_source("C:\\web\\page.html", "run.js")
                .unwrap()
                .ends_with("web\\run.js")
        );
        assert!(
            resolve_script_source("C:\\web\\page.html", "https://example.test/run.js").is_err()
        );
    }

    #[test]
    fn local_scripts_are_bounded_and_unicode_decoded() {
        let filename = std::env::temp_dir().join(format!(
            "opbrowser-script-{}-{}.js",
            std::process::id(),
            68142
        ));
        std::fs::write(&filename, "const message = 'Привет';").unwrap();
        let path = filename.display().to_string();
        assert!(load(&path, &path, 10).is_err());
        assert_eq!(
            load(&path, &path, 128).unwrap(),
            "const message = 'Привет';"
        );
        std::fs::remove_file(filename).unwrap();
    }
}
