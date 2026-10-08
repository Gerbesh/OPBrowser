//! Owned document decoding subset. Tables follow WHATWG Encoding indexes
//! (2024-09-18); selection follows BOM -> supported transport -> early HTML meta.
use crate::LoadError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1251,
    Windows1252,
}

fn from_label(label: &str) -> Option<Encoding> {
    match label
        .trim_matches(|c: char| c.is_ascii_whitespace())
        .to_ascii_lowercase()
        .as_str()
    {
        "utf-8" | "utf8" | "unicode-1-1-utf-8" | "unicode11utf8" | "unicode20utf8"
        | "x-unicode20utf8" => Some(Encoding::Utf8),
        "windows-1251" | "cp1251" | "x-cp1251" => Some(Encoding::Windows1251),
        "windows-1252" | "cp1252" | "x-cp1252" | "ascii" | "us-ascii" | "latin1" | "iso-8859-1"
        | "iso8859-1" | "iso88591" | "iso_8859-1" | "iso_8859-1:1987" | "l1" | "ansi_x3.4-1968"
        | "cp819" | "csisolatin1" | "ibm819" | "iso-ir-100" => Some(Encoding::Windows1252),
        "utf-16" | "utf-16le" | "unicode" | "unicodefeff" | "ucs-2" | "csunicode"
        | "iso-10646-ucs-2" => Some(Encoding::Utf16Le),
        "utf-16be" | "unicodefffe" => Some(Encoding::Utf16Be),
        _ => None,
    }
}

pub(crate) fn decode_html(
    bytes: &[u8],
    label: Option<&str>,
    source: &str,
) -> Result<String, LoadError> {
    if let Some((encoding, bytes)) = select_bom(bytes) {
        return decode_selected(bytes, encoding, source);
    }
    let encoding = if let Some(label) = label {
        from_label(label).ok_or_else(|| LoadError::UnsupportedCharset(label.into()))?
    } else {
        sniff_meta(bytes).unwrap_or(Encoding::Utf8)
    };
    decode_selected(bytes, encoding, source)
}

pub(crate) fn decode_script(
    bytes: &[u8],
    label: Option<&str>,
    source: &str,
) -> Result<String, LoadError> {
    let (encoding, bytes) = if let Some(bom) = select_bom(bytes) {
        bom
    } else if let Some(label) = label {
        (
            from_label(label).ok_or_else(|| LoadError::UnsupportedCharset(label.into()))?,
            bytes,
        )
    } else {
        (Encoding::Utf8, bytes)
    };
    decode_selected(bytes, encoding, source)
}

pub(crate) fn decode_css(
    bytes: &[u8],
    label: Option<&str>,
    source: &str,
) -> Result<String, LoadError> {
    let bom = select_bom(bytes);
    let (encoding, bytes) = if let Some(selected) = bom {
        selected
    } else {
        let encoding = if let Some(label) = label {
            from_label(label).ok_or_else(|| LoadError::UnsupportedCharset(label.into()))?
        } else {
            sniff_css_charset(bytes).unwrap_or(Encoding::Utf8)
        };
        (encoding, bytes)
    };
    decode_selected(bytes, encoding, source).map(strip_css_charset_rule)
}

fn strip_css_charset_rule(text: String) -> String {
    let Some(rest) = text.strip_prefix("@charset \"") else {
        return text;
    };
    let Some(end) = rest.find("\";") else {
        return text;
    };
    rest[end + 2..].to_owned()
}

fn select_bom(bytes: &[u8]) -> Option<(Encoding, &[u8])> {
    if let Some(bytes) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        Some((Encoding::Utf8, bytes))
    } else if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        Some((Encoding::Utf16Le, bytes))
    } else {
        bytes
            .strip_prefix(&[0xfe, 0xff])
            .map(|bytes| (Encoding::Utf16Be, bytes))
    }
}

fn sniff_css_charset(bytes: &[u8]) -> Option<Encoding> {
    let prefix = b"@charset \"";
    let rest = bytes.strip_prefix(prefix)?;
    let end = rest.iter().position(|byte| *byte == b'"')?;
    if rest.get(end + 1) != Some(&b';') {
        return None;
    }
    std::str::from_utf8(&rest[..end]).ok().and_then(from_label)
}

fn decode_selected(bytes: &[u8], encoding: Encoding, source: &str) -> Result<String, LoadError> {
    match encoding {
        Encoding::Utf8 => String::from_utf8(bytes.to_vec()).map_err(|_| LoadError::InvalidUtf8 {
            source: source.into(),
        }),
        Encoding::Utf16Le | Encoding::Utf16Be => {
            let invalid = || LoadError::InvalidEncoding {
                source: source.into(),
                encoding: if encoding == Encoding::Utf16Le {
                    "UTF-16LE"
                } else {
                    "UTF-16BE"
                }
                .into(),
            };
            let (pairs, remainder) = bytes.as_chunks::<2>();
            if !remainder.is_empty() {
                return Err(invalid());
            }
            let words: Vec<u16> = pairs
                .iter()
                .map(|pair| {
                    if encoding == Encoding::Utf16Le {
                        u16::from_le_bytes(*pair)
                    } else {
                        u16::from_be_bytes(*pair)
                    }
                })
                .collect();
            String::from_utf16(&words).map_err(|_| invalid())
        }
        Encoding::Windows1251 | Encoding::Windows1252 => Ok(bytes
            .iter()
            .map(|byte| {
                let point = match (encoding, *byte) {
                    (_, 0..=0x7f) => u32::from(*byte),
                    (Encoding::Windows1251, 0x80..=0xbf) => {
                        u32::from(WINDOWS_1251_LOW[usize::from(*byte - 0x80)])
                    }
                    (Encoding::Windows1251, _) => 0x410 + u32::from(*byte - 0xc0),
                    (Encoding::Windows1252, 0x80..=0x9f) => {
                        u32::from(WINDOWS_1252_C1[usize::from(*byte - 0x80)])
                    }
                    _ => u32::from(*byte),
                };
                char::from_u32(point).expect("single-byte table contains Unicode scalar values")
            })
            .collect()),
    }
}

// https://encoding.spec.whatwg.org/index-windows-1251.txt
const WINDOWS_1251_LOW: [u16; 64] = [
    0x0402, 0x0403, 0x201a, 0x0453, 0x201e, 0x2026, 0x2020, 0x2021, 0x20ac, 0x2030, 0x0409, 0x2039,
    0x040a, 0x040c, 0x040b, 0x040f, 0x0452, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
    0x0098, 0x2122, 0x0459, 0x203a, 0x045a, 0x045c, 0x045b, 0x045f, 0x00a0, 0x040e, 0x045e, 0x0408,
    0x00a4, 0x0490, 0x00a6, 0x00a7, 0x0401, 0x00a9, 0x0404, 0x00ab, 0x00ac, 0x00ad, 0x00ae, 0x0407,
    0x00b0, 0x00b1, 0x0406, 0x0456, 0x0491, 0x00b5, 0x00b6, 0x00b7, 0x0451, 0x2116, 0x0454, 0x00bb,
    0x0458, 0x0405, 0x0455, 0x0457,
];
// https://encoding.spec.whatwg.org/index-windows-1252.txt
const WINDOWS_1252_C1: [u16; 32] = [
    0x20ac, 0x0081, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008d, 0x017d, 0x008f, 0x0090, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
    0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x009d, 0x017e, 0x0178,
];

/// Parse charset parameters while ignoring separators inside quoted values.
pub(crate) fn charset_parameter(value: &str) -> Option<String> {
    let mut start = 0;
    let mut quote = None;
    for (index, character) in value
        .char_indices()
        .chain(std::iter::once((value.len(), ';')))
    {
        if let Some(current) = quote {
            if character == current {
                quote = None;
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if character == ';' {
            if let Some((name, label)) = value[start..index].trim().split_once('=')
                && name.trim().eq_ignore_ascii_case("charset")
            {
                return Some(label.trim().trim_matches(['\'', '"']).to_owned());
            }
            start = index + 1;
        }
    }
    None
}

fn sniff_meta(bytes: &[u8]) -> Option<Encoding> {
    let bytes = &bytes[..bytes.len().min(1024)];
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position..].starts_with(b"<!--") {
            let offset = bytes[position + 4..]
                .windows(3)
                .position(|window| window == b"-->")?;
            position += 4 + offset + 3;
            continue;
        }
        if bytes[position] != b'<' {
            position += 1;
            continue;
        }
        position += 1;
        let start = position;
        while bytes
            .get(position)
            .is_some_and(|b| b.is_ascii_alphanumeric())
        {
            position += 1;
        }
        let is_meta = bytes[start..position].eq_ignore_ascii_case(b"meta");
        let mut attributes = Vec::new();
        let mut finished = false;
        // Consume the entire tag, including quoted attributes on non-meta tags.
        while let Some(byte) = bytes.get(position) {
            if *byte == b'>' {
                position += 1;
                finished = true;
                break;
            }
            if byte.is_ascii_whitespace() || *byte == b'/' {
                position += 1;
                continue;
            }
            let name_start = position;
            while bytes
                .get(position)
                .is_some_and(|b| !b.is_ascii_whitespace() && !matches!(b, b'=' | b'/' | b'>'))
            {
                position += 1;
            }
            let name = String::from_utf8_lossy(&bytes[name_start..position]).to_ascii_lowercase();
            while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
                position += 1;
            }
            let mut value = String::new();
            if bytes.get(position) == Some(&b'=') {
                position += 1;
                while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
                    position += 1;
                }
                let delimiter = bytes
                    .get(position)
                    .copied()
                    .filter(|b| matches!(b, b'\'' | b'"'));
                if delimiter.is_some() {
                    position += 1;
                }
                let value_start = position;
                while let Some(byte) = bytes.get(position) {
                    if delimiter.map_or_else(
                        || byte.is_ascii_whitespace() || *byte == b'>',
                        |quote| *byte == quote,
                    ) {
                        break;
                    }
                    position += 1;
                }
                // Abort if the prescan ends part-way through an attribute value.
                bytes.get(position)?;
                value = String::from_utf8_lossy(&bytes[value_start..position]).into_owned();
                if delimiter.is_some() {
                    position += 1;
                }
            }
            if !attributes.iter().any(|(existing, _)| existing == &name) {
                attributes.push((name, value));
            }
        }
        if !finished {
            return None;
        }
        if !is_meta {
            continue;
        }
        let attribute = |name| {
            attributes
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };
        let label = attribute("charset").map(str::to_owned).or_else(|| {
            if attribute("http-equiv").is_some_and(|v| v.eq_ignore_ascii_case("content-type")) {
                attribute("content").and_then(charset_parameter)
            } else {
                None
            }
        });
        if let Some(encoding) = label.as_deref().and_then(from_label) {
            return Some(match encoding {
                Encoding::Utf16Le | Encoding::Utf16Be => Encoding::Utf8,
                other => other,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_cyrillic_and_latin_code_pages_and_aliases() {
        for label in ["windows-1251", "CP1251", " x-cp1251 "] {
            assert_eq!(
                decode_html(
                    &[0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2, 0x20, 0xa8, 0xb8, 0xb9],
                    Some(label),
                    "test"
                )
                .unwrap(),
                "Привет Ёё№"
            );
        }
        for label in ["windows-1252", "iso-8859-1", "us-ascii"] {
            assert_eq!(
                decode_html(&[0x80, 0x20, 0x93, 0xe9, 0x94], Some(label), "test").unwrap(),
                "€ “é”"
            );
        }
    }

    #[test]
    fn decodes_css_transport_or_charset_and_strips_charset_rule() {
        let mut css = b"@charset \"windows-1251\";p{content:\"".to_vec();
        css.extend_from_slice(&[0xcf, 0xf0]);
        css.extend_from_slice(b"\"}");
        assert_eq!(
            decode_css(&css, None, "test.css").unwrap(),
            "p{content:\"Пр\"}"
        );
        assert_eq!(
            decode_css(
                b"@charset \"windows-1251\";p{color:red}",
                Some("utf-8"),
                "test.css"
            )
            .unwrap(),
            "p{color:red}"
        );
        assert_eq!(
            decode_css(
                b"\xef\xbb\xbf@charset \"unsupported\";p{color:blue}",
                Some("unsupported"),
                "test.css"
            )
            .unwrap(),
            "p{color:blue}"
        );
    }

    #[test]
    fn bom_overrides_headers_and_unicode_is_strict() {
        assert_eq!(
            decode_html(b"\xef\xbb\xbfUTF-8", Some("windows-1251"), "test").unwrap(),
            "UTF-8"
        );
        assert_eq!(
            decode_html(&[0xff, 0xfe, 0x1f, 4, 0x40, 4], Some("unsupported"), "test").unwrap(),
            "Пр"
        );
        assert_eq!(
            decode_html(&[0xfe, 0xff, 4, 0x1f, 4, 0x40], None, "test").unwrap(),
            "Пр"
        );
        assert!(matches!(
            decode_html(&[0xff, 0xfe, 0], None, "test"),
            Err(LoadError::InvalidEncoding { .. })
        ));
        assert!(matches!(
            decode_html(&[0xff], Some("utf-8"), "test"),
            Err(LoadError::InvalidUtf8 { .. })
        ));
    }

    #[test]
    fn sniffs_meta_with_pragma_comments_quotes_and_boundaries() {
        assert_eq!(
            sniff_meta(b"<!-- <meta charset=utf-8> --><META CHARSET='cp1251'>"),
            Some(Encoding::Windows1251)
        );
        assert_eq!(sniff_meta(b"<div title='<meta charset=utf-8>'><meta content='text/html; charset=windows-1251' http-equiv=Content-Type>"), Some(Encoding::Windows1251));
        assert_eq!(
            sniff_meta(b"<meta content='text/html; charset=windows-1251'>"),
            None
        );
        assert_eq!(
            sniff_meta(b"<meta charset=unknown><meta charset=windows-1252>"),
            Some(Encoding::Windows1252)
        );
        assert_eq!(sniff_meta(b"<meta charset=utf-16>"), Some(Encoding::Utf8));
        let mut late = vec![b' '; 1024];
        late.extend_from_slice(b"<meta charset=windows-1251>");
        assert_eq!(sniff_meta(&late), None);
        assert_eq!(sniff_meta(b"<meta charset='cp1251'"), None);
        assert_eq!(
            charset_parameter("text/html; other='x;charset=bad'; charset=\"cp1251\""),
            Some("cp1251".into())
        );
        assert_eq!(
            decode_html(
                b"<meta charset=windows-1251><p>\xcf\xf0\xe8\xe2\xe5\xf2</p>",
                None,
                "test"
            )
            .unwrap(),
            "<meta charset=windows-1251><p>Привет</p>"
        );
        assert!(decode_html(b"<meta charset=windows-1251>\xff", Some("utf-8"), "test").is_err());
    }
}
