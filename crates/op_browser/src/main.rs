use std::sync::mpsc;

use op_engine::{Engine, RenderedPage};
use op_platform_win::{NativeBrowserWindow, NavigationEvent};

const START_PAGE: &str = r#"
<html><head><title>OPBrowser</title><style>
body { color: #20232a; font-size: 18px; }
h1 { color: #6d28d9; font-size: 42px; }
.status { color: #087a35; font-size: 21px; font-weight: bold; }
.accent { color: #b42318; font-weight: bold; }
.demo-block { display: block; color: #075985; font-size: 20px; }
.box-demo { width: 70%; max-width: 620px; margin: 14px auto; padding: 3% 1.25em; box-sizing: border-box; background-color: #eef2ff; border-top: 2px solid #4338ca; border-right: 6px solid #7c3aed; border-bottom: 3px solid #4338ca; border-left: 6px solid #2563eb; color: #312e81; font-weight: bold; }
.hidden-proof { display: none; }
a { font-weight: bold; }
</style></head><body>
<h1>OPBrowser</h1>
<p class="status">CSS теперь проходит через resource loading → cascade → computed style → layout → Win32.</p>
<p>Поддерживаются встроенные стили, style="" и внешние link rel=stylesheet для local/file/HTTP(S).</p>
<p><a href="https://example.com">Открыть Example Domain</a></p>
<p>Работают <span class="accent">color, font-size, font-weight</span> и <span class="demo-block">display: block / inline / none.</span></p>
<div class="box-demo">Box model: width/max-width + margin:auto + %/em padding + box-sizing + independent borders.</div>
<p>Selectors: attributes, + / ~ siblings and :root/:first-child/:last-child/:only-child/:empty/:link now match in the author cascade.</p>
<p class="hidden-proof">Если вы видите эту строку, display:none сломан.</p>
<p>Введите https://example.com в адресной строке и нажмите Enter или Go.</p>
<p>Ctrl+L — выделить адрес. F5 — обновить. Back / Forward — история.</p>
<p>Колесо мыши — прокрутка страницы.</p>
<p>Работают HTTP, HTTPS, локальные HTML-файлы, data:text/html и PNG/JPEG/GIF/BMP.</p>
<p>Собственный движок: HTML → DOM → CSS resources/cascade → layout → display list → Win32.</p>
</body></html>
"#;

struct LoadCommand {
    event: NavigationEvent,
    width: i32,
    height: i32,
}

struct LoadResult {
    page: Result<Option<RenderedPage>, String>,
    back: bool,
    forward: bool,
    reload: bool,
    viewport: (i32, i32),
    reflow: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let source = args.iter().find(|argument| !argument.starts_with("--"));
    let smoke = args.iter().any(|argument| argument == "--smoke-test");
    let link_smoke = args.iter().any(|argument| argument == "--link-smoke-test");
    let image_smoke = args.iter().any(|argument| argument == "--image-smoke-test");
    let resize_smoke = args
        .iter()
        .any(|argument| argument == "--resize-smoke-test");
    let navigation_smoke = args
        .iter()
        .any(|argument| argument == "--navigation-smoke-test");
    let mut engine = Engine::new();
    engine.start();
    let display_list = engine.set_html_page(START_PAGE, 1200, 700);
    let window = NativeBrowserWindow::create("OPBrowser", display_list).unwrap_or_else(|error| {
        eprintln!("OPBrowser startup failed: {error}");
        std::process::exit(1);
    });
    let (width, height) = window.viewport_size();
    window.present_reflow(engine.reflow(width, height).unwrap().display_list);

    if smoke {
        if let Some(source) = source {
            let page = engine
                .navigate(source, width, height)
                .unwrap_or_else(|error| {
                    eprintln!("OPBrowser document load failed: {error}");
                    std::process::exit(3);
                });
            println!(
                "Loaded {} ({}, {} paint commands)",
                page.address,
                page.mime_type,
                page.display_list.commands.len()
            );
            window.present(&page.address, page.display_list);
        }
        if !window.painted_once() {
            eprintln!("OPBrowser smoke test failed: WM_PAINT did not run");
            std::process::exit(2);
        }
        return;
    }

    let (commands, requests) = mpsc::channel::<LoadCommand>();
    let (responses, results) = mpsc::channel::<LoadResult>();
    std::thread::spawn(move || {
        while let Ok(command) = requests.recv() {
            let LoadCommand {
                event,
                width,
                height,
            } = command;
            let reflow = event == NavigationEvent::Resize;
            let page = match event {
                NavigationEvent::Navigate(source) => {
                    engine.navigate(&source, width, height).map(Some)
                }
                NavigationEvent::FollowLink(href) => {
                    engine.follow_link(&href, width, height).map(Some)
                }
                NavigationEvent::Back => engine.go_back(width, height),
                NavigationEvent::Forward => engine.go_forward(width, height),
                NavigationEvent::Reload => engine.reload(width, height),
                NavigationEvent::Resize => Ok(engine.reflow(width, height)),
                NavigationEvent::Poll => continue,
            }
            .map_err(|error| error.to_string());
            let history = engine.navigation();
            if responses
                .send(LoadResult {
                    page,
                    back: history.can_go_back(),
                    forward: history.can_go_forward(),
                    reload: history.current().is_some(),
                    viewport: (width, height),
                    reflow,
                })
                .is_err()
            {
                break;
            }
        }
    });

    // Both CLI sources and the interactive smoke enter through the native Enter path.
    if let Some(source) = source {
        window.submit_address(source);
    } else if resize_smoke {
        window.submit_address("examples/navigation/resize.html");
    } else if image_smoke {
        window.submit_address("examples/images/index.html");
    } else if link_smoke {
        window.submit_address("examples/navigation/index.html");
    } else if navigation_smoke {
        window.submit_address("data:text/html,%3Ch1%3ENavigation%20smoke%3C%2Fh1%3E");
    }

    let mut busy = false;
    let mut back = false;
    let mut forward = false;
    let mut reload = false;
    let mut smoke_exit = 0;
    let mut link_smoke_clicked = false;
    let mut awaiting_navigation = false;
    let mut last_error = None;
    let mut resize_phase = 0;
    let mut wide_text_count = 0;
    let mut presented_viewport = (width, height);
    let (smoke_done, smoke_watch) = mpsc::channel::<()>();
    if resize_smoke {
        std::thread::spawn(move || {
            if smoke_watch.recv_timeout(std::time::Duration::from_secs(10))
                == Err(mpsc::RecvTimeoutError::Timeout)
            {
                eprintln!("OPBrowser resize smoke timed out");
                std::process::exit(4);
            }
        });
    }
    let exit_code = window.run_message_loop(|event| {
        if event == NavigationEvent::Poll {
            match results.try_recv() {
                Ok(result) => {
                    busy = false;
                    back = result.back;
                    forward = result.forward;
                    reload = result.reload;
                    let mut page_loaded = false;
                    let stale_size = result.viewport != window.viewport_size();
                    match result.page {
                        Ok(Some(page)) => {
                            awaiting_navigation |= !result.reflow;
                            if !result.reflow {
                                last_error = None;
                            }
                            if !stale_size {
                                println!(
                                    "{} {} ({}, {} paint commands)",
                                    if result.reflow { "Reflowed" } else { "Loaded" },
                                    page.address,
                                    page.mime_type,
                                    page.display_list.commands.len()
                                );
                                let text_count = page.display_list.commands.iter()
                                    .filter(|command| matches!(command, op_paint::PaintCommand::Text { .. }))
                                    .count();
                                if resize_smoke && resize_phase == 0 {
                                    wide_text_count = text_count;
                                }
                                if resize_smoke && resize_phase == 1
                                    && (result.viewport.0 != 320 || text_count <= wide_text_count)
                                {
                                    eprintln!("OPBrowser resize smoke failed: latest width did not rewrap text");
                                    smoke_exit = 2;
                                }
                                if awaiting_navigation {
                                    window.present(&page.address, page.display_list);
                                    awaiting_navigation = false;
                                } else {
                                    window.present_reflow(page.display_list);
                                }
                                presented_viewport = result.viewport;
                                window.set_status(last_error.as_deref().unwrap_or("Ready"));
                                page_loaded = true;
                                if (navigation_smoke || link_smoke || image_smoke || resize_smoke)
                                    && !window.painted_once()
                                {
                                    smoke_exit = 2;
                                }
                                if (image_smoke || (resize_smoke && resize_phase < 2))
                                    && window.painted_image_count() == 0
                                {
                                    eprintln!("OPBrowser image smoke failed: no raster image painted");
                                    smoke_exit = 2;
                                }
                            }
                        }
                        Ok(None) => window.set_status(last_error.as_deref().unwrap_or("Ready")),
                        Err(error) => {
                            eprintln!("OPBrowser document load failed: {error}");
                            last_error = Some(format!("Load failed: {error}"));
                            window.set_status(last_error.as_deref().unwrap());
                            if navigation_smoke || link_smoke || image_smoke || resize_smoke {
                                smoke_exit = 3;
                            }
                        }
                    }
                    if stale_size && smoke_exit == 0 {
                        let (width, height) = window.viewport_size();
                        busy = commands.send(LoadCommand {
                            event: NavigationEvent::Resize,
                            width,
                            height,
                        }).is_ok();
                        window.set_navigation_state(back, forward, reload, busy);
                        return;
                    }
                    window.set_navigation_state(back, forward, reload, busy);
                    if resize_smoke && page_loaded && resize_phase == 0 && smoke_exit == 0 {
                        resize_phase = 1;
                        window.resize_viewport(640, 500);
                        window.resize_viewport(480, 500);
                        window.resize_viewport(800, 500);
                    } else if (link_smoke || (resize_smoke && resize_phase == 1))
                        && page_loaded && !link_smoke_clicked && smoke_exit == 0
                    {
                        if window.click_first_link() {
                            link_smoke_clicked = true;
                            if resize_smoke {
                                resize_phase = 2;
                            }
                        } else {
                            eprintln!("OPBrowser link smoke failed: no visible clickable link");
                            smoke_exit = 2;
                            window.close();
                        }
                    } else if navigation_smoke || link_smoke || image_smoke || resize_smoke {
                        if (link_smoke || resize_smoke) && page_loaded && !back {
                            smoke_exit = 2;
                        }
                        window.close();
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    busy = false;
                    window.set_status("Navigation worker stopped");
                    window.set_navigation_state(back, forward, reload, busy);
                    if navigation_smoke || link_smoke || image_smoke || resize_smoke {
                        smoke_exit = 3;
                        window.close();
                    }
                }
            }
        } else if !busy {
            let (width, height) = window.viewport_size();
            if event == NavigationEvent::Resize && (width, height) == presented_viewport {
                return;
            }
            let reflow = event == NavigationEvent::Resize;
            if commands
                .send(LoadCommand {
                    event,
                    width,
                    height,
                })
                .is_ok()
            {
                busy = true;
                window.set_status(if reflow { "Layout..." } else { "Loading..." });
                window.set_navigation_state(back, forward, reload, busy);
                if resize_smoke && resize_phase == 1 && reflow && width == 800 {
                    // Change size while the worker owns an older viewport request.
                    window.resize_viewport(480, 500);
                    window.resize_viewport(320, 500);
                }
            }
        }
    });
    drop(smoke_done);
    drop(commands);
    std::process::exit(if smoke_exit != 0 {
        smoke_exit
    } else {
        exit_code
    });
}
