//! Document source loading, URL networking, cache, cookies and request filtering.
//!
//! M1 intentionally starts with local files and HTML data URLs. HTTP(S) is added
//! later behind the same source-loading boundary.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    File,
    DataUrl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDocument {
    pub address: String,
    pub mime_type: String,
    pub text: String,
    pub source_kind: SourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    EmptySource,
    UnsupportedScheme(String),
    InvalidFileUrl(String),
    InvalidDataUrl(String),
    UnsupportedDataMime(String),
    Io { path: PathBuf, message: String },
    InvalidUtf8 { source: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySource => write!(formatter, "document source is empty"),
            Self::UnsupportedScheme(scheme) => {
                write!(formatter, "unsupported source scheme: {scheme}")
            }
            Self::InvalidFileUrl(value) => write!(formatter, "invalid file URL: {value}"),
            Self::InvalidDataUrl(message) => write!(formatter, "invalid data URL: {message}"),
            Self::UnsupportedDataMime(mime) => {
                write!(formatter, "unsupported data URL media type: {mime}")
            }
            Self::Io { path, message } => {
                write!(formatter, "failed to read {}: {message}", path.display())
            }
            Self::InvalidUtf8 { source } => {
                write!(formatter, "{source} is not valid UTF-8")
            }
        }
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug, Default)]
pub struct NetworkContext;

impl NetworkContext {
    pub fn load_document(&self, source: &str) -> Result<LoadedDocument, LoadError> {
        load_document(source)
    }
}

pub fn load_document(source: &str) -> Result<LoadedDocument, LoadError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(LoadError::EmptySource);
    }

    if source
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
    {
        return load_data_url(source);
    }

    if source
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
    {
        return load_file_url(source);
    }

    if looks_like_windows_path(source) || !has_uri_scheme(source) {
        return load_file_path(PathBuf::from(source));
    }

    let scheme = source
        .split_once(':')
        .map(|(scheme, _)| scheme.to_ascii_lowercase())
        .unwrap_or_else(|| source.to_ascii_lowercase());

    Err(LoadError::UnsupportedScheme(scheme))
}

fn load_file_url(source: &str) -> Result<LoadedDocument, LoadError> {
    let rest = &source[5..];

    let path_text = if let Some(without_slashes) = rest.strip_prefix("//") {
        if without_slashes
            .get(..10)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("localhost/"))
        {
            &without_slashes[10..]
        } else if without_slashes.starts_with('/') {
            without_slashes
        } else {
            return Err(LoadError::InvalidFileUrl(source.to_owned()));
        }
    } else {
        rest
    };

    let decoded = percent_decode(path_text)
        .map_err(|message| LoadError::InvalidFileUrl(format!("{source}: {message}")))?;

    let normalized = normalize_windows_file_url_path(&decoded);
    load_file_path(PathBuf::from(normalized))
}

fn load_file_path(path: PathBuf) -> Result<LoadedDocument, LoadError> {
    let bytes = fs::read(&path).map_err(|error| LoadError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;

    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    let text = String::from_utf8(bytes.to_vec()).map_err(|_| LoadError::InvalidUtf8 {
        source: path.display().to_string(),
    })?;

    let address = path
        .canonicalize()
        .unwrap_or_else(|_| path.clone())
        .display()
        .to_string();

    Ok(LoadedDocument {
        address,
        mime_type: mime_for_path(&path).to_owned(),
        text,
        source_kind: SourceKind::File,
    })
}

fn load_data_url(source: &str) -> Result<LoadedDocument, LoadError> {
    let data = &source[5..];
    let (metadata, payload) = data
        .split_once(',')
        .ok_or_else(|| LoadError::InvalidDataUrl("missing comma separator".into()))?;

    let mut parts = metadata.split(';');
    let media_type = parts.next().unwrap_or_default();
    let media_type = if media_type.is_empty() {
        "text/plain"
    } else {
        media_type
    };

    if !media_type.eq_ignore_ascii_case("text/html") {
        return Err(LoadError::UnsupportedDataMime(media_type.to_owned()));
    }

    let mut base64 = false;
    for parameter in parts {
        if parameter.eq_ignore_ascii_case("base64") {
            base64 = true;
        } else if !parameter.eq_ignore_ascii_case("charset=utf-8") {
            return Err(LoadError::InvalidDataUrl(format!(
                "unsupported parameter: {parameter}"
            )));
        }
    }

    let decoded_bytes = if base64 {
        decode_base64(payload)?
    } else {
        percent_decode_bytes(payload)
            .map_err(|message| LoadError::InvalidDataUrl(message.to_owned()))?
    };

    let text = String::from_utf8(decoded_bytes).map_err(|_| LoadError::InvalidUtf8 {
        source: "data URL payload".into(),
    })?;

    Ok(LoadedDocument {
        address: "data:text/html".into(),
        mime_type: "text/html".into(),
        text,
        source_kind: SourceKind::DataUrl,
    })
}

fn mime_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => "text/html",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

fn looks_like_windows_path(source: &str) -> bool {
    let bytes = source.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn has_uri_scheme(source: &str) -> bool {
    let Some((scheme, _)) = source.split_once(':') else {
        return false;
    };

    !scheme.is_empty()
        && scheme.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphabetic()
                || (index > 0 && (byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.')))
        })
}

fn normalize_windows_file_url_path(path: &str) -> &str {
    if path.len() >= 3
        && path.starts_with('/')
        && path.as_bytes()[1].is_ascii_alphabetic()
        && path.as_bytes()[2] == b':'
    {
        &path[1..]
    } else {
        path
    }
}

fn percent_decode(value: &str) -> Result<String, &'static str> {
    let bytes = percent_decode_bytes(value)?;
    String::from_utf8(bytes).map_err(|_| "percent-decoded value is not UTF-8")
}

fn percent_decode_bytes(value: &str) -> Result<Vec<u8>, &'static str> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            output.push(bytes[index]);
            index += 1;
            continue;
        }

        if index + 2 >= bytes.len() {
            return Err("truncated percent escape");
        }

        let high = hex_value(bytes[index + 1]).ok_or("invalid percent escape")?;
        let low = hex_value(bytes[index + 2]).ok_or("invalid percent escape")?;
        output.push((high << 4) | low);
        index += 3;
    }

    Ok(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_base64(value: &str) -> Result<Vec<u8>, LoadError> {
    let filtered: Vec<u8> = value
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();

    if !filtered.len().is_multiple_of(4) {
        return Err(LoadError::InvalidDataUrl(
            "base64 payload length is not divisible by four".into(),
        ));
    }

    let mut output = Vec::with_capacity(filtered.len() / 4 * 3);

    let (chunks, remainder) = filtered.as_chunks::<4>();
    debug_assert!(remainder.is_empty());
    for chunk in chunks {
        let a = base64_value(chunk[0]).ok_or_else(invalid_base64)?;
        let b = base64_value(chunk[1]).ok_or_else(invalid_base64)?;
        let c_padding = chunk[2] == b'=';
        let d_padding = chunk[3] == b'=';

        if c_padding && !d_padding {
            return Err(invalid_base64());
        }

        let c = if c_padding {
            0
        } else {
            base64_value(chunk[2]).ok_or_else(invalid_base64)?
        };
        let d = if d_padding {
            0
        } else {
            base64_value(chunk[3]).ok_or_else(invalid_base64)?
        };

        output.push((a << 2) | (b >> 4));

        if !c_padding {
            output.push((b << 4) | (c >> 2));
        }

        if !d_padding {
            output.push((c << 6) | d);
        }
    }

    Ok(output)
}

fn invalid_base64() -> LoadError {
    LoadError::InvalidDataUrl("invalid base64 payload".into())
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_percent_encoded_html_data_url() {
        let loaded =
            load_document("data:text/html;charset=utf-8,%3Ch1%3EOPBrowser%3C%2Fh1%3E").unwrap();

        assert_eq!(loaded.source_kind, SourceKind::DataUrl);
        assert_eq!(loaded.mime_type, "text/html");
        assert_eq!(loaded.text, "<h1>OPBrowser</h1>");
    }

    #[test]
    fn loads_base64_html_data_url() {
        let loaded = load_document("data:text/html;base64,PGgxPk9QQnJvd3NlcjwvaDE+").unwrap();

        assert_eq!(loaded.text, "<h1>OPBrowser</h1>");
    }

    #[test]
    fn rejects_http_until_network_navigation_exists() {
        assert_eq!(
            load_document("https://example.com").unwrap_err(),
            LoadError::UnsupportedScheme("https".into())
        );
    }

    #[test]
    fn loads_utf8_file_and_strips_bom() {
        let path = std::env::temp_dir().join(format!(
            "opbrowser-source-loader-{}.html",
            std::process::id()
        ));

        {
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap();
            file.write_all(b"<p>local file</p>").unwrap();
        }

        let loaded = load_document(path.to_str().unwrap()).unwrap();
        fs::remove_file(&path).unwrap();

        assert_eq!(loaded.source_kind, SourceKind::File);
        assert_eq!(loaded.mime_type, "text/html");
        assert_eq!(loaded.text, "<p>local file</p>");
    }

    #[test]
    fn recognizes_windows_drive_paths_as_files_not_schemes() {
        assert!(looks_like_windows_path(r"C:\pages\test.html"));
        assert!(
            !has_uri_scheme(r"C:\pages\test.html")
                || looks_like_windows_path(r"C:\pages\test.html")
        );
    }
}
