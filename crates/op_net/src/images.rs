use crate::{LoadError, has_uri_scheme, looks_like_windows_path, resolve_link};
use std::io::Read;

const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;

fn prefix(source: &str, value: &str) -> bool {
    source
        .get(..value.len())
        .is_some_and(|part| part.eq_ignore_ascii_case(value))
}

pub fn resolve_image_source(base: Option<&str>, source: &str) -> Result<String, LoadError> {
    let source = source.trim();
    if source.is_empty() || source.starts_with('#') {
        return Err(LoadError::InvalidLink("empty image source".into()));
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
            "HTTPS image downgrade is blocked".into(),
        ));
    }
    Ok(resolved)
}

pub(super) fn load(source: &str, limit: usize) -> Result<Vec<u8>, LoadError> {
    let limit = limit.min(MAX_IMAGE_BYTES);
    if prefix(source, "http://") || prefix(source, "https://") {
        return super::http::load_image(source, limit);
    }
    if prefix(source, "data:") {
        let (metadata, payload) = source[5..]
            .split_once(',')
            .ok_or_else(|| LoadError::InvalidDataUrl("missing comma".into()))?;
        let mut parts = metadata.split(';');
        let mime = parts.next().unwrap_or_default().to_ascii_lowercase();
        if !matches!(
            mime.as_str(),
            "image/png" | "image/jpeg" | "image/gif" | "image/bmp"
        ) {
            return Err(LoadError::UnsupportedDataMime(mime));
        }
        let mut base64 = false;
        for part in parts {
            if part.eq_ignore_ascii_case("base64") && !base64 {
                base64 = true;
            } else {
                return Err(LoadError::InvalidDataUrl(
                    "unsupported image parameter".into(),
                ));
            }
        }
        // Bound the escaped input too, before allocating percent/base64 buffers.
        if payload.len() > limit.saturating_mul(4) {
            return Err(LoadError::ImageTooLarge);
        }
        let payload = super::percent_decode_bytes(payload)
            .map_err(|e| LoadError::InvalidDataUrl(e.into()))?;
        let bytes = if base64 {
            let text = std::str::from_utf8(&payload)
                .map_err(|_| LoadError::InvalidDataUrl("invalid base64 text".into()))?;
            super::decode_base64(text)?
        } else {
            payload
        };
        if bytes.len() > limit {
            return Err(LoadError::ImageTooLarge);
        }
        return Ok(bytes);
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
    let io_error = |e: std::io::Error| LoadError::Io {
        path: path.clone(),
        message: e.to_string(),
    };
    let file = std::fs::File::open(&path).map_err(io_error)?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(LoadError::ImageTooLarge);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_images_without_file_access_or_https_downgrade_from_network_pages() {
        assert_eq!(
            resolve_image_source(Some("https://example.com/a/page"), "../pic.png").unwrap(),
            "https://example.com/pic.png"
        );
        for source in [
            "file:///C:/secret.png",
            "C:\\secret.png",
            "http://example.com/pic.png",
            "javascript:x",
            "#id",
            "",
        ] {
            assert!(
                resolve_image_source(Some("https://example.com/page"), source).is_err(),
                "{source}"
            );
        }
        assert!(resolve_image_source(Some("data:text/html"), "pic.png").is_err());
        assert!(resolve_image_source(None, "data:image/png;base64,AA==").is_ok());
    }

    #[test]
    fn bounds_and_validates_image_data_urls() {
        assert_eq!(
            load("data:image/png;base64,AAECAw==", 4).unwrap(),
            [0, 1, 2, 3]
        );
        assert_eq!(load("data:image/gif,%00%FF", 2).unwrap(), [0, 255]);
        assert_eq!(
            load("data:image/png;base64,AAECAw==", 3),
            Err(LoadError::ImageTooLarge)
        );
        assert!(load("data:text/html,x", 10).is_err());
        assert!(load("data:image/png;bad,x", 10).is_err());
    }
}
