use op_engine::Engine;
use op_platform_win::NativeBrowserWindow;

fn main() {
    let mut engine = Engine::new();
    engine.start();

    let window = match NativeBrowserWindow::create("OPBrowser") {
        Ok(window) => window,
        Err(error) => {
            eprintln!("OPBrowser startup failed: {error}");
            std::process::exit(1);
        }
    };

    if std::env::args().any(|arg| arg == "--smoke-test") {
        return;
    }

    let exit_code = window.run_message_loop();
    std::process::exit(exit_code);
}
