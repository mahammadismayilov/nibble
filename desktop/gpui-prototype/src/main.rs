#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod device;
mod profile;
mod ui;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use std::borrow::Cow;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let demo = args.iter().any(|arg| arg == "--demo");
    let experimental_write = args.iter().any(|arg| arg == "--experimental-write") && !demo;
    Application::new().run(move |cx: &mut App| {
        if let Err(error) = cx.text_system().add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/Outfit.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono.ttf").as_slice()),
        ]) {
            eprintln!("Could not load interface fonts: {error}");
        }
        let display = cx.primary_display().map(|display| display.bounds().size);
        let width = display.map(|size| size.width.min(px(1100.0))).unwrap_or(px(1100.0));
        // Leave space for the native title bar and the Windows taskbar on small displays.
        let height = display.map(|size| (size.height - px(96.0)).max(px(650.0)).min(px(750.0))).unwrap_or(px(750.0));
        let bounds = Bounds::centered(None, size(width, height), cx);
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
            move |_, cx| cx.new(|cx| ui::Nibble::new(cx, demo, experimental_write)),
        )
        .expect("Could not open Nibble prototype window");
        cx.activate(true);
    });
}

