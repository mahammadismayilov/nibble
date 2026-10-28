#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gpui::{
    App, AppContext, Application, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, px, rgb, size,
};
use hidapi::HidApi;

struct Prototype {
    status: String,
}

impl Prototype {
    fn new(cx: &mut Context<Self>) -> Self {
        let work = cx.background_executor().spawn(async { read_mouse_status() });
        cx.spawn(async move |view, cx| {
            let status = work.await;
            let _ = view.update(cx, |view, cx| {
                view.status = status;
                cx.notify();
            });
        })
        .detach();
        Self { status: "Checking receiver...".into() }
    }
}

impl Render for Prototype {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_8()
            .bg(rgb(0x101116))
            .text_color(rgb(0xf1f2f6))
            .child(div().text_xl().child("Nibble · GPUI prototype"))
            .child(div().text_color(rgb(0x9da6b5)).child("AJAZZ AJ179P · status check"))
            .child(div().rounded_md().p_4().bg(rgb(0x20232d)).child(self.status.clone()))
            .child(div().text_color(rgb(0x9da6b5)).child("Read-only experiment. No mouse settings are changed."))
    }
}

// This uses the same 33-byte status query as the existing CompX driver.
fn read_mouse_status() -> String {
    let api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => return format!("Could not initialize HID: {e}"),
    };
    let info = api.device_list().find(|d| {
        d.vendor_id() == 0x249a && d.product_id() == 0x5c2f
            && ((d.usage_page() == 1 && d.usage() == 0) || d.usage_page() >= 0xff00)
    });
    let Some(info) = info else {
        return "AJ179P configuration interface not found.".into();
    };
    let device = match info.open_device(&api) {
        Ok(device) => device,
        Err(e) => return format!("Receiver found, but could not open it: {e}"),
    };
    let mut query = [0u8; 33];
    query[1] = 0x10;
    let mut reply = [0u8; 65];
    for _ in 0..5 {
        if let Err(e) = device.write(&query) {
            return format!("Receiver found, but status query failed: {e}");
        }
        match device.read_timeout(&mut reply, 300) {
            Ok(n) if n > 0 => {
                if let Some(status) = parse_status(&reply[..n]) {
                    return status;
                }
            }
            Ok(_) => {}
            Err(e) => return format!("Receiver found, but reading status failed: {e}"),
        }
    }
    "Receiver found; no recognizable battery response. Close other mouse software and retry.".into()
}

fn parse_status(bytes: &[u8]) -> Option<String> {
    for offset in 0..bytes.len().min(3) {
        if bytes[offset] == 0xc0 && bytes.len() > offset + 2 {
            if bytes[offset + 1] == 0 {
                return Some("Receiver connected · mouse sleeping".into());
            }
            return battery_text(bytes[offset + 2]);
        }
        if bytes[offset] == 0x10 && bytes.len() > offset + 13 {
            return battery_text(bytes[offset + 13]);
        }
    }
    None
}

fn battery_text(raw: u8) -> Option<String> {
    match raw {
        0 => None,
        255 => Some("Receiver connected · charging, battery level unavailable".into()),
        1..=100 => Some(format!("Receiver connected · battery {raw}%")),
        _ => Some(format!("Receiver connected · battery {}%", ((raw as f32 / 255.0) * 100.0).round() as u8)),
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(560.0), px(300.0)), cx);
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            |_, cx| cx.new(Prototype::new),
        ).expect("Could not open prototype window");
        cx.activate(true);
    });
}

