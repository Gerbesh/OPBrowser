#![cfg(windows)]

use op_engine::Engine;
use op_paint::{PaintCommand, RasterImage};
use std::sync::Arc;

fn images(commands: &[PaintCommand]) -> Vec<&Arc<RasterImage>> {
    commands
        .iter()
        .filter_map(|command| {
            if let PaintCommand::Image { image, .. } = command {
                Some(image)
            } else {
                None
            }
        })
        .collect()
}

fn painted_text(commands: &[PaintCommand]) -> String {
    commands
        .iter()
        .filter_map(|command| {
            if let PaintCommand::Text { text, .. } = command {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn local_images_and_fallback_text_reach_paint_and_image_link_navigation() {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/images/index.html");
    let mut engine = Engine::new();
    let page = engine.navigate(source.to_str().unwrap(), 800, 600).unwrap();
    assert_eq!(images(&page.display_list.commands).len(), 4);
    assert!(painted_text(&page.display_list.commands).contains("Показан текст alt"));
    let (width, height, href) = page
        .display_list
        .commands
        .iter()
        .find_map(|command| {
            if let PaintCommand::Image {
                width,
                height,
                href,
                ..
            } = command
            {
                Some((*width, *height, href.as_deref()))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!((width, height), (320, 160));
    let next = engine.follow_link(href.unwrap(), 800, 600).unwrap();
    assert!(painted_text(&next.display_list.commands).contains("Link destination"));
    assert_eq!(engine.navigation().entries().len(), 2);
}

#[test]
fn data_images_share_pixels_and_bound_node_count_without_failing_document_history() {
    let payload = include_bytes!("../../../examples/images/colors.png")
        .iter()
        .map(|byte| format!("%{byte:02X}"))
        .collect::<String>();
    let src = format!("data:image/png,{payload}");
    let html = format!(
        "<body>{}<p>tail</p><template><img src='https://invalid.example/x'></template></body>",
        format!("<img src='{src}' width=4 alt='limited'>").repeat(34)
    );
    // Outer data payload may contain '%' sequences, so escape the entire HTML.
    let html = html
        .bytes()
        .map(|byte| format!("%{byte:02X}"))
        .collect::<String>();
    let mut engine = Engine::new();
    let page = engine
        .navigate(&format!("data:text/html,{html}"), 800, 600)
        .unwrap();
    let rasters = images(&page.display_list.commands);
    assert_eq!(rasters.len(), 32);
    assert!(rasters.iter().all(|image| Arc::ptr_eq(rasters[0], image)));
    assert!(painted_text(&page.display_list.commands).contains("limited"));
    assert!(painted_text(&page.display_list.commands).contains("tail"));
    assert_eq!(engine.navigation().entries().len(), 1);
}

#[test]
fn bounds_unique_resource_attempts_and_total_decoded_pixel_storage() {
    for (bytes, expected) in [
        (
            include_bytes!("../../../examples/images/colors.png").as_slice(),
            8,
        ),
        (
            include_bytes!("../../../examples/images/pixel-budget.png").as_slice(),
            2,
        ),
    ] {
        let payload = bytes
            .iter()
            .map(|b| format!("%{b:02X}"))
            .collect::<String>();
        // PNG tolerates trailing bytes; each source is unique but keeps the same
        // dimensions. This exercises budgets without making network requests.
        let html = (0..10)
            .map(|n| {
                format!(
                    "<img src='data:image/png,{payload}{}' alt='limited'>",
                    "%00".repeat(n)
                )
            })
            .collect::<String>();
        let html = html
            .bytes()
            .map(|b| format!("%{b:02X}"))
            .collect::<String>();
        let page = Engine::new()
            .render_source(&format!("data:text/html,{html}"), 800, 600)
            .unwrap();
        assert_eq!(images(&page.display_list.commands).len(), expected);
        assert!(painted_text(&page.display_list.commands).contains("limited"));
        assert!(
            images(&page.display_list.commands)
                .iter()
                .map(|i| i.pixels().len())
                .sum::<usize>()
                <= 32 * 1024 * 1024
        );
    }
}

#[test]
fn http_images_use_redirected_base_cache_and_nonfatal_failure_fallback() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let html = b"<body><img src='../colors.png?a=1&amp;b=2'><img src='../colors.png?a=1&amp;b=2'><img src='bad.png' alt='decode failure'><img src='missing.png' alt='HTTP failure'><img src='other.png' alt='MIME failure'><img src='huge.png' alt='size failure'><img src='file:///C:/private.png' alt='blocked file'><img src='missing.png' alt='cached failure'><p>after images</p></body>";
    let server = std::thread::spawn(move || {
        let responses: Vec<(&str, &str, &str, &[u8])> = vec![
            ("/start", "302 Found", "Location: /nested/page\r\n", b""),
            (
                "/nested/page",
                "200 OK",
                "Content-Type: text/html\r\n",
                html,
            ),
            (
                "/colors.png?a=1&b=2",
                "200 OK",
                "Content-Type: image/png\r\n",
                include_bytes!("../../../examples/images/colors.png"),
            ),
            (
                "/nested/bad.png",
                "200 OK",
                "Content-Type: image/png\r\n",
                b"broken",
            ),
            ("/nested/missing.png", "404 Missing", "", b""),
            (
                "/nested/other.png",
                "200 OK",
                "Content-Type: text/html\r\n",
                b"<p>not an image</p>",
            ),
            (
                "/nested/huge.png",
                "200 OK",
                "Content-Type: image/png\r\n",
                include_bytes!("../../../examples/images/oversized.png"),
            ),
        ];
        for (path, status, headers, bytes) in responses {
            let started = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(started.elapsed() < Duration::from_secs(5));
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
                assert!(request.len() < 16 * 1024);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(
                request.starts_with(&format!("GET {path} HTTP/1.1")),
                "{request}"
            );
            assert!(!request.contains("Cookie:"));
            write!(
                stream,
                "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(bytes).unwrap();
        }
    });
    let mut engine = Engine::new();
    let page = engine
        .navigate(&format!("{address}/start"), 800, 600)
        .unwrap();
    server.join().unwrap();
    let rasters = images(&page.display_list.commands);
    assert_eq!(rasters.len(), 2);
    assert!(Arc::ptr_eq(rasters[0], rasters[1]));
    for fallback in [
        "decode failure",
        "HTTP failure",
        "MIME failure",
        "size failure",
        "blocked file",
        "cached failure",
        "after images",
    ] {
        assert!(
            painted_text(&page.display_list.commands).contains(fallback),
            "{fallback}"
        );
    }
    assert_eq!(engine.navigation().entries().len(), 1);
    assert_eq!(
        engine.navigation().current().unwrap().address,
        format!("{address}/nested/page")
    );
}
