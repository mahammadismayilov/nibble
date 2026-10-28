#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod device;
mod ui;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use std::borrow::Cow;

fn main() {
    Application::new().run(|cx: &mut App| {
        if let Err(error) = cx.text_system().add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/Outfit.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono.ttf").as_slice()),
        ]) {
            eprintln!("Could not load interface fonts: {error}");
        }
        let bounds = Bounds::centered(None, size(px(1100.0), px(750.0)), cx);
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

