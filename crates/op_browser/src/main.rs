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
    <p>Chromium внутри: 0%.</p>
  </body>
</html>
"#;

fn main() {
    let mut engine = Engine::new();
    engine.start();

    let display_list = engine.render_html(START_PAGE, 1280, 800);

    let window = match NativeBrowserWindow::create("OPBrowser", display_list) {
        Ok(window) => window,
        Err(error) => {
            eprintln!("OPBrowser startup failed: {error}");
            std::process::exit(1);
        }
    };

    if std::env::args().any(|arg| arg == "--smoke-test") {
        if !window.painted_once() {
            eprintln!("OPBrowser smoke test failed: WM_PAINT did not run");
            std::process::exit(2);
        }
        return;
    }

    let exit_code = window.run_message_loop();
    std::process::exit(exit_code);
}
