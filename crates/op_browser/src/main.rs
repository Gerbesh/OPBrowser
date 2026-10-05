use std::sync::mpsc;

use op_engine::{Engine, RenderedPage};
use op_platform_win::{NativeBrowserWindow, NavigationEvent};

const START_PAGE: &str = r#"
<html><head><title>OPBrowser</title></head><body>
<h1>OPBrowser</h1>
<p>Введите https://example.com в адресной строке и нажмите Enter или Go.</p>
<p>Ctrl+L — выделить адрес. F5 — обновить. Back / Forward — история.</p>
<p>Колесо мыши — прокрутка страницы.</p>
<p>Работают HTTP, HTTPS, локальные HTML-файлы и data:text/html.</p>
<p>Собственный движок: HTML → DOM → layout → display list → Win32.</p>
<p>Пока отображается текст HTML. CSS, изображения и JavaScript ещё в разработке.</p>
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
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let source = args.iter().find(|argument| !argument.starts_with("--"));
    let smoke = args.iter().any(|argument| argument == "--smoke-test");
    let navigation_smoke = args
        .iter()
        .any(|argument| argument == "--navigation-smoke-test");
    let mut engine = Engine::new();
    engine.start();
    let display_list = engine.render_html(START_PAGE, 1200, 700);
    let window = NativeBrowserWindow::create("OPBrowser", display_list).unwrap_or_else(|error| {
        eprintln!("OPBrowser startup failed: {error}");
        std::process::exit(1);
    });
    let (width, height) = window.viewport_size();

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
            let page = match event {
                NavigationEvent::Navigate(source) => {
                    engine.navigate(&source, width, height).map(Some)
                }
                NavigationEvent::Back => engine.go_back(width, height),
                NavigationEvent::Forward => engine.go_forward(width, height),
                NavigationEvent::Reload => engine.reload(width, height),
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
    } else if navigation_smoke {
        window.submit_address("data:text/html,%3Ch1%3ENavigation%20smoke%3C%2Fh1%3E");
    }

    let mut busy = false;
    let mut back = false;
    let mut forward = false;
    let mut reload = false;
    let mut smoke_exit = 0;
    let exit_code = window.run_message_loop(|event| {
        if event == NavigationEvent::Poll {
            match results.try_recv() {
                Ok(result) => {
                    busy = false;
                    back = result.back;
                    forward = result.forward;
                    reload = result.reload;
                    match result.page {
                        Ok(Some(page)) => {
                            println!(
                                "Loaded {} ({}, {} paint commands)",
                                page.address,
                                page.mime_type,
                                page.display_list.commands.len()
                            );
                            window.present(&page.address, page.display_list);
                            window.set_status("Ready");
                            if navigation_smoke && !window.painted_once() {
                                smoke_exit = 2;
                            }
                        }
                        Ok(None) => window.set_status("Ready"),
                        Err(error) => {
                            eprintln!("OPBrowser document load failed: {error}");
                            window.set_status(&format!("Load failed: {error}"));
                            if navigation_smoke {
                                smoke_exit = 3;
                            }
                        }
                    }
                    window.set_navigation_state(back, forward, reload, busy);
                    if navigation_smoke {
                        window.close();
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    busy = false;
                    window.set_status("Navigation worker stopped");
                    window.set_navigation_state(back, forward, reload, busy);
                    if navigation_smoke {
                        smoke_exit = 3;
                        window.close();
                    }
                }
            }
        } else if !busy {
            let (width, height) = window.viewport_size();
            if commands
                .send(LoadCommand {
                    event,
                    width,
                    height,
                })
                .is_ok()
            {
                busy = true;
                window.set_status("Loading...");
                window.set_navigation_state(back, forward, reload, busy);
            }
        }
    });
    drop(commands);
    std::process::exit(if smoke_exit != 0 {
        smoke_exit
    } else {
        exit_code
    });
}
