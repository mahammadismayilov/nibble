use crate::device::{self, LightMode, Lighting, Readout, Sensor, SensorChange};
use gpui::{
    Context, Div, FocusHandle, FontWeight, Image, ImageFormat, IntoElement, KeyDownEvent, ObjectFit, Render, StyledImage,
    Window, div, img, prelude::*, px, relative, rgb,
};
use std::sync::Arc;
use std::path::PathBuf;

const TABS: [&str; 7] = ["Home", "Keys", "DPI", "Lighting", "Macro", "Settings", "About"];
const STAGE_COLORS: [u32; 6] = [0xff0000, 0x00ff00, 0x0000ff, 0x00ffff, 0xffff00, 0x800080];

#[derive(Clone, Copy)]
struct Palette {
    bg: u32,
    card: u32,
    inset: u32,
    hover: u32,
    text: u32,
    muted: u32,
    accent: u32,
    accent_soft: u32,
    success: u32,
}

impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self { bg: 0x0c0b10, card: 0x19181d, inset: 0x111116, hover: 0x28272d, text: 0xf5f4f8, muted: 0xa9a7b2, accent: 0x739cff, accent_soft: 0x222e43, success: 0x34ce9b }
        } else {
            Self { bg: 0xf3f3f4, card: 0xffffff, inset: 0xe9e9ea, hover: 0xe4e5e8, text: 0x19191c, muted: 0x777780, accent: 0x3968d5, accent_soft: 0xdce4f6, success: 0x008f6b }
        }
    }
}

pub struct Nibble {
    screen: usize,
    dark: bool,
    demo: bool,
    experimental_write: bool,
    scanning: bool,
    applying: bool,
    readout: Option<Readout>,
    error: Option<String>,
    message: Option<String>,
    dpi_focus: FocusHandle,
    dpi_input: Option<String>,
    dpi_replace_input: bool,
    selected_key: usize,
    mouse: Arc<Image>,
    logo: Arc<Image>,
}

impl Nibble {
    pub fn new(cx: &mut Context<Self>, demo: bool, experimental_write: bool) -> Self {
        if !demo { Self::scan(cx); }
        let readout = demo.then(|| Readout {
            battery: Some(80),
            dpi: vec![400, 800, 1200, 1600, 2400, 10000],
            active_dpi: Some(5),
            dpi_count: Some(6),
            rate: Some(1000),
            lighting: Some(Lighting { mode: LightMode::Breathing, brightness: 0, speed: 6 }),
            sensor: Some(Sensor { lod: 1, angle_snap: false, ripple: false }),
            keys: Some([[0x10, 1, 0], [0x10, 2, 0], [0x10, 4, 0], [0x10, 8, 0], [0x10, 16, 0], [0x40, 1, 0]]),
            vid: 0x249a,
            pid: 0x5c2f,
            ..Default::default()
        });
        Self {
            screen: 0,
            dark: demo || load_theme(),
            demo,
            experimental_write,
            scanning: !demo,
            applying: false,
            readout,
            error: None,
            message: None,
            dpi_focus: cx.focus_handle(),
            dpi_input: None,
            dpi_replace_input: false,
            selected_key: 0,
            mouse: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../../../webapp/assets/device/mouse_aj179.png").to_vec(),
            )),
            logo: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../../src-tauri/icons/128x128.png").to_vec(),
            )),
        }
    }

    fn scan(cx: &mut Context<Self>) {
        let work = cx.background_executor().spawn(async { device::read_mouse() });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.scanning = false;
                match result {
                    Ok(data) => { view.readout = Some(data); view.error = None; }
                    Err(error) => { view.readout = None; view.error = Some(error); }
                }
                cx.notify();
            });
        }).detach();
    }

    fn set_theme(&mut self, dark: bool, cx: &mut Context<Self>) {
        self.dark = dark;
        if !self.demo {
            if let Some(path) = theme_path() {
                let saved = path.parent().is_some_and(|parent| std::fs::create_dir_all(parent).is_ok())
                    && std::fs::write(path, if dark { "dark" } else { "light" }).is_ok();
                if !saved { self.message = Some("Could not save appearance preference.".into()); }
            }
        }
        cx.notify();
    }

    fn select_dpi(&mut self, index: usize, cx: &mut Context<Self>) {
        self.dpi_input = None;
        self.update_dpi(index, false, cx);
    }

    fn select_dpi_count(&mut self, count: usize, cx: &mut Context<Self>) {
        self.dpi_input = None;
        self.update_dpi(count, true, cx);
    }

    fn edit_dpi(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        let Some(value) = self.readout.as_ref().and_then(|data| data.active_dpi.and_then(|i| data.dpi.get(i))) else { return; };
        self.dpi_input = Some(value.to_string());
        self.dpi_replace_input = true;
        window.focus(&self.dpi_focus);
        cx.notify();
    }

    fn handle_dpi_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(input) = self.dpi_input.as_mut() else { return; };
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.control && key.eq_ignore_ascii_case("a") {
            self.dpi_replace_input = true;
        } else if key == "escape" {
            self.dpi_input = None;
        } else if key == "enter" || key == "return" {
            let value = input.parse::<u32>().ok().map(|value| ((value.min(26000) + 50) / 100 * 100).clamp(50, 26000) as u16);
            self.dpi_input = None;
            if let Some(value) = value { self.set_dpi_value(value, cx); }
            else { self.message = Some("Enter a DPI value between 50 and 26000.".into()); }
        } else if key == "backspace" {
            if self.dpi_replace_input { input.clear(); self.dpi_replace_input = false; } else { input.pop(); }
        } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
            let typed = event.keystroke.key_char.as_deref().unwrap_or(key);
            if typed.len() == 1 && typed.as_bytes()[0].is_ascii_digit() {
                if self.dpi_replace_input { input.clear(); self.dpi_replace_input = false; }
                if input.len() < 5 { input.push_str(typed); }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn set_dpi_value(&mut self, value: u16, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        let Some(index) = self.readout.as_ref().and_then(|data| data.active_dpi) else { return; };
        if self.demo {
            if let Some(data) = &mut self.readout { if let Some(stage) = data.dpi.get_mut(index) { *stage = value; } }
            cx.notify();
            return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Applying DPI value…".into());
        let work = cx.background_executor().spawn(async move { device::set_dpi_value(&snapshot, index, value) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok((active, count, stages)) => {
                        if let Some(data) = &mut view.readout { data.active_dpi = Some(active); data.dpi_count = Some(count); data.dpi = stages; }
                        view.message = Some("DPI value confirmed by receiver.".into());
                    }
                    Err(error) => {
                        if let Some(data) = &mut view.readout { data.active_dpi = None; data.dpi_count = None; data.dpi.clear(); }
                        view.message = Some(error);
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn update_dpi(&mut self, value: usize, is_count: bool, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        if self.readout.as_ref().is_some_and(|data| {
            if is_count { data.dpi_count == Some(value) } else { data.active_dpi == Some(value) }
        }) { return; }
        if self.demo {
            if let Some(data) = &mut self.readout {
                if is_count {
                    data.dpi_count = Some(value);
                    data.active_dpi = data.active_dpi.map(|index| index.min(value - 1));
                } else {
                    data.active_dpi = Some(value);
                }
            }
            cx.notify();
            return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Applying DPI settings…".into());
        let work = cx.background_executor().spawn(async move {
            if is_count { device::set_dpi_count(&snapshot, value) }
            else { device::set_active_dpi(&snapshot, value) }
        });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok((active, count, stages)) => {
                        if let Some(data) = &mut view.readout {
                            data.active_dpi = Some(active);
                            data.dpi_count = Some(count);
                            data.dpi = stages;
                        }
                        view.message = Some("DPI settings confirmed by receiver.".into());
                    }
                    Err(error) => {
                        if let Some(data) = &mut view.readout {
                            data.active_dpi = None;
                            data.dpi_count = None;
                            data.dpi.clear();
                        }
                        view.message = Some(error);
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn select_rate(&mut self, rate: u16, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        if self.readout.as_ref().and_then(|data| data.rate) == Some(rate) { return; }
        if self.demo {
            if let Some(data) = &mut self.readout { data.rate = Some(rate); }
            cx.notify();
            return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Applying report rate…".into());
        let work = cx.background_executor().spawn(async move { device::set_rate(&snapshot, rate) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok(rate) => {
                        if let Some(data) = &mut view.readout { data.rate = Some(rate); }
                        view.message = Some("Report rate confirmed by receiver.".into());
                    }
                    Err(error) => {
                        if let Some(data) = &mut view.readout { data.rate = None; }
                        view.message = Some(error);
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn update_sensor(&mut self, change: SensorChange, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        if self.demo {
            if let Some(sensor) = self.readout.as_mut().and_then(|data| data.sensor.as_mut()) {
                match change {
                    SensorChange::Lod(value) => sensor.lod = value,
                    SensorChange::AngleSnap(value) => sensor.angle_snap = value,
                    SensorChange::Ripple(value) => sensor.ripple = value,
                }
            }
            cx.notify();
            return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Applying sensor setting…".into());
        let work = cx.background_executor().spawn(async move { device::set_sensor(&snapshot, change) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok(sensor) => {
                        if let Some(data) = &mut view.readout { data.sensor = Some(sensor); }
                        view.message = Some("Sensor setting confirmed by receiver.".into());
                    }
                    Err(error) => {
                        if let Some(data) = &mut view.readout { data.sensor = None; }
                        view.message = Some(error);
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn assign_key(&mut self, label: &'static str, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        let index = self.selected_key;
        if index == 0 {
            self.message = Some("Left-click is locked to prevent losing mouse control.".into());
            cx.notify();
            return;
        }
        let Some(wire) = device::key_wire(label) else { return; };
        if self.demo {
            if let Some(keys) = self.readout.as_mut().and_then(|data| data.keys.as_mut()) { keys[index] = wire; }
            cx.notify();
            return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Applying button assignment…".into());
        let work = cx.background_executor().spawn(async move { device::set_key(&snapshot, index, label) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok(keys) => {
                        if let Some(data) = &mut view.readout { data.keys = Some(keys); }
                        view.message = Some("Button assignment confirmed by receiver.".into());
                    }
                    Err(error) => {
                        if let Some(data) = &mut view.readout { data.keys = None; }
                        view.message = Some(error);
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn header(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let status = if self.demo {
            "Sample data · No receiver access".to_string()
        } else if self.scanning {
            "Checking receiver".to_string()
        } else if let Some(data) = &self.readout {
            format!("AJAZZ AJ179 / AJ179P · {:04X}:{:04X} · Last read", data.vid, data.pid)
        } else {
            "Disconnected · Connect mouse receiver to configure".into()
        };
        let battery = self.readout.as_ref().and_then(|data| data.battery).map(|value| format!("{value}%")).unwrap_or_else(|| "—".into());
        let mut tabs = div().flex().items_center().gap(px(4.0)).h(px(50.0));
        for (index, label) in TABS.iter().enumerate() {
            let selected = self.screen == index;
            tabs = tabs.child(
                div().id(("tab", index)).cursor_pointer().rounded(px(8.0))
                    .h(px(44.0)).px(px(15.0)).flex().items_center()
                    .bg(rgb(if selected { p.accent_soft } else { p.bg }))
                    .text_color(rgb(if selected { p.accent } else { p.muted }))
                    .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                    .child(*label)
                    .on_click(cx.listener(move |this, _, _, cx| { this.screen = index; cx.notify(); })),
            );
        }
        div().flex().flex_col().child(
            div().flex().items_center().gap(px(14.0)).h(px(55.0))
                .child(div().flex().items_center().gap(px(5.0)).font_weight(FontWeight::BOLD)
                    .child("Nibble")
                    .child(div().rounded(px(4.0)).bg(rgb(p.accent_soft)).px(px(5.0)).text_size(px(11.0)).text_color(rgb(p.accent)).child("DESKTOP"))
                    .child(div().font_weight(FontWeight::NORMAL).text_size(px(11.0)).text_color(rgb(p.muted)).child("v1.0.6")))
                .child(div().ml(px(8.0)).font_weight(FontWeight::SEMIBOLD).child("AJAZZ AJ179 / AJ179P  ⌄"))
                .child(div().flex_1().min_w(px(0.0)).text_size(px(12.0)).text_color(rgb(if self.readout.is_some() { p.success } else { p.muted })).child(format!("●  {status}")))
                .child(div().id("refresh").cursor_pointer().rounded(px(8.0)).px(px(12.0)).py(px(8.0)).bg(rgb(p.accent_soft)).text_color(rgb(p.accent))
                    .child(if self.demo { "Demo" } else if self.scanning { "Checking…" } else { "Refresh" })
                    .on_click(cx.listener(|this, _, _, cx| { if !this.demo && !this.scanning && !this.applying { this.scanning = true; this.message = None; Self::scan(cx); cx.notify(); } })))
                .child(div().text_color(rgb(p.muted)).text_size(px(12.0)).child(format!("▰  {battery}")))
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Default Profile  ⌄"))
                .child(div().id("theme").cursor_pointer().rounded(px(8.0)).px(px(9.0)).py(px(7.0)).text_size(px(18.0))
                    .child(if self.dark { "☼" } else { "☾" })
                    .on_click(cx.listener(|this, _, _, cx| { this.set_theme(!this.dark, cx); })))
        ).child(tabs)
    }

    fn home(&self, p: Palette) -> Div {
        let data = self.readout.as_ref();
        let dpi = data.and_then(|d| d.active_dpi.and_then(|i| d.dpi.get(i))).map(u16::to_string).unwrap_or_else(|| "—".into());
        let stage = data.and_then(|d| d.active_dpi).map(|i| format!("Stage {}", i + 1)).unwrap_or_else(|| "Not available".into());
        let rate = data.and_then(|d| d.rate).map(|value| value.to_string()).unwrap_or_else(|| "—".into());
        let battery = data.and_then(|d| d.battery).map(|value| format!("{value}%")).unwrap_or_else(|| "—".into());
        let battery_sub = match data { Some(d) if d.sleeping => "Sleeping", Some(d) if d.charging => "Charging", Some(d) if d.battery.is_some() => "Last read", _ => "Unavailable" };
        let lighting = data.and_then(|d| d.lighting.as_ref());
        let light_name = lighting.map(|light| match light.mode { LightMode::Off => "Close", LightMode::Breathing => "Breathing", LightMode::Constant => "Constant light" }).unwrap_or("—");
        let light_detail = lighting.map(|light| if light.mode == LightMode::Off { "Off".into() } else { format!("{}% brightness", light.brightness) }).unwrap_or_else(|| "Unavailable".into());
        let hero = div().flex_1().h(px(560.0)).flex().flex_col().items_center().justify_center()
            .child(img(self.mouse.clone()).w(px(280.0)).h(px(500.0)).object_fit(ObjectFit::Contain))
            .child(div().mt(px(18.0)).text_size(px(24.0)).font_weight(FontWeight::SEMIBOLD).child("AJAZZ AJ179 / AJ179P"))
            .child(div().mt(px(6.0)).text_size(px(14.0)).text_color(rgb(p.muted)).child("PAW3395 sensor"));
        let stats = div().flex_1().h(px(560.0)).flex().flex_col()
            .child(div().flex().h(px(185.0)).child(stat(p, "DPI", dpi, stage, true)).child(stat(p, "POLLING RATE", rate, "Hz".into(), true)))
            .child(div().h(px(185.0)).child(stat(p, "LIGHTING", light_name.into(), light_detail, false)))
            .child(div().flex().h(px(185.0)).child(stat(p, "CONNECTION", "USB".into(), if data.is_some() { "Receiver found".into() } else { "Not connected".into() }, false))
                .child(stat(p, "BATTERY", battery, battery_sub.into(), false)));
        div().flex().w_full().gap(px(12.0)).child(hero).child(stats)
    }

    fn dpi(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let data = self.readout.as_ref();
        let stage_count = data.and_then(|d| d.dpi_count).unwrap_or(6);
        let dpi_editable = (self.demo || self.experimental_write) && data.is_some_and(|d| d.dpi_count.is_some() && d.dpi.len() == 6);
        let rate_editable = (self.demo || self.experimental_write) && data.and_then(|d| d.rate).is_some();
        let mut stage_rows = div().flex().flex_col().gap(px(8.0));
        for pair in [[Some(0), Some(1)], [Some(2), Some(3)], [Some(4), None], [Some(5), None]] {
            let mut row = div().flex().gap(px(8.0));
            for i in pair.into_iter().flatten() {
                if i >= stage_count { continue; }
                let value = data.and_then(|d| d.dpi.get(i)).map(u16::to_string).unwrap_or_else(|| "—".into());
                let active = data.and_then(|d| d.active_dpi) == Some(i);
                let mut stage = div().id(("dpi-stage", i)).rounded(px(8.0)).bg(rgb(if active { p.accent_soft } else { p.inset })).px(px(10.0)).py(px(6.0)).flex().items_center().gap(px(8.0))
                    .child(div().w(px(28.0)).h(px(28.0)).rounded(px(6.0)).bg(rgb(if self.demo { STAGE_COLORS[i] } else { p.muted })))
                    .child(div().text_size(px(12.0)).font_weight(FontWeight::SEMIBOLD).child(format!("Stage {}", i + 1)))
                    .child(div().font_family("JetBrains Mono").text_size(px(12.0)).text_color(rgb(if active { p.accent } else { p.muted })).child(format!("{value} DPI")));
                if active {
                    stage = stage.child(div().rounded_full().bg(rgb(p.accent_soft)).px(px(7.0)).py(px(3.0)).text_size(px(10.0)).text_color(rgb(p.accent)).child("ACTIVE"));
                }
                if dpi_editable {
                    stage = stage.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.select_dpi(i, cx)));
                }
                row = row.child(stage);
            }
            stage_rows = stage_rows.child(row);
        }
        let mut track = div().relative().w_full().h(px(66.0))
            .child(div().absolute().top(px(41.0)).w_full().h(px(2.0)).bg(rgb(p.muted)));
        if let Some(data) = data {
            for (i, value) in data.dpi.iter().enumerate().take(stage_count) {
                let position = dpi_position(*value);
                let active = data.active_dpi == Some(i);
                let color = if self.demo { STAGE_COLORS[i] } else { p.accent };
                let mut number = div().id(("dpi-number", i)).absolute().left(relative(position)).top(px(0.0)).ml(px(-20.0))
                    .text_size(px(18.0)).font_weight(FontWeight::BOLD)
                    .text_color(rgb(if active { color } else { p.text })).child(value.to_string());
                let mut dot = div().id(("dpi-node", i)).absolute().left(relative(position)).top(px(36.0)).ml(px(-6.0)).size(px(12.0))
                    .rounded_full().bg(rgb(if active { color } else { p.text }));
                if dpi_editable {
                    number = number.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.select_dpi(i, cx)));
                    dot = dot.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.select_dpi(i, cx)));
                }
                track = track.child(number).child(dot);
            }
        }
        let mut scale = div().relative().w_full().h(px(25.0)).text_size(px(12.0)).text_color(rgb(p.muted));
        for value in [200u16, 1000, 2000, 3000, 4000, 6000, 8000, 26000] {
            scale = scale.child(div().absolute().left(relative(dpi_position(value))).ml(px(-12.0)).child(value.to_string()));
        }
        let chart = div().mt(px(55.0)).h(px(160.0)).px(px(12.0)).flex().flex_col().justify_center()
            .child(track).child(scale);
        let mut level_buttons = div().flex().rounded(px(8.0)).bg(rgb(p.inset)).p(px(4.0));
        for level in 1..=6 {
            let selected = data.and_then(|d| d.dpi_count) == Some(level);
            let mut button = div().id(("dpi-count", level)).w(px(31.0)).h(px(32.0)).rounded(px(5.0)).flex().items_center().justify_center()
                .bg(rgb(if selected { p.hover } else { p.inset }))
                .text_color(rgb(if selected { p.accent } else { p.muted })).child(level.to_string());
            if dpi_editable {
                button = button.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.select_dpi_count(level, cx)));
            }
            level_buttons = level_buttons.child(button);
        }
        let levels = div().flex().items_center().gap(px(8.0)).text_size(px(12.0)).text_color(rgb(p.muted)).child("DPI Levels:").child(level_buttons);
        let selected_value = self.dpi_input.clone().or_else(|| data.and_then(|d| d.active_dpi.and_then(|i| d.dpi.get(i)))
            .map(u16::to_string)).unwrap_or_else(|| "—".into());
        let selected_color = data.and_then(|d| d.active_dpi).filter(|_| self.demo)
            .and_then(|i| STAGE_COLORS.get(i)).copied().unwrap_or(p.muted);
        let mut dpi_input = div().id("dpi-value").track_focus(&self.dpi_focus).w(px(90.0)).rounded(px(8.0))
            .border_1().border_color(rgb(if self.dpi_input.is_some() { p.accent } else { p.inset })).bg(rgb(p.inset)).px(px(10.0)).py(px(10.0))
            .font_family("JetBrains Mono").text_color(rgb(p.text)).child(selected_value);
        if dpi_editable {
            dpi_input = dpi_input.cursor_text()
                .on_click(cx.listener(|this, _, window, cx| this.edit_dpi(window, cx)))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| this.handle_dpi_key(event, cx)));
        }
        let fine_controls = div().flex().items_center().gap(px(12.0)).text_size(px(12.0)).text_color(rgb(p.muted))
            .child("DPI Value")
            .child(dpi_input)
            .child("LED Color")
            .child(div().size(px(26.0)).rounded(px(4.0)).bg(rgb(selected_color)));
        let hero = card(p).flex_1().h(px(520.0)).px(px(36.0)).py(px(28.0)).flex().flex_col()
            .child(div().flex().items_center().justify_between()
                .child(div().flex().items_center().gap(px(20.0)).child(heading("DPI Sensitivity")).child(levels))
                .child(muted(p, "Reset DPI")))
            .child(chart)
            .child(div().mt(px(25.0)).flex().items_center().justify_between().gap(px(20.0))
                .child(stage_rows)
                .child(fine_controls));
        let mut rate_options = div().flex().flex_col().gap(px(8.0)).mt(px(14.0));
        for pair in [[125, 250], [500, 1000]] {
            let mut row = div().flex().gap(px(8.0));
            for rate in pair {
                let active = data.and_then(|d| d.rate) == Some(rate);
                let mut button = div().id(("rate", usize::from(rate))).flex_1().rounded(px(10.0)).h(px(80.0)).flex().flex_col().justify_center().items_center().gap(px(4.0))
                    .bg(rgb(if active { p.accent_soft } else { p.inset })).text_color(rgb(if active { p.accent } else { p.text }))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(format!("{rate} Hz")))
                    .child(div().text_size(px(11.0)).text_color(rgb(p.muted)).child(if rate == 1000 { "Default" } else { "Report rate" }));
                if rate_editable {
                    button = button.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.select_rate(rate, cx)));
                }
                row = row.child(button);
            }
            rate_options = rate_options.child(row);
        }
        let rates = card(p).w(px(280.0)).h(px(520.0)).child(heading("Report Rate"))
            .child(muted(p, "How often the mouse reports position to the PC"))
            .child(rate_options);
        div().flex().w_full().gap(px(16.0)).child(hero).child(rates)
    }

    fn keys(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let labels = ["Left Button", "Right Button", "Middle / Wheel", "Backward", "Forward", "DPI Button"];
        let positions = [(35.0, 140.0), (286.0, 140.0), (178.0, 50.0), (0.0, 265.0), (10.0, 206.0), (265.0, 265.0)];
        let keys = self.readout.as_ref().and_then(|data| data.keys.as_ref());
        let mut mouse = div().relative().flex_1().flex().justify_center().items_center()
            .child(img(self.mouse.clone()).w(px(280.0)).h(px(500.0)).object_fit(ObjectFit::Contain));
        for (index, (x, y)) in positions.into_iter().enumerate() {
            let function = keys.map(|keys| device::key_label(keys[index])).unwrap_or("—");
            let selected = self.selected_key == index;
            mouse = mouse.child(div().id(("key-callout", index)).absolute().left(px(x)).top(px(y))
                .cursor_pointer().rounded(px(8.0)).bg(rgb(if selected { p.accent_soft } else { p.card }))
                .text_color(rgb(if selected { p.accent } else { p.text })).px(px(12.0)).py(px(8.0)).text_size(px(12.0))
                .child(format!("{} · {function}", labels[index]))
                .on_click(cx.listener(move |this, _, _, cx| { this.selected_key = index; cx.notify(); })));
        }
        let left = div().flex_1().h(px(620.0)).flex().flex_col()
            .child(div().flex().justify_between().child(div().child(heading("Mouse Buttons")).child(muted(p, "Click a callout to rebind. Grayed options are not available yet."))).child(muted(p, "Reset all keys")))
            .child(mouse);
        let groups: [(&str, &[&str]); 4] = [
            ("MOUSE FEATURES", &["Left-Click", "Right-Click", "Middle-Click", "Forward", "Backward", "Double-Click", "Scroll Up", "Scroll Down", "DPI Loop", "Fire button", "Disable Key"]),
            ("MULTIMEDIA", &["Volume +", "Volume -", "Mute", "Play/Pause", "Previous", "Next", "Stop"]),
            ("SYSTEM SHORTCUT", &["Calculator", "My Computer", "Mail", "Refresh (F5)", "Switch Application", "Copy", "Paste", "Screen Brightness +", "Screen Brightness -"]),
            ("LIGHTING / MACRO", &["Lighting on/off", "Macro"]),
        ];
        let mut functions = div().id("key-functions").flex_1().min_h(px(0.0)).overflow_y_scroll().flex().flex_col().gap(px(14.0));
        for (title, items) in groups {
            let mut group = div().flex().flex_col().gap(px(6.0)).child(muted(p, title));
            for pair in items.chunks(2) {
                let mut row = div().flex().gap(px(6.0));
                for label in pair {
                    let label = *label;
                    let enabled = (self.demo || self.experimental_write) && keys.is_some() && self.selected_key != 0 && device::key_wire(label).is_some();
                    let selected = keys.and_then(|keys| device::key_wire(label).map(|wire| keys[self.selected_key] == wire)).unwrap_or(false);
                    let mut button = div().id(("key-function", label)).flex_1().rounded(px(8.0)).px(px(12.0)).py(px(11.0))
                        .bg(rgb(if selected { p.accent_soft } else { p.inset }))
                        .text_color(rgb(if selected { p.accent } else if enabled { p.text } else { p.muted })).child(label);
                    if enabled { button = button.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.assign_key(label, cx))); }
                    row = row.child(button);
                }
                group = group.child(row);
            }
            functions = functions.child(group);
        }
        let right = card(p).flex_1().h(px(620.0)).flex().flex_col().gap(px(10.0))
            .child(heading("Button Function"))
            .child(div().rounded(px(8.0)).p(px(12.0)).bg(rgb(p.accent_soft)).text_color(rgb(p.accent))
                .child(format!("{} → {}", labels[self.selected_key], keys.map(|keys| device::key_label(keys[self.selected_key])).unwrap_or("Unavailable"))))
            .child(functions);
        div().flex().gap(px(16.0)).child(left).child(right)
    }

    fn lighting(&self, p: Palette) -> Div {
        let image = div().w(px(410.0)).h(px(560.0)).flex().flex_col().items_center().justify_center()
            .child(img(self.mouse.clone()).w(px(280.0)).h(px(500.0)).object_fit(ObjectFit::Contain))
            .child(div().mt(px(18.0)).child(muted(p, "Live preview of the selected effect")));
        let mut effects = div().flex().flex_col().gap(px(8.0));
        let lighting = self.readout.as_ref().and_then(|d| d.lighting.as_ref());
        for row in [["Flowing light", "Breathing", "Constant light"], ["Neon", "Colorful waves", "Close"]] {
            let mut line = div().flex().gap(px(8.0));
            for item in row {
                let selected = lighting.is_some_and(|light| matches!((item, light.mode), ("Breathing", LightMode::Breathing) | ("Constant light", LightMode::Constant) | ("Close", LightMode::Off)));
                line = line.child(div().flex_1().rounded(px(10.0)).bg(rgb(if selected { p.accent_soft } else { p.inset }))
                    .text_color(rgb(if selected { p.accent } else { p.text })).px(px(12.0)).py(px(16.0)).text_center().child(item));
            }
            effects = effects.child(line);
        }
        let card = card(p).flex_1().flex().flex_col().gap(px(14.0))
            .child(div().flex().justify_between().child(div().child(heading("Lighting Effect"))
                .child(muted(p, "Constant mode color = active DPI stage color (change colors on the DPI tab)")))
                .child(muted(p, "Reset lighting")))
            .child(effects)
            .child(muted(p, "Lighting controls are not available in the preview."))
            .child(slider_preview(p, "Brightness", lighting.map(|light| format!("{}%", light.brightness))))
            .child(slider_preview(p, "Speed", lighting.map(|light| light.speed.to_string())))
            .child(field(p, "Color", "", "—"));
        div().flex().gap(px(16.0)).items_start().child(image).child(card)
    }

    fn settings(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let sensor = self.readout.as_ref().and_then(|data| data.sensor.as_ref());
        let lod = match sensor.map(|s| s.lod) { Some(1) => "1mm", Some(2) => "2mm", _ => "—" };
        let angle = sensor.map(|s| if s.angle_snap { "On" } else { "Off" }).unwrap_or("—");
        let ripple = sensor.map(|s| if s.ripple { "On" } else { "Off" }).unwrap_or("—");
        let mut lod_field = field(p, "Lift Off Distance (LOD)", "1mm or 2mm", lod).id("sensor-lod");
        let mut angle_field = field(p, "Angle Snap", "Straighten out slight angle offsets", angle).id("sensor-angle");
        let mut ripple_field = field(p, "Ripple Control", "Smooth cursor jitter at high DPIs", ripple).id("sensor-ripple");
        if let Some(sensor) = sensor.filter(|_| self.demo || self.experimental_write) {
            let next_lod = if sensor.lod == 1 { 2 } else { 1 };
            let next_angle = !sensor.angle_snap;
            let next_ripple = !sensor.ripple;
            lod_field = lod_field.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.update_sensor(SensorChange::Lod(next_lod), cx)));
            angle_field = angle_field.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.update_sensor(SensorChange::AngleSnap(next_angle), cx)));
            ripple_field = ripple_field.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.update_sensor(SensorChange::Ripple(next_ripple), cx)));
        }
        let mouse = card(p).flex_1().child(heading("Mouse Settings"))
            .child(lod_field).child(angle_field).child(ripple_field)
            .child(field(p, "Debounce Time", "ms · higher reduces double-clicks", "—"))
            .child(field(p, "High-Speed Mode", "Local only", "—"));
        let power = card(p).flex_1().child(heading("Power & Sleep"))
            .child(field(p, "Sleep after", "Inactivity timeout", "—"))
            .child(field(p, "Move wake-up", "", "—"))
            .child(field(p, "Move to turn off light", "", "—"))
            .child(field(p, "Connection mode (display)", "UI only", "USB    2.4G"));
        let system = card(p).mt(px(12.0)).child(heading("System"))
            .child(div().flex().gap(px(10.0)).mt(px(10.0))
                .child(info_tile(p, "App", "Nibble 1.0.6"))
                .child(info_tile(p, "Device", "—"))
                .child(info_tile(p, "USB ID", "—")))
            .child(div().flex().justify_between().items_center().gap(px(12.0)).min_h(px(74.0))
                .child(div().flex().flex_col().gap(px(3.0)).child(div().font_weight(FontWeight::MEDIUM).child("Appearance"))
                    .child(muted(p, "Light or dark interface theme")))
                .child(div().flex().rounded(px(8.0)).bg(rgb(p.inset)).p(px(4.0))
                    .child(div().id("theme-light").cursor_pointer().rounded(px(6.0)).px(px(12.0)).py(px(8.0))
                        .bg(rgb(if !self.dark { p.hover } else { p.inset })).child("Light")
                        .on_click(cx.listener(|this, _, _, cx| this.set_theme(false, cx))))
                    .child(div().id("theme-dark").cursor_pointer().rounded(px(6.0)).px(px(12.0)).py(px(8.0))
                        .bg(rgb(if self.dark { p.hover } else { p.inset })).child("Dark")
                        .on_click(cx.listener(|this, _, _, cx| this.set_theme(true, cx))))))
            .child(field(p, "Left-click lock", "Prevents accidental rebinding of LMB", "—"))
            .child(field(p, "Profile backup", "Save or load this profile as a JSON file", "Export    Import"))
            .child(muted(p, "Restore factory settings"));
        let experimental = card(p).mt(px(12.0)).child(heading("Experimental Features"))
            .child(muted(p, "Software-level unlocks that bypass OEM app limitations."))
            .child(field(p, "50-Step DPI Granularity", "Unlock 50-step increments (default is 100)", "—"));
        div().flex().flex_col().child(div().flex().gap(px(12.0)).child(mouse).child(power)).child(system).child(experimental)
    }

    fn about(&self, p: Palette) -> Div {
        let hero = card(p).flex().flex_col().gap(px(14.0))
            .child(div().flex().items_center().gap(px(14.0))
                .child(img(self.logo.clone()).w(px(40.0)).h(px(40.0)).object_fit(ObjectFit::Contain))
                .child(div().flex().flex_col().child(div().text_size(px(24.0)).font_weight(FontWeight::BOLD).child("Nibble"))
                    .child(muted(p, "Open mouse configurator"))))
            .child("Configure DPI, polling rate, lighting, keys, sensor options, and power — clean, lightweight, and portable.")
            .child(div().flex().gap(px(10.0)).child(info_tile(p, "Version", "1.0.6"))
                .child(info_tile(p, "Platform", "Desktop & Web Companion"))
                .child(info_tile(p, "License", "GPL-3.0")));
        let derived = card(p).mt(px(16.0)).child(heading("Derived from"))
            .child(muted(p, "Protocol packets and device behavior were reverse-engineered from the official Windows driver package. Nibble is an independent reimplementation and is not affiliated with the original vendor."))
            .child(div().mt(px(14.0)).flex().gap(px(10.0)).child(info_tile(p, "Source software", "AJAZZ Driver (X)"))
                .child(info_tile(p, "Version", "1.0.1.4")).child(info_tile(p, "Build date", "2024.01.04"))
                .child(info_tile(p, "Method", "Static analysis + USB captures")));
        let capabilities = card(p).mt(px(16.0)).child(heading("What works"))
            .child(muted(p, if self.experimental_write {
                "Experimental controls can change DPI levels, the active stage, report rate, and sensor settings. Other settings are previews."
            } else {
                "This GPUI preview reads battery, DPI stages, and polling rate. Mouse settings cannot be changed."
            }));
        let privacy = card(p).mt(px(16.0)).child(heading("Privacy"))
            .child(muted(p, "This preview does not send telemetry."));
        div().flex().justify_center().child(div().w(px(720.0)).child(hero).child(derived).child(capabilities).child(privacy))
    }
}

impl Render for Nibble {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.dark);
        let page = match self.screen {
            0 => self.home(p), 1 => self.keys(p, cx), 2 => self.dpi(p, cx), 3 => self.lighting(p),
            4 => div().h(px(440.0)).flex().justify_center().items_center().text_color(rgb(p.muted)).child("Macros are not available in this version."),
            5 => self.settings(p, cx), _ => self.about(p),
        };
        let notice = if self.dpi_input.is_some() { "Enter a DPI value, then press Enter to apply or Escape to cancel.".to_string() }
            else if self.demo { "Sample data · The receiver is not accessed in demo mode.".to_string() }
            else if self.scanning { "Reading the receiver…".to_string() }
            else if let Some(error) = &self.error { format!("{error}. Close other mouse software, then refresh.") }
            else if let Some(message) = &self.message { message.clone() }
            else if self.experimental_write { "Experimental mode · DPI, report rate, and sensor controls can change the receiver. Close other mouse software.".into() }
            else { "GPUI preview · Settings are read-only and may be unavailable.".to_string() };
        div().size_full().bg(rgb(p.bg)).text_color(rgb(p.text)).font_family("Outfit").text_size(px(14.0))
            .flex().flex_col().px(px(20.0)).pt(px(8.0))
            .child(self.header(p, cx))
            .child(div().id("content").flex_1().min_h(px(0.0)).overflow_y_scroll().pt(px(5.0)).child(page))
            .child(div().h(px(26.0)).text_size(px(11.0)).text_color(rgb(p.muted)).child(notice))
    }
}

fn heading(label: &'static str) -> Div {
    div().text_size(px(16.0)).font_weight(FontWeight::SEMIBOLD).child(label)
}

fn theme_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|path| PathBuf::from(path).join("Nibble-gpui").join("theme"))
}

fn load_theme() -> bool {
    theme_path().and_then(|path| std::fs::read_to_string(path).ok())
        .map(|theme| theme.trim() != "light").unwrap_or(true)
}

fn muted(p: Palette, label: &'static str) -> Div {
    div().text_size(px(14.0)).text_color(rgb(p.muted)).child(label)
}

fn card(p: Palette) -> Div {
    div().rounded(px(16.0)).bg(rgb(p.card)).px(px(20.0)).py(px(18.0))
}

fn stat(p: Palette, label: &'static str, value: String, sub: String, large: bool) -> Div {
    div().flex_1().h_full().flex().flex_col().justify_center().gap(px(8.0))
        .child(div().text_size(px(12.0)).text_color(rgb(p.muted)).font_weight(FontWeight::MEDIUM).child(label))
        .child(div().text_size(px(if large { 32.0 } else { 20.0 })).font_weight(FontWeight::BOLD).child(value))
        .child(div().text_size(px(14.0)).text_color(rgb(p.muted)).child(sub))
}

fn static_button(p: Palette, label: &'static str) -> Div {
    div().flex_1().rounded(px(10.0)).bg(rgb(p.inset)).px(px(12.0)).py(px(16.0)).text_center().child(label)
}

fn field(p: Palette, title: &'static str, subtitle: &'static str, value: &'static str) -> Div {
    let is_switch = matches!(title, "Angle Snap" | "Ripple Control" | "High-Speed Mode" | "Move wake-up" | "Move to turn off light" | "Left-click lock" | "50-Step DPI Granularity");
    let control = if is_switch {
        div().w(px(48.0)).h(px(28.0)).rounded_full().bg(rgb(p.hover)).flex().items_center().justify_center()
            .text_color(rgb(if value == "On" { p.accent } else { p.muted })).child(value)
    } else {
        div().rounded(px(8.0)).bg(rgb(p.inset)).px(px(12.0)).py(px(8.0)).text_color(rgb(p.muted)).child(value)
    };
    div().flex().justify_between().items_center().gap(px(12.0)).min_h(px(74.0))
        .child(div().flex().flex_col().gap(px(3.0)).child(div().font_weight(FontWeight::MEDIUM).child(title)).child(muted(p, subtitle)))
        .child(control)
}

fn info_tile(p: Palette, label: &'static str, value: &'static str) -> Div {
    div().flex_1().min_h(px(100.0)).rounded(px(10.0)).bg(rgb(p.inset)).p(px(14.0)).flex().flex_col().gap(px(6.0))
        .child(div().text_size(px(11.0)).text_color(rgb(p.muted)).child(label))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(value))
}

fn dpi_position(value: u16) -> f32 {
    let minimum = 100.0f32.ln();
    ((f32::from(value).clamp(100.0, 26000.0).ln() - minimum) / (26000.0f32.ln() - minimum)).clamp(0.03, 0.97)
}

fn slider_preview(p: Palette, label: &'static str, value: Option<String>) -> Div {
    div().flex().items_center().gap(px(14.0)).h(px(34.0)).text_size(px(14.0)).text_color(rgb(p.muted))
        .child(div().w(px(100.0)).child(label))
        .child(div().flex_1().h(px(4.0)).rounded_full().bg(rgb(p.hover)))
        .child(div().w(px(36.0)).child(value.unwrap_or_else(|| "—".into())))
}

