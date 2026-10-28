#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod device;
mod ui;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use std::borrow::Cow;

fn main() {
    let demo = std::env::args().any(|arg| arg == "--demo");
    Application::new().run(move |cx: &mut App| {
        if let Err(error) = cx.text_system().add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/Outfit.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono.ttf").as_slice()),
        ]) {
            eprintln!("Could not load interface fonts: {error}");
        }
        let display = cx.primary_display().map(|display| display.bounds().size);
        let width = display.map(|size| size.width.min(px(1100.0))).unwrap_or(px(1100.0));
        let height = display.map(|size| size.height.min(px(750.0))).unwrap_or(px(750.0));
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
            move |_, cx| cx.new(|cx| ui::Nibble::new(cx, demo)),
        )
        .expect("Could not open Nibble prototype window");
        cx.activate(true);
    });
}

