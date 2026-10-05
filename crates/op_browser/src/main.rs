use op_engine::Engine;
use op_platform_win::NativeBrowserWindow;

const START_PAGE: &str = r#"
<html>
  <head>
    <title>OPBrowser</title>
  </head>
  <body>
    <h1>OPBrowser</h1>
    <p>Первый видимый рендер нашего собственного движка.</p>
    <p>HTML → DOM → layout → display list → Win32.</p>
    <p>Передай путь к .html или data:text/html,... первым аргументом.</p>
    <p>Chromium внутри: 0%.</p>
  </body>
</html>
"#;

const VIEWPORT_WIDTH: i32 = 1280;
const VIEWPORT_HEIGHT: i32 = 800;

fn main() {
    let mut engine = Engine::new();
    engine.start();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let source = args.iter().find(|argument| !argument.starts_with("--"));

    let (display_list, window_title) = match source {
        Some(source) => match engine.render_source(source, VIEWPORT_WIDTH, VIEWPORT_HEIGHT) {
            Ok(page) => (page.display_list, format!("OPBrowser - {}", page.address)),
            Err(error) => {
                eprintln!("OPBrowser document load failed: {error}");
                std::process::exit(3);
            }
        },
        None => (
            engine.render_html(START_PAGE, VIEWPORT_WIDTH, VIEWPORT_HEIGHT),
            "OPBrowser".to_owned(),
        ),
    };

    let window = match NativeBrowserWindow::create(&window_title, display_list) {
        Ok(window) => window,
        Err(error) => {
            eprintln!("OPBrowser startup failed: {error}");
            std::process::exit(1);
        }
    };

    if args.iter().any(|argument| argument == "--smoke-test") {
        if !window.painted_once() {
            eprintln!("OPBrowser smoke test failed: WM_PAINT did not run");
            std::process::exit(2);
        }
        return;
    }

    let exit_code = window.run_message_loop();
    std::process::exit(exit_code);
}
