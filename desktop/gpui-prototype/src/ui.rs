use crate::device::{self, Readout};
use gpui::{
    Context, Div, FontWeight, Image, ImageFormat, IntoElement, ObjectFit, Render, StyledImage,
    Window, div, img, prelude::*, px, rgb,
};
use std::sync::Arc;

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
    scanning: bool,
    readout: Option<Readout>,
    error: Option<String>,
    mouse: Arc<Image>,
    logo: Arc<Image>,
}

impl Nibble {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::scan(cx);
        Self {
            screen: 0,
            dark: true,
            scanning: true,
            readout: None,
            error: None,
            mouse: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../../../webapp/assets/device/mouse_aj179.png").to_vec(),
            )),
            logo: Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                include_bytes!("../../../webapp/assets/nibble.svg").to_vec(),
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

    fn header(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let status = if self.scanning {
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
                    .child(if self.scanning { "Checking…" } else { "Refresh" })
                    .on_click(cx.listener(|this, _, _, cx| { if !this.scanning { this.scanning = true; Self::scan(cx); cx.notify(); } })))
                .child(div().text_color(rgb(p.muted)).text_size(px(12.0)).child(format!("▰  {battery}")))
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Default Profile  ⌄"))
                .child(div().id("theme").cursor_pointer().rounded(px(8.0)).px(px(9.0)).py(px(7.0)).text_size(px(18.0))
                    .child(if self.dark { "☼" } else { "☾" })
                    .on_click(cx.listener(|this, _, _, cx| { this.dark = !this.dark; cx.notify(); })))
        ).child(tabs)
    }

    fn home(&self, p: Palette) -> Div {
        let data = self.readout.as_ref();
        let dpi = data.and_then(|d| d.active_dpi.and_then(|i| d.dpi.get(i))).map(u16::to_string).unwrap_or_else(|| "—".into());
        let stage = data.and_then(|d| d.active_dpi).map(|i| format!("Stage {}", i + 1)).unwrap_or_else(|| "Not available".into());
        let rate = data.and_then(|d| d.rate).map(|value| value.to_string()).unwrap_or_else(|| "—".into());
        let battery = data.and_then(|d| d.battery).map(|value| format!("{value}%")).unwrap_or_else(|| "—".into());
        let battery_sub = match data { Some(d) if d.sleeping => "Sleeping", Some(d) if d.charging => "Charging", Some(d) if d.battery.is_some() => "Last read", _ => "Unavailable" };
        let hero = div().flex_1().h(px(560.0)).flex().flex_col().items_center().justify_center()
            .child(img(self.mouse.clone()).w(px(280.0)).h(px(400.0)).object_fit(ObjectFit::Contain))
            .child(div().mt(px(18.0)).text_size(px(24.0)).font_weight(FontWeight::SEMIBOLD).child("AJAZZ AJ179 / AJ179P"))
            .child(div().mt(px(6.0)).text_size(px(14.0)).text_color(rgb(p.muted)).child("PAW3395 sensor"));
        let stats = div().flex_1().h(px(560.0)).flex().flex_col()
            .child(div().flex().h(px(185.0)).child(stat(p, "DPI", dpi, stage, true)).child(stat(p, "POLLING RATE", rate, "Hz".into(), true)))
            .child(div().h(px(185.0)).child(stat(p, "LIGHTING", "—".into(), "Not read in preview".into(), false)))
            .child(div().flex().h(px(185.0)).child(stat(p, "CONNECTION", "USB".into(), if data.is_some() { "Receiver found".into() } else { "Not connected".into() }, false))
                .child(stat(p, "BATTERY", battery, battery_sub.into(), false)));
        div().flex().w_full().gap(px(12.0)).child(hero).child(stats)
    }

    fn dpi(&self, p: Palette) -> Div {
        let data = self.readout.as_ref();
        let mut stage_rows = div().flex().flex_col().gap(px(8.0));
        for pair in 0..3 {
            let mut row = div().flex().gap(px(8.0));
            for i in pair * 2..pair * 2 + 2 {
                let value = data.and_then(|d| d.dpi.get(i)).map(u16::to_string).unwrap_or_else(|| "—".into());
                let active = data.and_then(|d| d.active_dpi) == Some(i);
                row = row.child(div().rounded(px(8.0)).bg(rgb(if active { p.accent_soft } else { p.inset })).px(px(10.0)).py(px(6.0)).flex().items_center().gap(px(8.0))
                    .child(div().w(px(28.0)).h(px(28.0)).rounded(px(6.0)).bg(rgb(STAGE_COLORS[i])))
                    .child(div().text_size(px(12.0)).font_weight(FontWeight::SEMIBOLD).child(format!("Stage {}", i + 1)))
                    .child(div().font_family("JetBrains Mono").text_size(px(12.0)).text_color(rgb(p.muted)).child(format!("{value} DPI")))
                    .child(if active { "ACTIVE" } else { "" }));
            }
            stage_rows = stage_rows.child(row);
        }
        let mut values = div().flex().justify_between().w_full();
        for i in 0..6 {
            let value = data.and_then(|d| d.dpi.get(i)).map(u16::to_string).unwrap_or_else(|| "—".into());
            values = values.child(div().font_weight(FontWeight::BOLD).text_size(px(18.0)).child(value));
        }
        let chart = div().mt(px(55.0)).h(px(160.0)).px(px(12.0)).flex().flex_col().justify_center().gap(px(16.0))
            .child(values)
            .child(div().h(px(2.0)).w_full().bg(rgb(p.muted)))
            .child(div().flex().justify_between().text_size(px(12.0)).text_color(rgb(p.muted)).child("200").child("1000").child("2000").child("3000").child("4000").child("6000").child("8000").child("26000"));
        let levels = div().flex().items_center().gap(px(8.0)).text_size(px(12.0)).text_color(rgb(p.muted)).child("DPI Levels:")
            .child(div().rounded(px(8.0)).bg(rgb(p.inset)).px(px(12.0)).py(px(10.0)).child("1     2     3     4     5     6"));
        let hero = card(p).flex_1().h(px(470.0)).px(px(36.0)).py(px(28.0)).flex().flex_col()
            .child(div().flex().items_center().justify_between()
                .child(div().flex().items_center().gap(px(20.0)).child(heading("DPI Sensitivity")).child(levels))
                .child(muted(p, "Reset DPI")))
            .child(chart)
            .child(div().mt(px(25.0)).flex().items_center().justify_between().gap(px(20.0))
                .child(stage_rows)
                .child(div().flex().gap(px(14.0)).text_size(px(12.0)).text_color(rgb(p.muted)).child("DPI Value   —").child("LED Color   —")));
        let mut rate_options = div().flex().flex_col().gap(px(8.0)).mt(px(14.0));
        for pair in [[125, 250], [500, 1000]] {
            let mut row = div().flex().gap(px(8.0));
            for rate in pair {
                let active = data.and_then(|d| d.rate) == Some(rate);
                row = row.child(div().flex_1().rounded(px(10.0)).h(px(80.0)).flex().flex_col().justify_center().items_center().gap(px(4.0))
                    .bg(rgb(if active { p.accent_soft } else { p.inset })).text_color(rgb(if active { p.accent } else { p.text }))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(format!("{rate} Hz")))
                    .child(div().text_size(px(11.0)).text_color(rgb(p.muted)).child(if rate == 1000 { "Default" } else { "Report rate" })));
            }
            rate_options = rate_options.child(row);
        }
        let rates = card(p).w(px(280.0)).h(px(470.0)).child(heading("Report Rate"))
            .child(muted(p, "How often the mouse reports position to the PC"))
            .child(rate_options);
        div().flex().w_full().gap(px(16.0)).child(hero).child(rates)
    }

    fn keys(&self, p: Palette) -> Div {
        let left = div().flex_1().h(px(620.0)).flex().flex_col()
            .child(div().flex().justify_between().child(div().child(heading("Mouse Buttons")).child(muted(p, "Click a callout to rebind. Grayed options are not available yet."))).child(muted(p, "Reset all keys")))
            .child(div().relative().flex_1().flex().justify_center().items_center()
                .child(img(self.mouse.clone()).w(px(280.0)).h(px(400.0)).object_fit(ObjectFit::Contain))
                .child(callout(p, "Middle / Wheel · —", 178.0, 50.0))
                .child(callout(p, "Left Button · —", 35.0, 140.0))
                .child(callout(p, "Right Button · —", 286.0, 140.0))
                .child(callout(p, "Forward · —", 10.0, 206.0))
                .child(callout(p, "Backward · —", 0.0, 265.0))
                .child(callout(p, "DPI Button · —", 265.0, 265.0)));
        let mut functions = div().flex().flex_col().gap(px(8.0));
        for pair in [["Left-Click", "Right-Click"], ["Middle-Click", "Forward"], ["Backward", "Double-Click"], ["Scroll Up", "Scroll Down"], ["DPI Loop", "Fire button"], ["Disable Key", ""]] {
            let mut row = div().flex().gap(px(8.0));
            for label in pair {
                row = row.child(div().flex_1().rounded(px(8.0)).px(px(12.0)).py(px(11.0)).bg(rgb(p.inset)).child(label));
            }
            functions = functions.child(row);
        }
        let right = card(p).flex_1().h(px(620.0)).flex().flex_col().gap(px(10.0))
            .child(heading("Button Function"))
            .child(div().rounded(px(8.0)).p(px(12.0)).bg(rgb(p.accent_soft)).text_color(rgb(p.accent)).child("Select a button"))
            .child(muted(p, "MOUSE FEATURES"))
            .child(functions)
            .child(muted(p, "MULTIMEDIA"))
            .child(div().flex().gap(px(8.0)).child(static_button(p, "Volume +")).child(static_button(p, "Volume -")));
        div().flex().gap(px(16.0)).child(left).child(right)
    }

    fn lighting(&self, p: Palette) -> Div {
        let image = div().w(px(410.0)).h(px(560.0)).flex().flex_col().items_center().justify_center()
            .child(img(self.mouse.clone()).w(px(280.0)).h(px(400.0)).object_fit(ObjectFit::Contain))
            .child(div().mt(px(18.0)).child(muted(p, "Live preview of the selected effect")));
        let mut effects = div().flex().flex_col().gap(px(8.0));
        for row in [["Flowing light", "Breathing", "Constant light"], ["Neon", "Colorful waves", "Close"]] {
            let mut line = div().flex().gap(px(8.0));
            for item in row { line = line.child(static_button(p, item)); }
            effects = effects.child(line);
        }
        let card = card(p).flex_1().flex().flex_col().gap(px(14.0))
            .child(div().flex().justify_between().child(div().child(heading("Lighting Effect"))
                .child(muted(p, "Constant mode color = active DPI stage color (change colors on the DPI tab)")))
                .child(muted(p, "Reset lighting")))
            .child(effects)
            .child(muted(p, "Lighting settings are not available in the preview."))
            .child(field(p, "Brightness", "", "—"))
            .child(field(p, "Speed", "", "—"))
            .child(field(p, "Color", "", "—"));
        div().flex().gap(px(16.0)).items_start().child(image).child(card)
    }

    fn settings(&self, p: Palette) -> Div {
        let mouse = card(p).flex_1().child(heading("Mouse Settings"))
            .child(field(p, "Lift Off Distance (LOD)", "1mm or 2mm", "1mm    2mm"))
            .child(field(p, "Angle Snap", "Straighten out slight angle offsets", "—"))
            .child(field(p, "Ripple Control", "Smooth cursor jitter at high DPIs", "—"))
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
            .child(field(p, "Appearance", "Light or dark interface theme", if self.dark { "Dark" } else { "Light" }))
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
            .child(muted(p, "This GPUI preview reads battery, DPI stages, and polling rate. Mouse settings cannot be changed yet."));
        let privacy = card(p).mt(px(16.0)).child(heading("Privacy"))
            .child(muted(p, "This preview does not send telemetry."));
        div().flex().justify_center().child(div().w(px(720.0)).child(hero).child(derived).child(capabilities).child(privacy))
    }
}

impl Render for Nibble {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.dark);
        let page = match self.screen {
            0 => self.home(p), 1 => self.keys(p), 2 => self.dpi(p), 3 => self.lighting(p),
            4 => div().h(px(440.0)).flex().justify_center().items_center().text_color(rgb(p.muted)).child("Macros are not available in this version."),
            5 => self.settings(p), _ => self.about(p),
        };
        let notice = if self.scanning { "Reading the receiver…".to_string() }
            else if let Some(error) = &self.error { format!("{error}. Close other mouse software, then refresh.") }
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
    div().flex().justify_between().items_center().gap(px(12.0)).min_h(px(74.0))
        .child(div().flex().flex_col().gap(px(3.0)).child(div().font_weight(FontWeight::MEDIUM).child(title)).child(muted(p, subtitle)))
        .child(div().rounded(px(8.0)).bg(rgb(p.inset)).px(px(12.0)).py(px(8.0)).text_color(rgb(p.muted)).child(value))
}

fn callout(p: Palette, label: &'static str, left: f32, top: f32) -> Div {
    div().absolute().left(px(left)).top(px(top)).rounded(px(8.0)).bg(rgb(p.card)).px(px(12.0)).py(px(8.0)).text_size(px(12.0)).child(label)
}

fn info_tile(p: Palette, label: &'static str, value: &'static str) -> Div {
    div().flex_1().min_h(px(100.0)).rounded(px(10.0)).bg(rgb(p.inset)).p(px(14.0)).flex().flex_col().gap(px(6.0))
        .child(div().text_size(px(11.0)).text_color(rgb(p.muted)).child(label))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(value))
}

