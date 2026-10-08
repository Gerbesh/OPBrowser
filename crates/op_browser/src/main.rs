use std::sync::mpsc;

use op_engine::{Engine, RenderedPage};
use op_platform_win::{NativeBrowserWindow, NavigationEvent};

const START_PAGE: &str = r#"
<!doctype html>
<html><head><title>OPBrowser</title><style>
body { --op-accent:#b42318; --op-surface:#eef2ff; --op-edge:#4338ca; --op-gap:4px 8px; color: #20232a; font-size: 18px; }
h1 { color: #6d28d9; font-size: 42px; }
.status { color: #087a35; font-size: 21px; font-weight: bold; }
.accent { color: #b42318; font-weight: bold; }
.demo-block { display: block; color: #075985; font-size: 20px; }
.box-demo { width: 70%; max-width: 620px; margin: 14px auto; padding: 3% 1.25em; box-sizing: border-box; background-color: #eef2ff; border-top: 2px solid #4338ca; border-right: 6px solid #7c3aed; border-bottom: 3px solid #4338ca; border-left: 6px solid #2563eb; color: #312e81; font-weight: bold; }
.type-demo { width: 70%; margin: 12px auto; padding: 10px; text-align: center; line-height: 1.7; font-weight: 650; background-color: #f8fafc; border: 2px solid #0369a1; }
.inline-css-demo { width: 70%; margin: 12px auto; padding: 10px; font-style: italic; text-decoration: underline line-through; white-space: pre-wrap; background-color: #fff7ed; border: 2px solid #ea580c; }
.spacing-demo { width: 70%; margin: 12px auto; padding: 10px; text-transform: uppercase; letter-spacing: 3px; word-spacing: 8px; background-color: #f0fdf4; border: 2px solid #16a34a; }
.inline-box-chip { padding: 3px 8px; background-color: #ede9fe; border: 2px solid #7c3aed; color: #4c1d95; font-weight: bold; }
.functional-demo > span:is(.hot,.warm):not(.skip):nth-child(odd) { padding: 2px 6px; background: #eef2ff; border: 2px solid #4338ca; color: #b42318; }
.functional-demo > span:where(#functional-third) { font-weight: bold; }
.generated-demo::before { content: "[CSS before] "; padding: 2px 6px; background: #eef2ff; border: 1px solid #4338ca; color: #b42318; font-weight: bold; }
.generated-demo::after { content: " ✓ after"; color: #087a35; font-weight: bold; }
.var-demo { width:70%; margin:12px auto; padding:var(--op-gap); color:var(--op-accent); background:var(--op-surface); border:2px solid var(--op-edge); font-weight:bold; }
.var-demo::before { --var-label:"[var() inherited] "; content:var(--var-label); color:var(--op-edge); }
.counter-demo { width:70%; margin:12px auto; padding:8px; counter-reset:step; background:#f8fafc; border:2px solid #0f766e; box-sizing:border-box; }
.counter-demo p { counter-increment:step; margin:4px 0; }
.counter-demo p::before { content:attr(data-label) " " counter(step, decimal-leading-zero) ": "; color:#0f766e; font-weight:bold; }
.counter-demo::after { content:"Generated total: " counter(step); color:#087a35; font-weight:bold; }
.quote-demo { quotes:"«" "»" "‹" "›"; }
.quote-demo q::before, .quote-demo q::after { color:#7c3aed; }
.generated-block-demo::before { display:block; content:"CSS generated block"; width:60%; margin:8px auto; padding:6px; border:2px solid #0369a1; background:#eff6ff; text-align:center; box-sizing:border-box; }
.generated-block-demo::after { display:block; content:""; width:100px; height:12px; margin:6px auto; background:#7c3aed; }
.hidden-proof { display: none; }
a { font-weight: bold; }
</style></head><body>
<h1>OPBrowser</h1>
<p class="status">CSS теперь проходит через resource loading → cascade → computed style → layout → Win32.</p>
<p>Поддерживаются встроенные стили, style="" и внешние link rel=stylesheet для local/file/HTTP(S).</p>
<p><a href="https://example.com">Открыть Example Domain</a></p>
<p>Работают <span class="accent">color, font-size, font-weight</span> и <span class="demo-block">display: block / inline / none.</span></p>
<div class="box-demo">Box model: width/max-width + margin:auto + %/em padding + box-sizing + independent borders.</div>
<p>Selectors: attributes, + / ~ siblings, structural pseudos and :is()/:where()/:not()/:nth-child() now match in the author cascade.</p>
<p style="color:rgb(180 35 24); background-color:hsl(245 100% 97%); border:2px solid rgb(67 56 202); padding:8px">Colors: legacy/modern rgb()/rgba() and hsl()/hsla() now feed text, background and borders.</p>
<div class="type-demo">Typography: text-align:center, inherited line-height:1.7 and numeric font-weight:650 now affect real line geometry.<br>Second centered line proves line-height reaches layout.</div>
<div class="inline-css-demo">font-style:italic + underline + line-through
white-space:pre-wrap preserves this newline and  double spaces.</div>
<div class="spacing-demo">text-transform uppercase + letter-spacing + word-spacing now change layout and native paint.</div>
<p>Inline fragments: normal text <span class="inline-box-chip">padded <b>bold</b> span with background + border that can wrap across lines</span> and normal text again.</p>
<p class="functional-demo"><span class="hot">:is + odd</span> <span class="hot skip">:not blocks this</span> <span id="functional-third" class="warm">third + :where bold</span></p>
<p class="generated-demo">Real DOM text between generated pseudo-elements.</p>
<div class="var-demo">Custom properties feed color, background, padding, border and generated content; missing values can use var() fallbacks.</div>
<div class="counter-demo"><p data-label="Stage">Generated attr() + counter() content</p><p data-label="Stage">The counter increments in document order</p></div>
<p class="quote-demo"><q>Inherited CSS quotes with <q>a nested quotation</q> now reach native paint.</q></p>
<div class="generated-block-demo">Generated blocks now use real dimensions, margins, padding, backgrounds and borders.</div>
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
        let mut last_viewport = (width, height);
        loop {
            // Park without polling when there are no timers. Otherwise
            // wait for the earliest deadline or the next native command.
            let next_command = match engine.next_timer_wait() {
                Some(wait) => requests.recv_timeout(wait.max(std::time::Duration::from_millis(1))),
                None => requests
                    .recv()
                    .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
            };
            let command = match next_command {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(page) = engine.tick_timers(last_viewport.0, last_viewport.1) {
                        let history = engine.navigation();
                        if responses
                            .send(LoadResult {
                                page: Ok(Some(page)),
                                back: history.can_go_back(),
                                forward: history.can_go_forward(),
                                reload: history.current().is_some(),
                                viewport: last_viewport,
                                reflow: true,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    continue;
                }
            };
            let LoadCommand {
                event,
                width,
                height,
            } = command;
            last_viewport = (width, height);
            let reflow = matches!(
                event,
                NavigationEvent::Resize | NavigationEvent::Click { .. }
            );
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
                NavigationEvent::Click { x, y } => Ok(engine.click_at(x, y, width, height)),
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
            let reflow = matches!(event, NavigationEvent::Resize | NavigationEvent::Click { .. });
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
