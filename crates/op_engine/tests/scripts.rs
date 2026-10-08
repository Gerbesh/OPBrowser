use op_engine::Engine;
use op_net::{LoadError, NetworkContext};

#[test]
fn parser_blocking_inline_script_cannot_observe_future_dom_nodes() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        "<p id='earlier'>Earlier</p>         <script>var visible='WRONG';         if (document.getElementById('later') == null) { visible='NOT-YET'; }         document.getElementById('earlier').textContent='FIRST-UPDATED';</script>         <p id='later'>Later</p>         <script>document.getElementById('later').textContent=visible;</script>",
        800,
        600,
    );
    let visible = |text: &str| {
        page.commands.iter().any(|cmd| {
            matches!(
                cmd, op_paint::PaintCommand::Text {text: value,..} if value.contains(text)
            )
        })
    };
    assert!(visible("FIRST-UPDATED"));
    assert!(
        visible("NOT-YET"),
        "a parser-blocking script must not see future nodes"
    );
    assert!(!visible("WRONG"));
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.mutations),
        (2, 0, 2)
    );
}

#[test]
fn parser_snapshot_refresh_sees_later_elements_and_retains_globals() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        "<div id='box'><script>var partial=document.getElementById('box').textContent;         var suffix='READY';</script>Tail</div>         <p id='later'>Before</p>         <script>document.getElementById('later').textContent=partial+suffix;</script>",
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("READY")
    )));
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("TailREADY")
    )));
    assert_eq!(engine.active_script_report().unwrap().executed, 2);
}
#[test]
fn local_external_script_preserves_mixed_document_order_and_reflow() {
    let root = std::env::temp_dir().join(format!("opbrowser-js-m42-{}-v1", std::process::id(),));
    std::fs::create_dir_all(&root).unwrap();
    let page_path = root.join("index.html");
    let script_path = root.join("stage.js");
    std::fs::write(
        &script_path,
        "var state = state + 'B'; document.getElementById('result').textContent = state;",
    )
    .unwrap();
    std::fs::write(
        &page_path,
        concat!(
            "<!doctype html><p id='result'>Before</p>",
            "<script>var state='A';</script>",
            "<script src='stage.js'></script>",
            "<script>document.getElementById('result').textContent = state+'C';</script>",
        ),
    )
    .unwrap();

    let mut engine = Engine::new();
    let page = engine
        .navigate(&page_path.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (
            report.executed,
            report.failed,
            report.skipped,
            report.mutations
        ),
        (3, 0, 0, 2),
        "{report:?}"
    );
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ABC")
    )));
    assert!(
        engine
            .reflow(300, 400)
            .unwrap()
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(
                cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ABC")
            ))
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn missing_external_script_does_not_abort_following_inline_script() {
    let root = std::env::temp_dir().join(format!(
        "opbrowser-js-m42-missing-{}-v1",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("index.html");
    std::fs::write(
        &path,
        concat!(
            "<p id='result'>Before</p>",
            "<script src='missing.js'></script>",
            "<script>document.getElementById('result').textContent='Recovered';</script>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let page = engine
        .navigate(&path.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.mutations),
        (1, 1, 1)
    );
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Recovered")
    )));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn external_script_resource_type_uses_request_filter_before_loading() {
    let mut network = NetworkContext::default();
    let loaded = network
        .request_filter_mut()
        .import_adblock_rules("||blocked.example^$script");
    assert_eq!(loaded.accepted, 1);
    let error = network
        .load_script_for_page(
            "https://blocked.example/bundle.js",
            "https://blocked.example/page.html",
            1024,
        )
        .unwrap_err();
    assert!(
        matches!(error, LoadError::BlockedRequest { .. }),
        "{error:?}"
    );
}

#[test]
fn cross_origin_script_is_never_fetched() {
    let network = NetworkContext::default();
    let error = network
        .load_script_for_page(
            "https://unrelated.example/evil.js",
            "https://my-site.example/index.html",
            1024,
        )
        .unwrap_err();
    assert!(matches!(error, LoadError::InvalidLink(_)), "{error:?}");
}

#[test]
#[cfg(windows)]
fn winhttp_external_javascript_roundtrip_executes_to_dom() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        for expected in ["/index.html", "/src/app.js"] {
            let began = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(began.elapsed() < Duration::from_secs(8));
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut buf = [0u8; 8192];
            let bytes = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..bytes]);
            assert!(
                request.starts_with(&format!("GET {expected} ")),
                "{request:?}"
            );
            let (mime, body) = if expected == "/index.html" {
                (
                    "text/html",
                    concat!(
                        "<p id='target'>Before</p>",
                        "<script>var text='net-';</script>",
                        "<script src='src/app.js'></script>",
                        "<script>document.getElementById('target').textContent=text+'ok';</script>"
                    ),
                )
            } else {
                ("text/javascript", "text=text+'loaded-';")
            };
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {mime};charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()).unwrap();
            stream.flush().unwrap();
        }
    });
    let mut engine = Engine::new();
    let page = engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("net-loaded-ok")
    )));
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (3, 0, 0),
        "{report:?}"
    );
    server.join().unwrap();
}

#[test]
fn defer_scripts_wait_for_complete_dom_and_execute_in_source_order() {
    let root = std::env::temp_dir().join(format!("opbrowser-js-m47-defer-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        root.join("first.js"),
        "if (document.getElementById('later') == null) { throw 'no later DOM'; } trace=trace+'D';",
    )
    .unwrap();
    std::fs::write(
        root.join("second.js"),
        "trace=trace+'E'; document.getElementById('out').textContent=trace;",
    )
    .unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script>var trace='A';</script>",
            "<script defer src='first.js'></script>",
            "<script>trace=trace+'B';</script>",
            "<script defer src='second.js'></script>",
            "<p id='later'>Present after parse</p>",
            "<script>trace=trace+'C';</script>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (5, 0, 0),
        "{report:?}"
    );
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ABCDE")
    )));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn async_external_script_executes_in_retained_vm() {
    let root = std::env::temp_dir().join(format!("opbrowser-js-m47-async-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        root.join("async.js"),
        "document.getElementById('out').textContent='ASYNC-READY';",
    )
    .unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script async src='async.js'></script>",
            "<p id='later'>Later</p>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (1, 0, 0),
        "{report:?}"
    );
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ASYNC-READY")
    )));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn inline_async_and_defer_flags_do_not_defer_classic_inline_script() {
    let mut engine = Engine::new();
    let rendered = engine.set_html_page(
        "<p id='out'>Before</p><script async defer>var value='SYNC'; \
         if (document.getElementById('later') != null) { value='WRONG'; } \
         document.getElementById('out').textContent=value;</script><p id='later'>After</p>",
        800,
        600,
    );
    assert!(rendered.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("SYNC")
    )));
    assert!(!rendered.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("WRONG")
    )));
    assert_eq!(engine.active_script_report().unwrap().executed, 1);
}

#[test]
#[cfg(windows)]
fn async_scripts_execute_in_download_completion_order_not_tag_order() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..3 {
            let began = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(began.elapsed() < Duration::from_secs(8));
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            workers.push(std::thread::spawn(move || {
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut buffer = [0_u8; 8192];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]);
                let (mime, body) = if request.starts_with("GET /index.html ") {
                    ("text/html", concat!(
                        "<p id='out'>Before</p>",
                        "<script>var finished='';</script>",
                        "<script async src='slow.js'></script>",
                        "<script async src='fast.js'></script>",
                    ))
                } else if request.starts_with("GET /slow.js ") {
                    std::thread::sleep(Duration::from_millis(500));
                    ("text/javascript", "finished=finished+'S';document.getElementById('out').textContent=finished;")
                } else {
                    assert!(request.starts_with("GET /fast.js "), "{request:?}");
                    ("text/javascript", "finished=finished+'F';document.getElementById('out').textContent=finished;")
                };
                write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {mime};charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()).unwrap();
                stream.flush().unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (3, 0, 0),
        "{report:?}"
    );
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("FS")
    )));
    server.join().unwrap();
}

#[test]
fn lifecycle_events_follow_parser_completion_and_change_ready_state() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<p id='output'>Before</p>",
            "<script>",
            "var history='';",
            "document.addEventListener('readystatechange',function(e){",
            "history=history+'R'+document.readyState+'-'+e.eventPhase+';';",
            "});",
            "document.addEventListener('DOMContentLoaded',function(e){",
            "if (e.target == document && e.currentTarget == document && this == document) {",
            "history=history+'D'+document.readyState+';';",
            "}",
            "});",
            "window.addEventListener('load',function(e){",
            "if (this == window && e.currentTarget == window && e.target == document) {",
            "history=history+'L'+document.readyState+';';",
            "document.getElementById('output').textContent=history;",
            "}",
            "});",
            "</script>",
            "<p id='later'>Parsed after listener installation</p>",
        ),
        800,
        600,
    );
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.mutations),
        (1, 0, 1),
        "{report:?}"
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("Rinteractive-2;Dinteractive;Rcomplete-2;Lcomplete;")
    )));
}

#[test]
fn lifecycle_property_handlers_and_listener_removal_work() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<p id='output'>Before</p>",
            "<script>",
            "var history='';",
            "var gone=function(){history=history+'BAD';};",
            "document.addEventListener('DOMContentLoaded',gone);",
            "document.removeEventListener('DOMContentLoaded',gone);",
            "document.onreadystatechange=function(){history=history+'R'+document.readyState+';';};",
            "window.onload=function(){",
            "document.getElementById('output').textContent=history+'ONLOAD';",
            "};",
            "document.readyState='untrusted';",
            "if (document.readyState != 'loading') { throw 'readyState must be read-only'; }",
            "</script>",
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("Rinteractive;Rcomplete;ONLOAD")
    )));
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("BAD")
    )));
    let report = engine.active_script_report().unwrap();
    assert_eq!((report.executed, report.failed), (1, 0), "{report:?}");
}

#[test]
fn deferred_script_registers_dom_content_loaded_before_dispatch() {
    let root =
        std::env::temp_dir().join(format!("opbrowser-js-m48-lifecycle-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        root.join("deferred.js"),
        concat!(
            "if (document.readyState != 'interactive') { throw 'not interactive'; }",
            "document.addEventListener('DOMContentLoaded',function(){",
            "document.getElementById('out').textContent='DEFER-LOADED-'+document.readyState;",
            "});",
        ),
    )
    .unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script defer src='deferred.js'></script>",
            "<p id='later'>After parser</p>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("DEFER-LOADED-interactive")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn timers_fire_only_after_initial_page_load_when_worker_ticks() {
    let mut engine = Engine::new();
    let initial = engine.set_html_page(
        "<p id='output'>Before</p><script>\
         var history='script-';\
         window.addEventListener('load',function(){history=history+'load-';});\
         setTimeout(function(){\
         document.getElementById('output').textContent=history+document.readyState;\
         },0);</script>",
        800,
        600,
    );
    assert!(initial.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Before")
    )));
    assert!(engine.next_timer_wait().is_some());
    let painted = engine.tick_timers(800, 600).expect("timer updated DOM");
    assert!(painted.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("script-load-complete")
    )));
    assert!(engine.next_timer_wait().is_none());
    assert_eq!(engine.active_script_report().unwrap().mutations, 1);
}

#[test]
fn timer_cancellation_and_callback_arguments_preserve_single_vm() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='output'>Before</p><script>\
         var first=setTimeout(function(a,b){\
         clearTimeout(second);\
         document.getElementById('output').textContent=a+b;\
         },0,'YES','-OK');\
         var second=setTimeout(function(){\
         document.getElementById('output').textContent='SHOULD-NOT-RUN';\
         },0);</script>",
        800,
        600,
    );
    let page = engine
        .tick_timers(800, 600)
        .expect("first timer changes text");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("YES-OK")
    )));
    assert!(engine.next_timer_wait().is_none());
    assert!(engine.tick_timers(800, 600).is_none());
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn delayed_timers_and_failed_callbacks_do_not_block_next_task() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='output'>Before</p><script>\
         setTimeout(function(){throw 'error';},0);\
         setTimeout(function(){document.getElementById('output').textContent='RECOVER';},25);\
         </script>",
        800,
        600,
    );
    assert!(engine.tick_timers(800, 600).is_none());
    assert_eq!(engine.active_script_report().unwrap().failed, 1);
    std::thread::sleep(std::time::Duration::from_millis(40));
    let page = engine.tick_timers(800, 600).expect("later timer fired");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("RECOVER")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 1);
}

#[test]
fn replacing_document_discards_timer_callbacks_from_old_page() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='output'>Old</p>\
         <script>setTimeout(function(){\
         document.getElementById('output').textContent='WRONG';\
         },0);</script>",
        800,
        600,
    );
    engine.set_html_page("<p id='output'>New</p>", 800, 600);
    assert!(engine.next_timer_wait().is_none());
    assert!(engine.tick_timers(800, 600).is_none());
    assert!(
        engine
            .reflow(800, 600)
            .unwrap()
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(
                cmd, op_paint::PaintCommand::Text {text,..} if text.contains("New")
            ))
    );
}

#[test]
fn interval_self_cancel_runs_twice_without_orphaned_tasks() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='out'>Waiting</p><script>\
         var ticks=0;var id=setInterval(function(){\
           ticks=ticks+1;\
           document.getElementById('out').textContent='Tick-'+ticks;\
           if(ticks==2){clearInterval(id);}\
         },4);</script>",
        800,
        600,
    );
    assert!(engine.tick_timers(800, 600).is_none());
    std::thread::sleep(std::time::Duration::from_millis(15));
    let page = engine.tick_timers(800, 600).expect("first interval");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Tick-1")
    )));
    std::thread::sleep(std::time::Duration::from_millis(15));
    let page = engine.tick_timers(800, 600).expect("second interval");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Tick-2")
    )));
    assert!(engine.next_timer_wait().is_none());
    assert!(engine.tick_timers(800, 600).is_none());
}

#[test]
fn clear_timeout_also_cancels_interval_and_clear_interval_cancels_timeout() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='out'>Safe</p><script>\
         var id=setInterval(function(){document.getElementById('out').textContent='BAD';},4);\
         clearTimeout(id);\
         var once=setTimeout(function(){document.getElementById('out').textContent='BAD';},0);\
         clearInterval(once);</script>",
        800,
        600,
    );
    assert!(engine.next_timer_wait().is_none());
    std::thread::sleep(std::time::Duration::from_millis(8));
    assert!(engine.tick_timers(800, 600).is_none());
}

#[test]
fn microtask_checkpoint_is_fifo_and_runs_before_timer_macrotask() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        "<p id='out'>Waiting</p><script>\
         var log='S';\
         setTimeout(function(){log=log+'T';document.getElementById('out').textContent=log;},0);\
         queueMicrotask(function(){log=log+'A';queueMicrotask(function(){\
            log=log+'B';document.getElementById('out').textContent=log;});});\
         queueMicrotask(function(){log=log+'C';});\
         </script>",
        800,
        600,
    );
    // FIFO microtasks: A, C, B (B was queued by A).
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("SACB")
    )));
    let next = engine
        .tick_timers(800, 600)
        .expect("timer follows microtasks");
    assert!(next.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("SACBT")
    )));
}

#[test]
fn microtasks_from_timer_run_before_next_due_timer() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='out'>Waiting</p><script>\
         var log='';\
         setTimeout(function(){log=log+'A';queueMicrotask(function(){log=log+'M';});},0);\
         setTimeout(function(){document.getElementById('out').textContent=log+'B';},0);\
         </script>",
        800,
        600,
    );
    let page = engine.tick_timers(800, 600).expect("two timers");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("AMB")
    )));
}

#[test]
fn interval_does_not_survive_navigation() {
    let mut engine = Engine::new();
    engine.set_html_page("<script>setInterval(function(){},4);</script>", 800, 600);
    assert!(engine.next_timer_wait().is_some());
    engine.set_html_page("<p>New page</p>", 800, 600);
    assert!(engine.next_timer_wait().is_none());
}

#[test]
fn post_presentation_text_task_loads_local_resource_and_repaints() {
    let root = std::env::temp_dir().join(format!("opbrowser-m411-file-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(root.join("hello.txt"), "LOCAL-RESOURCE").unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script>opFetchText('hello.txt',function(body,error){",
            "if(error==null){document.getElementById('out').textContent=body;}else{document.getElementById('out').textContent=error;}",
            "});</script>"
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let initial = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("Before")
    )));
    let mut done = false;
    for _ in 0..100 {
        if let Some(result) = engine.tick_timers(800, 600)
            && result.display_list.commands.iter().any(|cmd| {
                matches!(
                    cmd,op_paint::PaintCommand::Text{text,..} if text.contains("LOCAL-RESOURCE")
                )
            })
        {
            done = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(done, "network completion did not repaint page");
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn blocked_cross_origin_text_request_returns_error_without_network() {
    let root = std::env::temp_dir().join(format!("opbrowser-m411-cross-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script>opFetchText('https://example.com/elsewhere.txt',function(body,error){",
            "if(body==null && error!=null){document.getElementById('out').textContent='BLOCKED';}else{document.getElementById('out').textContent='WRONG';}",
            "});</script>"
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let result = engine.tick_timers(800, 600).unwrap();
    assert!(result.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("BLOCKED")
    )));
    assert!(engine.next_timer_wait().is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(windows)]
fn winhttp_text_completion_runs_on_engine_after_first_render() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..2 {
            let start = std::time::Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(start.elapsed().as_secs() < 8);
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            workers.push(std::thread::spawn(move||{
                stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                let mut buf=[0_u8;8192];
                let n=stream.read(&mut buf).unwrap();
                let req=String::from_utf8_lossy(&buf[..n]);
                let (mime,body)=if req.starts_with("GET /index.html ") {
                    ("text/html",concat!(
                        "<p id='out'>INITIAL</p>",
                        "<script>opFetchText('data.json',function(body,error){",
                        "if(error==null){document.getElementById('out').textContent=body;}else{document.getElementById('out').textContent=error;}",
                        "});</script>"
                    ))
                }else{
                    assert!(req.starts_with("GET /data.json "),"unexpected {req:?}");
                    std::thread::sleep(std::time::Duration::from_millis(180));
                    ("application/json","JSON-TEXT")
                };
                write!(stream,
                  "HTTP/1.1 200 OK\r\nContent-Type: {mime}; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                  body.len()).unwrap();
                stream.flush().unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
    let mut engine = Engine::new();
    let initial = engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("INITIAL")
    )));
    assert!(engine.tick_timers(800, 600).is_none());
    let mut finished = false;
    for _ in 0..120 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        if let Some(page) = engine.tick_timers(800, 600)
            && page.display_list.commands.iter().any(|cmd| {
                matches!(
                    cmd,op_paint::PaintCommand::Text{text,..} if text.contains("JSON-TEXT")
                )
            })
        {
            finished = true;
            break;
        }
    }
    assert!(finished, "asynchronous network task did not repaint");
    server.join().unwrap();
}

#[test]
fn late_completion_from_previous_navigation_is_discarded() {
    let root = std::env::temp_dir().join(format!("opbrowser-m411-stale-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let old = root.join("old.html");
    let new = root.join("new.html");
    std::fs::write(root.join("data.txt"), "STALE-BODY").unwrap();
    std::fs::write(
        &old,
        concat!(
            "<p id='out'>Old</p>",
            "<script>opFetchText('data.txt',function(body){",
            "document.getElementById('out').textContent=body;",
            "});</script>"
        ),
    )
    .unwrap();
    std::fs::write(&new, "<p id='out'>NEW-PAGE</p>").unwrap();
    let mut engine = Engine::new();
    engine
        .navigate(&old.display().to_string(), 800, 600)
        .unwrap();
    engine.tick_timers(800, 600);
    let initial = engine
        .navigate(&new.display().to_string(), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("NEW-PAGE")
    )));
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(engine.tick_timers(800, 600).is_none());
    assert!(
        engine
            .reflow(800, 600)
            .unwrap()
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(
                cmd,op_paint::PaintCommand::Text{text,..} if text.contains("NEW-PAGE")
            ))
    );
    assert!(engine.next_timer_wait().is_none());
    std::fs::remove_dir_all(root).unwrap();
}
