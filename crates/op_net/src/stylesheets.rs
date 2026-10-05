use crate::{
    LoadError, LoadedStylesheet, SourceKind, has_uri_scheme, looks_like_windows_path, resolve_link,
};
use std::io::Read;

pub(crate) const MAX_STYLESHEET_BYTES: usize = 1024 * 1024;

fn prefix(source: &str, value: &str) -> bool {
    source
        .get(..value.len())
        .is_some_and(|part| part.eq_ignore_ascii_case(value))
}

pub fn resolve_stylesheet_source(base: Option<&str>, source: &str) -> Result<String, LoadError> {
    let source = source.trim();
    if source.is_empty() || source.starts_with('#') {
        return Err(LoadError::InvalidLink("empty stylesheet source".into()));
    }
    if prefix(source, "data:") {
        return Ok(source.to_owned());
    }

    let local_base =
        base.is_some_and(|base| looks_like_windows_path(base) || !has_uri_scheme(base));
    let resolved = if local_base && (looks_like_windows_path(source) || prefix(source, "file:")) {
        source.to_owned()
    } else {
        resolve_link(base, source)?
    };

    if base.is_some_and(|base| prefix(base, "https://")) && prefix(&resolved, "http://") {
        return Err(LoadError::InvalidLink(
            "HTTPS stylesheet downgrade is blocked".into(),
        ));
    }
    Ok(resolved)
}

pub(super) fn load(source: &str, limit: usize) -> Result<LoadedStylesheet, LoadError> {
    let limit = limit.min(MAX_STYLESHEET_BYTES);
    if prefix(source, "http://") || prefix(source, "https://") {
        return super::http::load_stylesheet(source, limit);
    }
    if prefix(source, "data:") {
        return load_data_url(source, limit);
    }

    let path = if prefix(source, "file:") {
        super::file_url_path(source)?
    } else if looks_like_windows_path(source) || !has_uri_scheme(source) {
        std::path::PathBuf::from(source)
    } else {
        return Err(LoadError::UnsupportedScheme(
            source.split(':').next().unwrap_or_default().into(),
        ));
    };

    let io_error = |error: std::io::Error| LoadError::Io {
        path: path.clone(),
        message: error.to_string(),
    };
    let file = std::fs::File::open(&path).map_err(io_error)?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(LoadError::StylesheetTooLarge);
    }

    let address = path
        .canonicalize()
        .unwrap_or_else(|_| path.clone())
        .display()
        .to_string();
    let text = crate::encoding::decode_css(&bytes, None, &address)?;
    Ok(LoadedStylesheet {
        address,
        mime_type: "text/css".into(),
        text,
        source_kind: SourceKind::File,
    })
}

fn load_data_url(source: &str, limit: usize) -> Result<LoadedStylesheet, LoadError> {
    let data = &source[5..];
    let (metadata, payload) = data
        .split_once(',')
        .ok_or_else(|| LoadError::InvalidDataUrl("missing comma separator".into()))?;
    let mut parts = metadata.split(';');
    let media_type = parts.next().unwrap_or_default();
    if !media_type.eq_ignore_ascii_case("text/css") {
        return Err(LoadError::UnsupportedDataMime(media_type.to_owned()));
    }

    let mut base64 = false;
    let mut charset = None;
    for parameter in parts {
        if parameter.eq_ignore_ascii_case("base64") {
            if base64 {
                return Err(LoadError::InvalidDataUrl(
                    "duplicate base64 stylesheet parameter".into(),
                ));
            }
            base64 = true;
        } else if let Some((name, value)) = parameter.split_once('=')
            && name.trim().eq_ignore_ascii_case("charset")
        {
            if charset.is_none() {
                charset = Some(value.trim().trim_matches(['\'', '"']));
            }
        } else {
            return Err(LoadError::InvalidDataUrl(format!(
                "unsupported stylesheet parameter: {parameter}"
            )));
        }
    }

    if payload.len() > limit.saturating_mul(4) {
        return Err(LoadError::StylesheetTooLarge);
    }
    let payload = super::percent_decode_bytes(payload)
        .map_err(|message| LoadError::InvalidDataUrl(message.into()))?;
    let bytes = if base64 {
        let text = std::str::from_utf8(&payload)
            .map_err(|_| LoadError::InvalidDataUrl("invalid stylesheet base64 text".into()))?;
        super::decode_base64(text)?
    } else {
        payload
    };
    if bytes.len() > limit {
        return Err(LoadError::StylesheetTooLarge);
    }

    let text = crate::encoding::decode_css(&bytes, charset, "data stylesheet")?;
    Ok(LoadedStylesheet {
        address: "data:text/css".into(),
        mime_type: "text/css".into(),
        text,
        source_kind: SourceKind::DataUrl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_stylesheets_without_network_to_file_or_https_downgrade() {
        assert_eq!(
            resolve_stylesheet_source(Some("https://example.com/a/page"), "../site.css").unwrap(),
            "https://example.com/site.css"
        );
        assert!(
            resolve_stylesheet_source(
                Some("https://example.com/page"),
                "http://example.com/site.css"
            )
            .is_err()
        );
        assert!(
            resolve_stylesheet_source(Some("https://example.com/page"), "file:///C:/secret.css")
                .is_err()
        );
        assert!(resolve_stylesheet_source(None, "data:text/css,p%7Bcolor:red%7D").is_ok());
    }

    #[test]
    fn loads_bounded_css_data_url() {
        let loaded = load("data:text/css;charset=utf-8,p%7Bcolor%3Ared%7D", 64).unwrap();
        assert_eq!(loaded.text, "p{color:red}");
        assert_eq!(loaded.mime_type, "text/css");
        assert_eq!(loaded.source_kind, SourceKind::DataUrl);

        assert_eq!(
            load("data:text/css,0123456789", 4),
            Err(LoadError::StylesheetTooLarge)
        );
        assert!(load("data:text/html,p", 64).is_err());
    }
}
