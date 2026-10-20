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
    fallback_scroll: Option<i32>,
    focus_scroll_bounds: Option<(i32, i32)>,
}

/// Keep the first/last capture events in order, but collapse repeated
/// movements while the layout worker is busy. A drag must not be lost when
/// the pointer is released before the preceding frame finishes painting.
fn queue_thumb_event(
    queued: &mut std::collections::VecDeque<NavigationEvent>,
    event: NavigationEvent,
) {
    match event {
        NavigationEvent::ScrollbarDragMove { y } => {
            if let Some(NavigationEvent::ScrollbarDragMove { y: last_y }) = queued.back_mut() {
                *last_y = y;
            } else {
                queued.push_back(NavigationEvent::ScrollbarDragMove { y });
            }
        }
        NavigationEvent::Hover { x, y } => {
            if let Some(NavigationEvent::Hover {
                x: last_x,
                y: last_y,
            }) = queued.back_mut()
            {
                *last_x = x;
                *last_y = y;
            } else {
                queued.push_back(NavigationEvent::Hover { x, y });
            }
        }
        NavigationEvent::Resize => {
            // A busy render must not swallow the latest native window size.
            // All pending resize events use the current viewport when replayed.
            if !queued.contains(&NavigationEvent::Resize) {
                queued.push_back(NavigationEvent::Resize);
            }
        }
        NavigationEvent::ScrollbarDragStart { .. }
        | NavigationEvent::ScrollbarDragEnd
        | NavigationEvent::PointerDown { .. }
        | NavigationEvent::PointerCancel
        | NavigationEvent::FocusNext { .. }
        | NavigationEvent::ActivateFocused
        | NavigationEvent::TextInput(_)
        | NavigationEvent::KeyDown { .. }
        | NavigationEvent::KeyUp { .. }
        | NavigationEvent::Click { .. }
        | NavigationEvent::FollowLinkAt { .. } => {
            queued.push_back(event);
        }
        _ => {}
    }
}

/// Native reflows (hover, press, and resize) can arrive before the queued
/// hyperlink navigation. The smoke test is complete only after history moves.
fn should_close_link_smoke(clicked: bool, can_go_back: bool, smoke_exit: i32) -> bool {
    smoke_exit != 0 || (clicked && can_go_back)
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
                                fallback_scroll: None,
                                focus_scroll_bounds: None,
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
            let keyboard_focus = matches!(event, NavigationEvent::FocusNext { .. })
                || matches!(event, NavigationEvent::KeyDown { ref key, .. } if key == "Tab");
            let keyboard_activate = matches!(event, NavigationEvent::ActivateFocused)
                || matches!(event, NavigationEvent::KeyDown { ref key, .. } if key == "Enter");
            let before_navigation = (
                engine.navigation().current_index(),
                engine.navigation().entries().len(),
            );
            let mut reflow = matches!(
                event,
                NavigationEvent::Resize
                    | NavigationEvent::Click { .. }
                    | NavigationEvent::PointerDown { .. }
                    | NavigationEvent::PointerCancel
                    | NavigationEvent::FocusNext { .. }
                    | NavigationEvent::ActivateFocused
                    | NavigationEvent::TextInput(_)
                    | NavigationEvent::KeyDown { .. }
                    | NavigationEvent::KeyUp { .. }
                    | NavigationEvent::Hover { .. }
                    | NavigationEvent::Scroll { .. }
                    | NavigationEvent::HorizontalScroll { .. }
                    | NavigationEvent::ScrollbarDragStart { .. }
                    | NavigationEvent::ScrollbarDragMove { .. }
                    | NavigationEvent::ScrollbarDragEnd
            );
            let mut fallback_scroll = None;
            let page = match event {
                NavigationEvent::Navigate(source) => {
                    engine.navigate(&source, width, height).map(Some)
                }
                NavigationEvent::FollowLinkAt { href, x, y } => {
                    engine.activate_link_at(&href, x, y, width, height)
                }
                NavigationEvent::Back => engine.go_back(width, height),
                NavigationEvent::Forward => engine.go_forward(width, height),
                NavigationEvent::Reload => engine.reload(width, height),
                NavigationEvent::Resize => Ok(engine.reflow(width, height)),
                NavigationEvent::Click { x, y } => Ok(engine.click_at(x, y, width, height)),
                NavigationEvent::PointerDown { x, y } => Ok(engine.press_at(x, y, width, height)),
                NavigationEvent::PointerCancel => Ok(engine.release_press(width, height)),
                NavigationEvent::FocusNext { backwards } => {
                    Ok(engine.focus_next(backwards, width, height))
                }
                NavigationEvent::ActivateFocused => engine.activate_focused(width, height),
                NavigationEvent::TextInput(ref text) => Ok(engine.text_input(text, width, height)),
                NavigationEvent::KeyDown {
                    ref key,
                    ref code,
                    shift,
                    ctrl,
                    repeat,
                } => engine.keyboard_event(
                    op_engine::NativeKeyboardInput {
                        key,
                        code,
                        down: true,
                        shift,
                        ctrl,
                        repeat,
                    },
                    width,
                    height,
                ),
                NavigationEvent::KeyUp {
                    ref key,
                    ref code,
                    shift,
                    ctrl,
                } => engine.keyboard_event(
                    op_engine::NativeKeyboardInput {
                        key,
                        code,
                        down: false,
                        shift,
                        ctrl,
                        repeat: false,
                    },
                    width,
                    height,
                ),
                NavigationEvent::Hover { x, y } => Ok(engine.hover_at(x, y, width, height)),
                NavigationEvent::Scroll { x, y, delta_y } => {
                    let page = engine.scroll_at(x, y, delta_y, width, height);
                    if page.is_none() {
                        fallback_scroll = Some(delta_y);
                    }
                    Ok(page)
                }
                NavigationEvent::HorizontalScroll { x, y, delta_x } => {
                    Ok(engine.scroll_horizontal_at(x, y, delta_x, width, height))
                }
                NavigationEvent::ScrollbarDragStart { x, y } => {
                    engine.start_scrollbar_drag_at(x, y, width, height);
                    Ok(None)
                }
                NavigationEvent::ScrollbarDragMove { y } => {
                    Ok(engine.move_scrollbar_drag_to(y, width, height))
                }
                NavigationEvent::ScrollbarDragEnd => {
                    engine.end_scrollbar_drag();
                    Ok(None)
                }
                NavigationEvent::Poll => continue,
            }
            .map_err(|error| error.to_string());
            if keyboard_activate
                && before_navigation
                    != (
                        engine.navigation().current_index(),
                        engine.navigation().entries().len(),
                    )
            {
                // Enter followed an anchor: present the full navigation and URL.
                reflow = false;
            }
            let focus_scroll_bounds = keyboard_focus
                .then(|| engine.take_focus_scroll_bounds())
                .flatten();
            let history = engine.navigation();
            if responses
                .send(LoadResult {
                    page,
                    back: history.can_go_back(),
                    forward: history.can_go_forward(),
                    reload: history.current().is_some(),
                    viewport: (width, height),
                    reflow,
                    fallback_scroll,
                    focus_scroll_bounds,
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
    // Preserve capture start/end even while a previous layout is rendering;
    // coalesce intermediate thumb positions to the newest mouse location.
    let mut queued_thumb_events = std::collections::VecDeque::new();
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
    if resize_smoke || link_smoke {
        std::thread::spawn(move || {
            if smoke_watch.recv_timeout(std::time::Duration::from_secs(10))
                == Err(mpsc::RecvTimeoutError::Timeout)
            {
                eprintln!("OPBrowser native link/resize smoke timed out before navigation");
                std::process::exit(4);
            }
        });
    }
    let exit_code = window.run_message_loop(|event| {
        if busy && event != NavigationEvent::Poll {
            queue_thumb_event(&mut queued_thumb_events, event);
            return;
        }
        if event == NavigationEvent::Poll {
            match results.try_recv() {
                Ok(result) => {
                    busy = false;
                    back = result.back;
                    forward = result.forward;
                    reload = result.reload;
                    let mut page_loaded = false;
                    let stale_size = result.viewport != window.viewport_size();
                    if resize_smoke {
                        eprintln!(
                            "Resize smoke: result={:?}, actual={:?}, phase={}, stale={}, back={}, clicked={}",
                            result.viewport,
                            window.viewport_size(),
                            resize_phase,
                            stale_size,
                            back,
                            link_smoke_clicked
                        );
                    }
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
                                if resize_smoke
                                    && resize_phase == 1
                                    && result.viewport.0 == 320
                                    && text_count <= wide_text_count
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
                                if let Some((top, bottom)) = result.focus_scroll_bounds {
                                    window.reveal_document_bounds(top, bottom);
                                }
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
                        Ok(None) => {
                            if let Some(delta) = result.fallback_scroll {
                                window.scroll_document_by(delta);
                            }
                            window.set_status(last_error.as_deref().unwrap_or("Ready"));
                        },
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
                        eprintln!("Resize smoke: enqueue initial triple resize");
                        resize_phase = 1;
                        window.resize_viewport(640, 500);
                        window.resize_viewport(480, 500);
                        window.resize_viewport(800, 500);
                        // Send the wide request explicitly, then shrink while
                        // the worker owns it. The stale-size path must retry
                        // at 320px even if Win32 coalesces WM_SIZE timers.
                        let (width, height) = window.viewport_size();
                        busy = commands.send(LoadCommand {
                            event: NavigationEvent::Resize,
                            width,
                            height,
                        }).is_ok();
                        window.set_navigation_state(back, forward, reload, busy);
                        window.resize_viewport(480, 500);
                        window.resize_viewport(320, 500);
                    } else if (link_smoke
                        || (resize_smoke && resize_phase == 1 && presented_viewport.0 == 320))
                        && page_loaded
                        && !link_smoke_clicked
                        && smoke_exit == 0
                    {
                        if window.click_first_link() {
                            if resize_smoke {
                                eprintln!("Resize smoke: click queued at {:?}", presented_viewport);
                            }
                            link_smoke_clicked = true;
                            if resize_smoke {
                                resize_phase = 2;
                            }
                        } else {
                            eprintln!("OPBrowser link smoke failed: no visible clickable link");
                            smoke_exit = 2;
                            window.close();
                        }
                    } else if navigation_smoke
                        || image_smoke
                        || ((link_smoke || resize_smoke)
                            && should_close_link_smoke(link_smoke_clicked, back, smoke_exit))
                    {
                        window.close();
                    }
                    // Serialize native capture events through the render worker.
                    // Starting the next command re-arms the UI's Poll timer.
                    if !busy && let Some(next) = queued_thumb_events.pop_front() {
                        let (width, height) = window.viewport_size();
                        busy = commands.send(LoadCommand {
                            event: next,
                            width,
                            height,
                        }).is_ok();
                        window.set_navigation_state(back, forward, reload, busy);
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
            let reflow = matches!(
                event,
                NavigationEvent::Resize | NavigationEvent::Click { .. }
                    | NavigationEvent::PointerDown { .. }
                    | NavigationEvent::PointerCancel
                    | NavigationEvent::FocusNext { .. }
                    | NavigationEvent::ActivateFocused
                    | NavigationEvent::TextInput(_)
                    | NavigationEvent::KeyDown { .. }
                    | NavigationEvent::KeyUp { .. }
                    | NavigationEvent::Hover { .. }
                    | NavigationEvent::Scroll { .. }
                    | NavigationEvent::HorizontalScroll { .. }
                    | NavigationEvent::ScrollbarDragStart { .. }
                    | NavigationEvent::ScrollbarDragMove { .. }
                    | NavigationEvent::ScrollbarDragEnd
            );
            if resize_smoke && event == NavigationEvent::Resize {
                eprintln!("Resize smoke: native Resize event at {width}x{height}");
            }
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

#[cfg(test)]
mod overflow_tests {
    use super::*;

    #[test]
    fn busy_native_resize_is_coalesced_not_lost() {
        let mut queued = std::collections::VecDeque::new();
        queue_thumb_event(&mut queued, NavigationEvent::Resize);
        queue_thumb_event(&mut queued, NavigationEvent::Hover { x: 5, y: 6 });
        queue_thumb_event(&mut queued, NavigationEvent::Resize);
        assert_eq!(queued.len(), 2);
        assert_eq!(queued.pop_front(), Some(NavigationEvent::Resize));
        assert_eq!(
            queued.pop_front(),
            Some(NavigationEvent::Hover { x: 5, y: 6 })
        );
    }

    #[test]
    fn native_link_smoke_waits_for_history_instead_of_failing_on_intermediate_reflow() {
        assert!(!should_close_link_smoke(false, false, 0));
        assert!(!should_close_link_smoke(true, false, 0));
        assert!(should_close_link_smoke(true, true, 0));
        assert!(should_close_link_smoke(false, false, 2));
    }

    #[test]
    fn worker_keeps_real_keydown_keyup_and_cancelable_enter_in_order() {
        let mut queue = std::collections::VecDeque::new();
        queue_thumb_event(
            &mut queue,
            NavigationEvent::KeyDown {
                key: "Tab".into(),
                code: "Tab".into(),
                shift: true,
                ctrl: false,
                repeat: false,
            },
        );
        queue_thumb_event(
            &mut queue,
            NavigationEvent::KeyUp {
                key: "Tab".into(),
                code: "Tab".into(),
                shift: true,
                ctrl: false,
            },
        );
        queue_thumb_event(
            &mut queue,
            NavigationEvent::KeyDown {
                key: "Enter".into(),
                code: "Enter".into(),
                shift: false,
                ctrl: false,
                repeat: false,
            },
        );
        assert_eq!(queue.len(), 3);
        assert!(
            matches!(queue.pop_front(), Some(NavigationEvent::KeyDown { ref key, shift: true, .. }) if key == "Tab")
        );
        assert!(
            matches!(queue.pop_front(), Some(NavigationEvent::KeyUp { ref key, .. }) if key == "Tab")
        );
        assert!(
            matches!(queue.pop_front(), Some(NavigationEvent::KeyDown { ref key, .. }) if key == "Enter")
        );
    }

    #[test]
    fn queued_keyboard_tab_steps_and_enter_keep_their_order() {
        let mut queue = std::collections::VecDeque::new();
        queue_thumb_event(&mut queue, NavigationEvent::FocusNext { backwards: false });
        queue_thumb_event(&mut queue, NavigationEvent::FocusNext { backwards: true });
        queue_thumb_event(&mut queue, NavigationEvent::ActivateFocused);
        assert_eq!(queue.len(), 3);
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::FocusNext { backwards: false })
        );
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::FocusNext { backwards: true })
        );
        assert_eq!(queue.pop_front(), Some(NavigationEvent::ActivateFocused));
    }

    #[test]
    fn pointer_hover_events_coalesce_to_latest_position_while_worker_is_busy() {
        let mut queue = std::collections::VecDeque::new();
        for x in 0..1000 {
            queue_thumb_event(&mut queue, NavigationEvent::Hover { x, y: 70 });
        }
        assert_eq!(queue.len(), 1);
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::Hover { x: 999, y: 70 })
        );
        queue_thumb_event(&mut queue, NavigationEvent::Hover { x: 1, y: 5 });
        queue_thumb_event(&mut queue, NavigationEvent::ScrollbarDragEnd);
        queue_thumb_event(&mut queue, NavigationEvent::Hover { x: -1, y: -1 });
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn busy_worker_retains_pointer_press_release_order_without_hover_flood() {
        let mut queue = std::collections::VecDeque::new();
        queue_thumb_event(&mut queue, NavigationEvent::PointerDown { x: 42, y: 35 });
        for x in 43..300 {
            queue_thumb_event(&mut queue, NavigationEvent::Hover { x, y: 35 });
        }
        queue_thumb_event(&mut queue, NavigationEvent::Click { x: 73, y: 35 });
        assert_eq!(queue.len(), 3);
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::PointerDown { x: 42, y: 35 })
        );
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::Hover { x: 299, y: 35 })
        );
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::Click { x: 73, y: 35 })
        );
        queue_thumb_event(&mut queue, NavigationEvent::PointerDown { x: 2, y: 3 });
        queue_thumb_event(&mut queue, NavigationEvent::PointerCancel);
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::PointerDown { x: 2, y: 3 })
        );
        assert_eq!(queue.pop_front(), Some(NavigationEvent::PointerCancel));
    }

    #[test]
    fn r12c_busy_drag_queue_coalesces_moves_without_losing_capture_edges() {
        let mut queue = std::collections::VecDeque::new();
        queue_thumb_event(
            &mut queue,
            NavigationEvent::ScrollbarDragStart { x: 41, y: 21 },
        );
        for y in 22..100 {
            queue_thumb_event(&mut queue, NavigationEvent::ScrollbarDragMove { y });
        }
        queue_thumb_event(&mut queue, NavigationEvent::ScrollbarDragEnd);
        assert_eq!(queue.len(), 3);
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::ScrollbarDragStart { x: 41, y: 21 })
        );
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::ScrollbarDragMove { y: 99 })
        );
        assert_eq!(queue.pop_front(), Some(NavigationEvent::ScrollbarDragEnd));
        queue_thumb_event(&mut queue, NavigationEvent::Click { x: 1, y: 2 });
        assert_eq!(
            queue.pop_front(),
            Some(NavigationEvent::Click { x: 1, y: 2 })
        );
    }

    fn rgb_at(pixels: &[u8], width: i32, x: i32, y: i32) -> (u8, u8, u8) {
        let index = ((y * width + x) * 4) as usize;
        (pixels[index + 2], pixels[index + 1], pixels[index])
    }

    #[test]
    fn r2c_shorthand_flex_one_and_percentage_basis_control_painted_width() {
        let engine = Engine::new();
        let read = |html: &str| {
            let page = engine.render_html(html, 430, 170);
            let widths: Vec<i32> = [(240, 0, 0), (0, 0, 240)]
                .iter()
                .map(|&(r, g, b)| {
                    page.commands
                        .iter()
                        .find_map(|command| {
                            let op_paint::PaintCommand::FillRect { width, color, .. } = command
                            else {
                                return None;
                            };
                            (color.r == r && color.g == g && color.b == b).then_some(*width)
                        })
                        .unwrap()
                })
                .collect();
            let pixels = op_platform_win::render_display_list_to_bgra(&page, 430, 170).unwrap();
            (widths, pixels)
        };
        let (widths, _) = read(
            "<body style='margin:0'><div style='display:flex;width:300px'>\
             <div style='width:20px;flex:1;height:30px;background:#f00000'></div>\
             <div style='width:80px;flex:1;height:30px;background:#0000f0'></div></div>",
        );
        assert_eq!(
            widths,
            [150, 150],
            "flex:1 ignores different CSS widths via 0% basis"
        );

        let (widths, _) = read(
            "<body style='margin:0'><div style='display:flex;width:300px'>\
             <div style='width:20px;flex-basis:50%;height:30px;background:#f00000'></div>\
             <div style='width:40px;height:30px;background:#0000f0'></div></div>",
        );
        assert_eq!(
            widths,
            [150, 40],
            "percentage basis uses containing flex width"
        );

        let (widths, _) = read(
            "<body style='margin:0'><div style='display:flex;width:300px;gap:10px'>\
             <div style='width:180px;flex-shrink:0;height:30px;background:#f00000'></div>\
             <div style='width:180px;flex-shrink:0;height:30px;background:#0000f0'></div></div>",
        );
        assert_eq!(
            widths,
            [180, 180],
            "shrink factor zero must retain overflow"
        );
    }

    #[test]
    fn r2c_shrink_weights_basis_and_minimums_paint_real_native_pixels() {
        let engine = Engine::new();
        let rects = |html: &str| {
            let page = engine.render_html(html, 440, 180);
            let red = page
                .commands
                .iter()
                .find_map(|cmd| {
                    let op_paint::PaintCommand::FillRect {
                        x, y, width, color, ..
                    } = cmd
                    else {
                        return None;
                    };
                    (color.r == 240 && color.g == 0 && color.b == 0).then_some((*x, *y, *width))
                })
                .expect("red flex box");
            let blue = page
                .commands
                .iter()
                .find_map(|cmd| {
                    let op_paint::PaintCommand::FillRect {
                        x, y, width, color, ..
                    } = cmd
                    else {
                        return None;
                    };
                    (color.r == 0 && color.g == 0 && color.b == 240).then_some((*x, *y, *width))
                })
                .expect("blue flex box");
            let pixels = op_platform_win::render_display_list_to_bgra(&page, 440, 180).unwrap();
            (red, blue, pixels)
        };
        let (red, blue, pixels) = rects(
            "<body style='margin:0'><div style='display:flex;width:300px;gap:10px'>\
             <div style='width:180px;flex-shrink:1;height:30px;background:#f00000'>R</div>\
             <div style='width:180px;flex-shrink:3;height:30px;background:#0000f0'>B</div></div>",
        );
        assert_eq!(red.2, 163, "1:3 shrink of 70px deficit");
        assert_eq!(blue.2, 127);
        assert_eq!(blue.0 - red.0, 173);
        assert_eq!(rgb_at(&pixels, 440, red.0 + 155, red.1 + 20), (240, 0, 0));
        assert_eq!(rgb_at(&pixels, 440, blue.0 + 120, blue.1 + 20), (0, 0, 240));

        let (red, blue, _) = rects(
            "<body style='margin:0'><div style='display:flex;width:300px;gap:10px'>\
             <div style='width:40px;flex-basis:120px;flex-grow:1;height:30px;background:#f00000'>R</div>\
             <div style='width:40px;flex-grow:1;height:30px;background:#0000f0'>B</div></div>",
        );
        assert_eq!(red.2, 185, "basis 120px plus 65px growth");
        assert_eq!(blue.2, 105, "basis auto resolves explicit width 40px");
        assert_eq!(blue.0 - red.0, 195);

        let (red, blue, _) = rects(
            "<body style='margin:0'><div style='display:flex;width:250px'>\
             <div style='width:200px;min-width:150px;height:30px;background:#f00000'>R</div>\
             <div style='width:200px;height:30px;background:#0000f0'>B</div></div>",
        );
        assert_eq!(red.2, 150, "first item freezes at min-width");
        assert_eq!(
            blue.2, 100,
            "remaining deficit redistributes to second item"
        );
    }

    #[test]
    fn r2b_growth_is_independent_for_each_wrapped_flex_line() {
        let engine = Engine::new();
        let page = engine.render_html(
            "<body style='margin:0'><div style='display:flex;flex-wrap:wrap;width:180px;gap:6px 10px'>\
             <div style='width:80px;height:30px;flex-grow:1;background:#f00000'></div>\
             <div style='width:80px;height:30px;flex-grow:1;background:#00f000'></div>\
             <div style='width:80px;height:30px;flex-grow:1;background:#0000f0'></div>\
             </div></body>",
            400, 180,
        );
        let colored = |r: u8, g: u8, b: u8| {
            page.commands
                .iter()
                .find_map(|command| {
                    let op_paint::PaintCommand::FillRect {
                        x, y, width, color, ..
                    } = command
                    else {
                        return None;
                    };
                    (color.r == r && color.g == g && color.b == b).then_some((*x, *y, *width))
                })
                .unwrap()
        };
        let red = colored(240, 0, 0);
        let green = colored(0, 240, 0);
        let blue = colored(0, 0, 240);
        assert_eq!(red.2, 85);
        assert_eq!(green.2, 85);
        assert_eq!(green.0 - red.0, 95);
        assert_eq!(green.1, red.1);
        assert_eq!(blue.2, 180);
        assert_eq!(blue.0, red.0);
        assert_eq!(blue.1 - red.1, 36);
        let pixels = op_platform_win::render_display_list_to_bgra(&page, 400, 180).unwrap();
        assert_eq!(rgb_at(&pixels, 400, blue.0 + 175, blue.1 + 10), (0, 0, 240));
    }

    #[test]
    fn r2b_justify_and_flex_grow_render_real_native_pixels() {
        let engine = Engine::new();
        let page = engine.render_html(
            "<body style='margin:0'><div style='display:flex;width:300px;gap:12px'>\
             <div style='width:60px;height:35px;flex-grow:1;background:#f00000'>Red</div>\
             <div style='width:60px;height:35px;flex-grow:2;background:#0000f0'>Blue</div>\
             </div></body>",
            400,
            180,
        );
        let rectangle = |r: u8, g: u8, b: u8| {
            page.commands
                .iter()
                .find_map(|cmd| {
                    let op_paint::PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } = cmd
                    else {
                        return None;
                    };
                    (color.r == r && color.g == g && color.b == b)
                        .then_some((*x, *y, *width, *height))
                })
                .expect("grown flex item must paint")
        };
        let red = rectangle(240, 0, 0);
        let blue = rectangle(0, 0, 240);
        assert_eq!(red.2, 116, "60 base + 1/3 of 168 free px");
        assert_eq!(blue.2, 172, "60 base + 2/3 of 168 free px");
        assert_eq!(blue.0 - red.0, 128, "grown width plus 12px gap");
        let pixels = op_platform_win::render_display_list_to_bgra(&page, 400, 180).unwrap();
        assert_eq!(rgb_at(&pixels, 400, red.0 + 110, red.1 + 20), (240, 0, 0));
        assert_eq!(rgb_at(&pixels, 400, blue.0 + 160, blue.1 + 20), (0, 0, 240));
        assert_eq!(
            rgb_at(&pixels, 400, red.0 + 120, red.1 + 20),
            (255, 255, 255)
        );
    }

    #[test]
    fn r2b_justify_content_centers_and_distributes_per_line() {
        let engine = Engine::new();
        let mut origin = None;
        for (alignment, first_x, second_x) in [
            ("flex-start", 0, 70),
            ("center", 85, 155),
            ("flex-end", 170, 240),
            ("space-between", 0, 240),
            ("space-around", 42, 197),
            ("space-evenly", 56, 183),
        ] {
            let html = format!(
                "<body style='margin:0'><div style='display:flex;width:300px;gap:10px;justify-content:{alignment}'>\
                <div style='width:60px;height:20px;background:#f00000'></div>\
                <div style='width:60px;height:20px;background:#0000f0'></div>\
                </div></body>"
            );
            let page = engine.render_html(&html, 400, 120);
            let pos = |r: u8, b: u8| {
                page.commands
                    .iter()
                    .find_map(|cmd| {
                        let op_paint::PaintCommand::FillRect { x, color, .. } = cmd else {
                            return None;
                        };
                        (color.r == r && color.b == b).then_some(*x)
                    })
                    .unwrap()
            };
            let base = *origin.get_or_insert(pos(240, 0));
            assert_eq!(pos(240, 0) - base, first_x, "{alignment} first item");
            assert_eq!(pos(0, 240) - base, second_x, "{alignment} second item");
        }
    }

    #[test]
    fn r2a_wrapped_flex_cards_paint_separate_gdi_rows() {
        let engine = Engine::new();
        let page = engine.render_html(
            "<body style='margin:0'><div style='display:flex;flex-wrap:wrap;gap:8px 15px;width:200px'>\
              <div style='width:90px;height:25px;background:#f00000'></div>\
              <div style='width:90px;height:25px;background:#00f000'></div>\
              <div style='width:90px;height:25px;background:#0000f0'></div>\
             </div></body>",
            400, 240,
        );
        let rectangle = |r: u8, g: u8, b: u8| {
            page.commands
                .iter()
                .find_map(|cmd| {
                    let op_paint::PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } = cmd
                    else {
                        return None;
                    };
                    (color.r == r && color.g == g && color.b == b)
                        .then_some((*x, *y, *width, *height))
                })
                .expect("flex item is actually painted")
        };
        let red = rectangle(240, 0, 0);
        let green = rectangle(0, 240, 0);
        let blue = rectangle(0, 0, 240);
        assert_eq!(green.0 - red.0, 105);
        assert_eq!(green.1, red.1);
        assert_eq!(blue.0, red.0);
        assert_eq!(blue.1 - red.1, 33);
        let pixels = op_platform_win::render_display_list_to_bgra(&page, 400, 240).unwrap();
        assert_eq!(rgb_at(&pixels, 400, red.0 + 5, red.1 + 5), (240, 0, 0));
        assert_eq!(rgb_at(&pixels, 400, green.0 + 5, green.1 + 5), (0, 240, 0));
        assert_eq!(rgb_at(&pixels, 400, blue.0 + 5, blue.1 + 5), (0, 0, 240));
    }

    #[test]
    fn r11_scroll_top_moves_native_pixels_inside_overflow_auto() {
        let mut engine = Engine::new();
        let without = engine.set_html_page(
            "<body><div id='scroll' style='width:140px;height:40px;overflow:auto'>\
             <div style='width:140px;height:40px;background:#ff0000'></div>\
             <div style='width:140px;height:40px;background:#0000ff'></div>\
             </div><script>document.getElementById('scroll').scrollTop=40;</script></body>",
            500,
            320,
        );
        let rect = without
            .commands
            .iter()
            .find_map(|cmd| {
                let op_paint::PaintCommand::Clipped { rect, command } = cmd else {
                    return None;
                };
                let op_paint::PaintCommand::FillRect { color, .. } = command.as_ref() else {
                    return None;
                };
                (color.b == 255 && color.r == 0).then_some(*rect)
            })
            .expect("blue content is clipped to original scroll padding box");
        let pixel = op_platform_win::render_display_list_to_bgra(&without, 500, 320).unwrap();
        assert_eq!(rgb_at(&pixel, 500, rect.x + 8, rect.y + 8), (0, 0, 255));
        assert_eq!(
            rgb_at(&pixel, 500, rect.x + 8, rect.y + rect.height + 8),
            (255, 255, 255)
        );

        let unscrolled = engine.render_html(
            "<body><div style='width:140px;height:40px;overflow:auto'>\
             <div style='width:140px;height:40px;background:#ff0000'></div>\
             <div style='width:140px;height:40px;background:#0000ff'></div>\
             </div></body>",
            500,
            320,
        );
        let original = op_platform_win::render_display_list_to_bgra(&unscrolled, 500, 320).unwrap();
        assert_eq!(rgb_at(&original, 500, rect.x + 8, rect.y + 8), (255, 0, 0));
    }

    #[test]
    fn r10_overflow_hidden_clips_real_pixels_but_visible_does_not() {
        let engine = Engine::new();
        let clipped=engine.render_html(
            "<body><div style='width:140px;height:42px;overflow:hidden'>             <div style='width:140px;height:140px;background:#ff0000'></div>             </div></body>",
            500,320
        );
        let (rect, child_height) = clipped
            .commands
            .iter()
            .find_map(|command| {
                let op_paint::PaintCommand::Clipped { rect, command } = command else {
                    return None;
                };
                let op_paint::PaintCommand::FillRect { height, color, .. } = command.as_ref()
                else {
                    return None;
                };
                (color.r == 255 && color.g == 0 && color.b == 0).then_some((*rect, *height))
            })
            .expect("red child paint command has CSS overflow ancestor clip");
        assert!(child_height > rect.height);
        let pixels = op_platform_win::render_display_list_to_bgra(&clipped, 500, 320).unwrap();
        assert_eq!(rgb_at(&pixels, 500, rect.x + 10, rect.y + 10), (255, 0, 0));
        assert_eq!(
            rgb_at(&pixels, 500, rect.x + 10, rect.y + rect.height + 10),
            (255, 255, 255)
        );

        let visible=engine.render_html(
            "<body><div style='width:140px;height:42px;overflow:visible'>             <div style='width:140px;height:140px;background:#ff0000'></div>             </div></body>",
            500,320
        );
        let pixels = op_platform_win::render_display_list_to_bgra(&visible, 500, 320).unwrap();
        assert_eq!(
            rgb_at(&pixels, 500, rect.x + 10, rect.y + rect.height + 10),
            (255, 0, 0)
        );
    }

    #[test]
    fn r10_nested_overflow_clips_to_intersection_of_parent_padding_boxes() {
        let engine = Engine::new();
        let list=engine.render_html(
            "<body><div style='height:64px;width:130px;overflow:hidden'>               <div style='height:100px;width:80px;overflow:clip'>                 <div style='height:180px;width:140px;background:#ff0000'></div>               </div>             </div></body>",
            500,320
        );
        let clip = list
            .commands
            .iter()
            .find_map(|command| {
                let op_paint::PaintCommand::Clipped { rect, command } = command else {
                    return None;
                };
                let op_paint::PaintCommand::FillRect { color, .. } = command.as_ref() else {
                    return None;
                };
                (color.r == 255 && color.g == 0 && color.b == 0).then_some(*rect)
            })
            .expect("child paint command clipped to nested CSS overflow");
        assert!(clip.width <= 80 && clip.height <= 64);
        let pixels = op_platform_win::render_display_list_to_bgra(&list, 500, 320).unwrap();
        assert_eq!(rgb_at(&pixels, 500, clip.x + 10, clip.y + 10), (255, 0, 0));
        assert_eq!(
            rgb_at(&pixels, 500, clip.x + clip.width + 8, clip.y + 10),
            (255, 255, 255)
        );
        assert_eq!(
            rgb_at(&pixels, 500, clip.x + 10, clip.y + clip.height + 8),
            (255, 255, 255)
        );
    }
}
