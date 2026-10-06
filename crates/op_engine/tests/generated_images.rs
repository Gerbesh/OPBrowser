#![cfg(windows)]

use op_engine::Engine;
use op_paint::{PaintCommand, RasterImage};
use std::sync::Arc;

fn percent(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("%{byte:02X}")).collect()
}

fn rasters(commands: &[PaintCommand]) -> Vec<&Arc<RasterImage>> {
    commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Image { image, .. } => Some(image),
            _ => None,
        })
        .collect()
}

#[test]
fn sole_generated_image_css_geometry_reaches_paint_and_native_link_identity() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/css/generated-images.html");
    let mut engine = Engine::new();
    let original = engine
        .navigate(source.to_str().unwrap(), 800, 600)
        .unwrap()
        .display_list;
    assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::Image { width:40, height:32, href:Some(href), .. } if href == "../navigation/destination.html")));
    assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::FillRect { width:52, height:44, color, .. } if *color == op_paint::Color { r:238, g:242, b:255 })));
    engine.reflow(240, 600).unwrap();
    assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
}

#[test]
fn decorated_dom_image_boxes_reach_paint_links_and_retained_reflow() {
    let src = format!(
        "data:image/png,{}",
        percent(include_bytes!("../../../examples/images/colors.png"))
    );
    let html = format!(
        "<style>img {{ padding:4px; border:2px solid red; background:green; box-sizing:border-box; width:42px; height:32px }}</style><p><a href='next.html'><img src='{src}' width=300 height=200></a>Tail</p>"
    );
    let mut engine = Engine::new();
    let original = engine
        .navigate(
            &format!("data:text/html,{}", percent(html.as_bytes())),
            800,
            600,
        )
        .unwrap()
        .display_list;
    assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::Image { width:30, height:20, href:Some(href), .. } if href == "next.html")));
    assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::FillRect { width:42, height:32, color, .. } if *color == op_paint::Color { r:0, g:128, b:0 })));
    assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::FillRect { width:42, height:2, color, .. } if *color == op_paint::Color { r:255, g:0, b:0 })));
    engine.reflow(240, 600).unwrap();
    assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
}

#[test]
fn generated_urls_use_consumer_stylesheet_bases_and_retained_shared_pixels() {
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            for file in [
                "index.html",
                "defs/vars.css",
                "assets/theme.css",
                "assets/colors.png",
            ] {
                let _ = std::fs::remove_file(self.0.join(file));
            }
            for directory in ["defs", "assets"] {
                let _ = std::fs::remove_dir(self.0.join(directory));
            }
            let _ = std::fs::remove_dir(&self.0);
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "opbrowser-css-images-{}-{stamp}",
        std::process::id()
    )));
    std::fs::create_dir(&fixture.0).unwrap();
    for directory in ["defs", "assets"] {
        std::fs::create_dir(fixture.0.join(directory)).unwrap();
    }
    std::fs::write(
        fixture.0.join("defs/vars.css"),
        "p { --icon:url(colors.png) }",
    )
    .unwrap();
    std::fs::write(fixture.0.join("assets/theme.css"), ".demo::before { content:'A' var(--icon) 'B' url('colors.png') 'C' } .demo::after { content:url(colors.png); display:block; margin-top:8px }").unwrap();
    std::fs::write(
        fixture.0.join("assets/colors.png"),
        include_bytes!("../../../examples/images/colors.png"),
    )
    .unwrap();
    std::fs::write(fixture.0.join("index.html"), "<link rel='stylesheet' href='defs/vars.css'><link rel='stylesheet' href='assets/theme.css'><p><a class='demo' href='next.html'>Body</a><img src='assets/colors.png'></p>").unwrap();
    let mut engine = Engine::new();
    let original = engine
        .navigate(fixture.0.join("index.html").to_str().unwrap(), 800, 600)
        .unwrap()
        .display_list;
    let pixels = rasters(&original.commands);
    assert_eq!(pixels.len(), 4);
    assert!(pixels.iter().all(|image| Arc::ptr_eq(pixels[0], image)));
    let generated_links = original
        .commands
        .iter()
        .filter(|command| {
            matches!(command,
                PaintCommand::Image { href:Some(href), .. } if href == "next.html"
            )
        })
        .count();
    assert_eq!(generated_links, 3);
    let content = original
        .commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(content, "ABCBody");
    let first = pixels[0].clone();
    let source = fixture.0.join("index.html");
    drop(fixture);
    assert!(!source.exists());
    engine.reflow(240, 600).unwrap();
    let restored = engine.reflow(800, 600).unwrap().display_list;
    assert_eq!(restored, original);
    assert!(
        rasters(&restored.commands)
            .iter()
            .all(|image| Arc::ptr_eq(&first, image))
    );
}

#[test]
fn generated_and_dom_images_share_node_attempt_and_pixel_budgets() {
    let payload = percent(include_bytes!("../../../examples/images/colors.png"));
    let src = format!("data:image/png,{payload}");
    let html = format!(
        "<style>.image::before {{ content:url({src}) }}</style><body><div style='display:none'>{}</div>{}<img src='{src}' alt='node limit'><p>Tail</p></body>",
        "<span class=image></span>".repeat(40),
        "<span class=image></span>".repeat(33)
    );
    let page = Engine::new()
        .render_source(
            &format!("data:text/html,{}", percent(html.as_bytes())),
            800,
            600,
        )
        .unwrap();
    let images = rasters(&page.display_list.commands);
    assert_eq!(images.len(), 32);
    assert!(images.iter().all(|image| Arc::ptr_eq(images[0], image)));
    assert!(page.display_list.commands.iter().any(
        |command| matches!(command, PaintCommand::Text { text, .. } if text.contains("node limit"))
    ));
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
        let payload = percent(bytes);
        let css = (0..10)
            .map(|index| {
                format!(
                    "#image{index}::before {{ content:url(data:image/png,{payload}{}) }}",
                    "%00".repeat(index)
                )
            })
            .collect::<String>();
        let body = (0..10)
            .map(|index| format!("<span id=image{index}></span>"))
            .collect::<String>();
        let html = format!("<style>{css}</style><body>{body}</body>");
        let page = Engine::new()
            .render_source(
                &format!("data:text/html,{}", percent(html.as_bytes())),
                800,
                600,
            )
            .unwrap();
        assert_eq!(rasters(&page.display_list.commands).len(), expected);
    }
}

#[test]
fn redirected_css_images_share_cache_and_skip_hidden_blocked_and_failed_sources() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let responses: Vec<(&str, &str, &str, &[u8])> = vec![
            ("/index.html", "200 OK", "Content-Type: text/html\r\n", b"<link rel=stylesheet href='/theme.css'><p><a href='next.html'>Body</a><img src='/assets/icon.png'></p><div class=hidden>hidden</div><span class=off>Off</span><span class=failed>Body</span><span class=blocked>Blocked</span>"),
            ("/theme.css", "302 Found", "Location: /assets/theme.css\r\n", b""),
            ("/assets/theme.css", "200 OK", "Content-Type: text/css\r\n", b"a::before { content:'Before' url(icon.png) 'After' } a::after { content:url('icon.png') } .hidden { display:none } .hidden::before { content:url(hidden.png) } .off::before { display:none; content:url(off.png) } .failed::before { content:'ok' url(missing.png) 'tail' url(missing.png) } .blocked::before { content:url('file:///C:/private.png') }"),
            ("/assets/icon.png", "200 OK", "Content-Type: image/png\r\n", include_bytes!("../../../examples/images/colors.png")),
            ("/assets/missing.png", "404 Missing", "", b""),
        ];
        for (path, status, headers, bytes) in responses {
            let started = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(started.elapsed() < Duration::from_secs(5));
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = String::new();
            while !request.ends_with("\r\n\r\n") {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0 && request.len() < 32 * 1024);
                request.push_str(std::str::from_utf8(&buffer[..count]).unwrap());
            }
            assert!(
                request.starts_with(&format!("GET {path} HTTP/1.1")),
                "{request}"
            );
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
    let original = engine
        .navigate(&format!("{address}/index.html"), 800, 600)
        .unwrap()
        .display_list;
    server.join().unwrap();
    let images = rasters(&original.commands);
    assert_eq!(images.len(), 3);
    assert!(images.iter().all(|image| Arc::ptr_eq(images[0], image)));
    let text = original
        .commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(text.contains("oktailBody"));
    assert!(!text.contains("hidden"));
    assert!(!text.contains("[image]"));
    engine.reflow(240, 600).unwrap();
    assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
}
