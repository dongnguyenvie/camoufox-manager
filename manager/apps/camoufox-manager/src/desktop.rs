//! App window: the server runs in-process on 127.0.0.1 (random port unless `CM_PORT` is set),
//! the system webview (WKWebView) opens it. Closing the window stops profiles, then exits.

use crate::{build_manager, env_port};
use anyhow::Result;
use cm_service::Manager;
use std::{sync::Arc, time::Duration};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

pub fn run() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    // Start the server first; on a config error show an error page instead of quitting silently.
    let started: Result<(Arc<Manager>, u16)> = rt.block_on(async {
        let app = build_manager()?;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", env_port("CM_PORT")?.unwrap_or(0))).await?;
        let port = listener.local_addr()?.port();
        app.clone().spawn_scheduler(); // phase 10: scheduled runs
        cm_api::sync::start(app.clone()).await; // phase 16: device sync listener
        let r = cm_api::router(app.clone());
        tokio::spawn(async move {
            let _ = axum::serve(listener, r).await;
        });
        Ok((app, port))
    });

    let event_loop = EventLoopBuilder::new().build();
    let _menu = install_menu();
    let window = WindowBuilder::new()
        .with_title("Camoufox Manager")
        .with_inner_size(LogicalSize::new(1440.0, 860.0))
        .with_min_inner_size(LogicalSize::new(720.0, 480.0))
        .build(&event_loop)?;

    let builder = WebViewBuilder::new().with_devtools(cfg!(debug_assertions));
    let (app, builder) = match started {
        Ok((app, port)) => (Some(app), builder.with_url(format!("http://127.0.0.1:{port}/"))),
        Err(e) => (None, builder.with_html(error_page(&format!("{e:#}")))),
    };
    let _webview = builder.build(&window)?;

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            if let Some(app) = &app {
                rt.block_on(async {
                    let _ = tokio::time::timeout(Duration::from_secs(15), app.stop_all()).await;
                });
            }
            *flow = ControlFlow::Exit;
        }
    });
}

/// Standard macOS menu: without an Edit menu Cmd+C/V/A do nothing in the webview.
fn install_menu() -> muda::Menu {
    use muda::{Menu, PredefinedMenuItem as P, Submenu};
    let menu = Menu::new();
    let app_menu = Submenu::with_items(
        "Camoufox Manager",
        true,
        &[&P::about(None, None), &P::separator(), &P::hide(None), &P::hide_others(None), &P::separator(), &P::quit(None)],
    )
    .expect("menu");
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[&P::undo(None), &P::redo(None), &P::separator(), &P::cut(None), &P::copy(None), &P::paste(None), &P::select_all(None)],
    )
    .expect("menu");
    let window = Submenu::with_items("Window", true, &[&P::minimize(None), &P::fullscreen(None), &P::close_window(None)]).expect("menu");
    menu.append_items(&[&app_menu, &edit, &window]).expect("menu");
    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();
    menu
}

fn error_page(msg: &str) -> String {
    let esc = msg.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<!doctype html><meta charset="utf-8"><body style="font:15px -apple-system,sans-serif;padding:48px;color:#222;background:#fafafa">
<h2 style="margin:0 0 12px">Camoufox Manager could not start</h2>
<pre style="white-space:pre-wrap;background:#fff;border:1px solid #ddd;border-radius:8px;padding:16px">{esc}</pre>
<p>Fix it, then reopen the app.</p></body>"#
    )
}
