use crate::{LoadError, LoadedDocument, SourceKind};

pub(crate) const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;

pub(crate) fn validate_url(source: &str) -> Result<(), LoadError> {
    HttpUrl::parse(source).map(|_| ())
}

/// Initial HTTP URL subset: ASCII DNS/IPv4 or bracketed IPv6, optional port,
/// UTF-8 path/query encoded as bytes. Credentials and malformed escapes are rejected.
#[derive(Debug, PartialEq, Eq)]
struct HttpUrl {
    secure: bool,
    host: String,
    port: u16,
    target: String,
}

impl HttpUrl {
    fn parse(source: &str) -> Result<Self, LoadError> {
        let invalid = || LoadError::InvalidHttpUrl(source.to_owned());
        if source
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
        {
            return Err(invalid());
        }
        let (scheme, rest) = source.split_once("://").ok_or_else(invalid)?;
        let secure = match scheme.to_ascii_lowercase().as_str() {
            "https" => true,
            "http" => false,
            _ => return Err(invalid()),
        };
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..end];
        let (host, port_text) = if let Some(ipv6) = authority.strip_prefix('[') {
            let (host, suffix) = ipv6.split_once(']').ok_or_else(invalid)?;
            host.parse::<std::net::Ipv6Addr>().map_err(|_| invalid())?;
            let port = if suffix.is_empty() {
                None
            } else {
                Some(suffix.strip_prefix(':').ok_or_else(invalid)?)
            };
            (host, port)
        } else {
            let (host, port) = authority
                .split_once(':')
                .map_or((authority, None), |(h, p)| (h, Some(p)));
            if host.is_empty()
                || !host
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.'))
            {
                return Err(invalid());
            }
            (host, port)
        };
        let port = match port_text {
            Some(value) if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                value.parse::<u16>().map_err(|_| invalid())?
            }
            Some(_) => return Err(invalid()),
            None => {
                if secure {
                    443
                } else {
                    80
                }
            }
        };
        if port == 0 {
            return Err(invalid());
        }
        let path = rest[end..].split('#').next().unwrap_or_default();
        let mut target = String::new();
        if !path.starts_with('/') {
            target.push('/');
        }
        let bytes = path.as_bytes();
        for (index, byte) in bytes.iter().copied().enumerate() {
            if byte == b'%'
                && (index + 2 >= bytes.len()
                    || !bytes[index + 1].is_ascii_hexdigit()
                    || !bytes[index + 2].is_ascii_hexdigit())
            {
                return Err(invalid());
            }
            if byte.is_ascii() {
                target.push(char::from(byte));
            } else {
                use std::fmt::Write;
                write!(target, "%{byte:02X}").expect("writing to String");
            }
        }
        Ok(Self {
            secure,
            host: host.to_ascii_lowercase(),
            port,
            target,
        })
    }
}

fn decode_document(
    address: String,
    content_type: &str,
    bytes: Vec<u8>,
) -> Result<LoadedDocument, LoadError> {
    let mut parts = content_type.split(';');
    let mime = parts
        .next()
        .unwrap_or("text/html")
        .trim()
        .to_ascii_lowercase();
    if mime != "text/html" {
        return Err(LoadError::UnsupportedContentType(mime));
    }
    for parameter in parts {
        if let Some((name, value)) = parameter.trim().split_once('=')
            && name.trim().eq_ignore_ascii_case("charset")
        {
            let charset = value.trim().trim_matches(['\'', '"']);
            if !charset.eq_ignore_ascii_case("utf-8") && !charset.eq_ignore_ascii_case("us-ascii") {
                return Err(LoadError::UnsupportedCharset(charset.to_owned()));
            }
        }
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    let text = String::from_utf8(bytes.to_vec()).map_err(|_| LoadError::InvalidUtf8 {
        source: address.clone(),
    })?;
    Ok(LoadedDocument {
        address,
        mime_type: mime,
        text,
        source_kind: SourceKind::Http,
    })
}

pub(crate) fn load(source: &str) -> Result<LoadedDocument, LoadError> {
    let url = HttpUrl::parse(source)?;
    #[cfg(windows)]
    {
        windows::load(url)
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err(LoadError::Network("HTTP transport requires Windows".into()))
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Networking::WinHttp::*;

    struct Handle(*mut c_void);
    impl Handle {
        fn checked(value: *mut c_void, operation: &str) -> Result<Self, LoadError> {
            if value.is_null() {
                Err(error(operation))
            } else {
                Ok(Self(value))
            }
        }
        fn option(&self, option: u32, value: u32) -> Result<(), LoadError> {
            check(
                unsafe { WinHttpSetOption(self.0, option, (&value as *const u32).cast(), 4) },
                "set request option",
            )
        }
    }
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
    fn error(operation: &str) -> LoadError {
        LoadError::Network(format!("{operation}: {}", std::io::Error::last_os_error()))
    }
    fn check(result: i32, operation: &str) -> Result<(), LoadError> {
        if result == 0 {
            Err(error(operation))
        } else {
            Ok(())
        }
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    pub(super) fn load(url: HttpUrl) -> Result<LoadedDocument, LoadError> {
        // WinHTTP handles only transport/TLS/proxy/framing, never HTML or rendering.
        let agent = wide("OPBrowser/0.1");
        let session = Handle::checked(
            unsafe {
                WinHttpOpen(
                    agent.as_ptr(),
                    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                    null(),
                    null(),
                    0,
                )
            },
            "open session",
        )?;
        check(
            unsafe { WinHttpSetTimeouts(session.0, 10_000, 10_000, 10_000, 10_000) },
            "set timeouts",
        )?;
        let host = wide(&url.host);
        let connection = Handle::checked(
            unsafe { WinHttpConnect(session.0, host.as_ptr(), url.port, 0) },
            "connect",
        )?;
        let verb = wide("GET");
        let target = wide(&url.target);
        let accept = wide("text/html");
        let accept_types = [accept.as_ptr(), null()];
        let request = Handle::checked(
            unsafe {
                WinHttpOpenRequest(
                    connection.0,
                    verb.as_ptr(),
                    target.as_ptr(),
                    null(),
                    null(),
                    accept_types.as_ptr(),
                    if url.secure { WINHTTP_FLAG_SECURE } else { 0 },
                )
            },
            "open request",
        )?;
        request.option(
            WINHTTP_OPTION_DISABLE_FEATURE,
            WINHTTP_DISABLE_COOKIES | WINHTTP_DISABLE_AUTHENTICATION,
        )?;
        request.option(WINHTTP_OPTION_MAX_HTTP_AUTOMATIC_REDIRECTS, 5)?;
        request.option(
            WINHTTP_OPTION_REDIRECT_POLICY,
            WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP,
        )?;
        request.option(
            WINHTTP_OPTION_DECOMPRESSION,
            WINHTTP_DECOMPRESSION_FLAG_GZIP | WINHTTP_DECOMPRESSION_FLAG_DEFLATE,
        )?;
        let started = Instant::now();
        check(
            unsafe { WinHttpSendRequest(request.0, null(), 0, null(), 0, 0, 0) },
            "send GET",
        )?;
        check(
            unsafe { WinHttpReceiveResponse(request.0, null_mut()) },
            "receive response",
        )?;
        let mut status: u32 = 0;
        let mut length = 4;
        check(
            unsafe {
                WinHttpQueryHeaders(
                    request.0,
                    WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                    null(),
                    (&mut status as *mut u32).cast(),
                    &mut length,
                    null_mut(),
                )
            },
            "read HTTP status",
        )?;
        if !(200..300).contains(&status) {
            return Err(LoadError::HttpStatus(status));
        }
        let content_type = query_string(&request, Some(WINHTTP_QUERY_CONTENT_TYPE))?;
        let address = query_string(&request, None)?;
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 16 * 1024];
        loop {
            if started.elapsed() > Duration::from_secs(30) {
                return Err(LoadError::Network("document read deadline exceeded".into()));
            }
            let mut read = 0;
            check(
                unsafe {
                    WinHttpReadData(
                        request.0,
                        buffer.as_mut_ptr().cast(),
                        buffer.len() as u32,
                        &mut read,
                    )
                },
                "read body",
            )?;
            if read == 0 {
                break;
            }
            if bytes.len() + read as usize > MAX_DOCUMENT_BYTES {
                return Err(LoadError::DocumentTooLarge);
            }
            bytes.extend_from_slice(&buffer[..read as usize]);
        }
        decode_document(address, &content_type, bytes)
    }

    fn query_string(request: &Handle, header: Option<u32>) -> Result<String, LoadError> {
        let mut length = 0;
        let query = |buffer: *mut c_void, length: &mut u32| unsafe {
            match header {
                Some(header) => {
                    WinHttpQueryHeaders(request.0, header, null(), buffer, length, null_mut())
                }
                None => WinHttpQueryOption(request.0, WINHTTP_OPTION_URL, buffer, length),
            }
        };
        query(null_mut(), &mut length);
        if length == 0 {
            // Missing Content-Type is treated as HTML at this milestone.
            if header.is_some() {
                return Ok("text/html".into());
            }
            return Err(error("query response URL"));
        }
        let mut buffer = vec![0u16; length as usize / 2 + 1];
        check(
            query(buffer.as_mut_ptr().cast(), &mut length),
            "query response metadata",
        )?;
        let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        String::from_utf16(&buffer[..end])
            .map_err(|_| LoadError::Network("invalid response metadata".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn serve(responses: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::{Duration, Instant};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for response in responses {
                let started = Instant::now();
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                started.elapsed() < Duration::from_secs(5),
                                "test server timed out"
                            );
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("{error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut buffer = [0u8; 1024];
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0 && request.len() < 32 * 1024);
                    request.extend_from_slice(&buffer[..count]);
                }
                requests.push(String::from_utf8(request).unwrap());
                // A bounded loader may close early on an oversized document.
                let _ = stream.write_all(&response);
            }
            requests
        });
        (address, thread)
    }

    #[cfg(windows)]
    fn response(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
        let mut bytes = format!(
            "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
            body.len()
        )
        .into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }

    #[test]
    #[cfg(windows)]
    fn fetches_http_utf8_and_sends_get_without_fragment() {
        let (address, server) = serve(vec![response(
            "200 OK",
            "Content-Type: text/html; charset=UTF-8\r\n",
            "<h1>Привет</h1>".as_bytes(),
        )]);
        let loaded = load(&format!("{address}/page?x=1#fragment")).unwrap();
        assert_eq!(loaded.text, "<h1>Привет</h1>");
        assert_eq!(loaded.source_kind, SourceKind::Http);
        assert_eq!(loaded.address, format!("{address}/page?x=1"));
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("GET /page?x=1 HTTP/1.1\r\n"));
        assert!(requests[0].contains("OPBrowser/0.1"));
    }

    #[test]
    #[cfg(windows)]
    fn follows_relative_redirect_and_does_not_send_cookies() {
        let (address, server) = serve(vec![
            response(
                "302 Found",
                "Location: /final\r\nSet-Cookie: tracking=1\r\n",
                b"",
            ),
            response(
                "200 OK",
                "Content-Type: text/html\r\n",
                b"<p>Redirected</p>",
            ),
        ]);
        let loaded = load(&format!("{address}/start")).unwrap();
        assert_eq!(loaded.address, format!("{address}/final"));
        assert_eq!(loaded.text, "<p>Redirected</p>");
        let requests = server.join().unwrap();
        assert!(requests[1].starts_with("GET /final "));
        assert!(!requests[1].to_ascii_lowercase().contains("\r\ncookie:"));
    }

    #[test]
    #[cfg(windows)]
    fn decodes_gzip_and_chunked_responses() {
        let gzip = [
            0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0x0a, 0xb3, 0xc9, 0x30, 0xb4, 0x4b, 0xce, 0xcf, 0x2d,
            0x28, 0x4a, 0x2d, 0x2e, 0x4e, 0x4d, 0xb1, 0xd1, 0xcf, 0x30, 0xb4, 3, 0, 0x52, 7, 0x2e,
            0x99, 0x13, 0, 0, 0,
        ];
        let (address, server) = serve(vec![response(
            "200 OK",
            "Content-Type: text/html\r\nContent-Encoding: gzip\r\n",
            &gzip,
        )]);
        assert_eq!(load(&address).unwrap().text, "<h1>compressed</h1>");
        server.join().unwrap();
        let (address, server) = serve(vec![b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n8\r\n<p>chunk\r\n5\r\n</p>!\r\n0\r\n\r\n".to_vec()]);
        assert_eq!(load(&address).unwrap().text, "<p>chunk</p>!");
        server.join().unwrap();
    }

    #[test]
    #[cfg(windows)]
    fn rejects_http_errors_binary_content_and_oversized_bodies() {
        for (status, headers, body, expected) in [
            (
                "404 Not Found",
                "Content-Type: text/html\r\n",
                vec![],
                LoadError::HttpStatus(404),
            ),
            (
                "200 OK",
                "Content-Type: image/png\r\n",
                vec![0],
                LoadError::UnsupportedContentType("image/png".into()),
            ),
            (
                "200 OK",
                "Content-Type: text/html\r\n",
                vec![b'a'; MAX_DOCUMENT_BYTES + 1],
                LoadError::DocumentTooLarge,
            ),
        ] {
            let (address, server) = serve(vec![response(status, headers, &body)]);
            assert_eq!(load(&address).unwrap_err(), expected);
            server.join().unwrap();
        }
    }

    #[test]
    #[cfg(windows)]
    fn bounds_redirect_loops() {
        let redirect = response("302 Found", "Location: /loop\r\n", b"");
        let (address, server) = serve(vec![redirect; 6]);
        assert!(matches!(load(&address), Err(LoadError::Network(_))));
        assert_eq!(server.join().unwrap().len(), 6);
    }

    #[test]
    fn parses_ports_queries_fragments_and_utf8_paths() {
        assert_eq!(
            HttpUrl::parse("HTTP://Example.COM:8080?x=1#section").unwrap(),
            HttpUrl {
                secure: false,
                host: "example.com".into(),
                port: 8080,
                target: "/?x=1".into()
            }
        );
        assert_eq!(
            HttpUrl::parse("https://[::1]/тест").unwrap().target,
            "/%D1%82%D0%B5%D1%81%D1%82"
        );
        assert_eq!(HttpUrl::parse("https://example.com").unwrap().port, 443);
    }

    #[test]
    fn rejects_ambiguous_or_malformed_urls() {
        for url in [
            "http:example.com",
            "http:///a",
            "http://a:0",
            "http://a:65536",
            "http://a:-1",
            "http://user:pass@a/",
            "http://a/%xy",
            "http://a/%",
            "http://a/\r\nx",
            "http://a\\b",
            "http://[broken]/",
            "http://тест.рф/",
        ] {
            assert!(HttpUrl::parse(url).is_err(), "{url}");
        }
    }

    #[test]
    fn validates_html_mime_charset_and_utf8() {
        let loaded = decode_document(
            "http://a/".into(),
            "Text/HTML; charset=\"UTF-8\"",
            b"\xef\xbb\xbf<p>Hello</p>".to_vec(),
        )
        .unwrap();
        assert_eq!(loaded.text, "<p>Hello</p>");
        assert!(matches!(
            decode_document("a".into(), "image/png", vec![]),
            Err(LoadError::UnsupportedContentType(_))
        ));
        assert!(matches!(
            decode_document("a".into(), "text/html; charset=windows-1251", vec![]),
            Err(LoadError::UnsupportedCharset(_))
        ));
        assert!(matches!(
            decode_document("a".into(), "text/html", vec![255]),
            Err(LoadError::InvalidUtf8 { .. })
        ));
    }
}
