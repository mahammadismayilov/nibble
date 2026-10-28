#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod device;
mod ui;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use std::borrow::Cow;

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--probe-output")) {
        let Some(path) = args.next() else { std::process::exit(2); };
        let mut trace = Vec::new();
        let result = device::read_mouse_with_trace(&mut trace);
        let passed = matches!(&result, Ok(data) if data.battery.is_some() && !data.dpi.is_empty() && data.rate.is_some());
        trace.push(format!("[DEBUG-gpui] result: {result:?}"));
        if std::fs::write(path, trace.join("\n")).is_err() { std::process::exit(2); }
        std::process::exit(if passed { 0 } else { 1 });
    }
    Application::new().run(|cx: &mut App| {
        if let Err(error) = cx.text_system().add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/Outfit.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono.ttf").as_slice()),
        ]) {
            eprintln!("Could not load interface fonts: {error}");
        }
        let display = cx.primary_display().map(|display| display.bounds().size);
        let width = display.map(|size| size.width.as_f32().min(1100.0)).unwrap_or(1100.0);
        let height = display.map(|size| size.height.as_f32().min(750.0)).unwrap_or(750.0);
        let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(900.0), px(650.0))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Nibble — Mouse Configuration".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(ui::Nibble::new),
        )
        .expect("Could not open Nibble prototype window");
        cx.activate(true);
    });
}

