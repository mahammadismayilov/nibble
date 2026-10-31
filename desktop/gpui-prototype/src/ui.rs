use crate::device::{self, LightChange, LightMode, Lighting, PowerSettings, Readout, Sensor, SensorChange};
use crate::profile::{self, Profiles, ReadoutScope};
use gpui::{
    Bounds, Context, Div, FocusHandle, FontWeight, Image, ImageFormat, IntoElement, KeyDownEvent, MouseButton, ObjectFit, PathPromptOptions, Pixels, Point, Render, ScrollHandle, StyledImage,
    Window, canvas, div, img, point, prelude::*, px, relative, rgb,
};
use std::{cell::Cell, rc::Rc};
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;

const TABS: [&str; 7] = ["Home", "Keys", "DPI", "Lighting", "Macro", "Settings", "About"];

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
    dpi_track_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    dpi_drag: Option<(usize, u16)>,
    light_brightness_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    light_speed_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    light_drag: Option<(bool, u8)>,
    light_brightness_focus: FocusHandle,
    light_speed_focus: FocusHandle,
    color_focus: FocusHandle,
    color_input: Option<String>,
    color_stage: Option<usize>,
    color_replace_input: bool,
    selected_key: usize,
    confirm_reset_keys: bool,
    content_scroll: ScrollHandle,
    profiles: Profiles,
    profiles_writable: bool,
    show_profiles: bool,
    mouse: Arc<Image>,
    logo: Arc<Image>,
}

impl Nibble {
    pub fn new(cx: &mut Context<Self>, demo: bool, experimental_write: bool, demo_screen: usize, demo_light: bool) -> Self {
        if !demo { Self::scan(cx); Self::schedule_status_poll(cx); }
        let (profiles, profile_error, profiles_writable) = if demo {
            (Profiles::new(), None, true)
        } else if let Some(path) = profile::profile_path() {
            match Profiles::load(&path) {
                Ok(profiles) => (profiles, None, true),
                Err(error) => (Profiles::new(), Some(format!("{error}. Saved files were not changed.")), false),
            }
        } else {
            (Profiles::new(), Some("Could not locate the profile folder.".into()), false)
        };
        let readout = demo.then(|| Readout {
            battery: Some(80),
            dpi: vec![400, 800, 1200, 1600, 2400, 10000],
            active_dpi: Some(5),
            dpi_count: Some(6),
            rate: Some(1000),
            lighting: Some(Lighting { mode: LightMode::Breathing, brightness: 0, speed: 6, color: Some([128, 0, 128]) }),
            sensor: Some(Sensor { lod: 1, angle_snap: false, ripple: false }),
            keys: Some([[0x10, 1, 0], [0x10, 2, 0], [0x10, 4, 0], [0x10, 8, 0], [0x10, 16, 0], [0x40, 1, 0]]),
            vid: 0x249a,
            pid: 0x5c2f,
            model_confirmed: true,
            ..Default::default()
        });
        Self {
            screen: if demo { demo_screen.min(TABS.len() - 1) } else { 0 },
            dark: if demo { !demo_light } else { load_theme() },
            demo,
            experimental_write,
            scanning: !demo,
            applying: false,
            readout,
            error: None,
            message: profile_error,
            dpi_focus: cx.focus_handle(),
            dpi_input: None,
            dpi_replace_input: false,
            dpi_track_bounds: Rc::new(Cell::new(None)),
            dpi_drag: None,
            light_brightness_bounds: Rc::new(Cell::new(None)),
            light_speed_bounds: Rc::new(Cell::new(None)),
            light_drag: None,
            light_brightness_focus: cx.focus_handle(),
            light_speed_focus: cx.focus_handle(),
            color_focus: cx.focus_handle(),
            color_input: None,
            color_stage: None,
            color_replace_input: false,
            selected_key: 0,
            confirm_reset_keys: false,
            content_scroll: ScrollHandle::new(),
            profiles,
            profiles_writable,
            show_profiles: false,
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
                    Ok(data) => {
                        view.readout = Some(data);
                        view.error = None;
                    }
                    Err(error) => { view.readout = None; view.error = Some(error); }
                }
                cx.notify();
            });
        }).detach();
    }

    fn schedule_status_poll(cx: &mut Context<Self>) {
        let executor = cx.background_executor().clone();
        cx.spawn(async move |view, cx| {
            loop {
                executor.timer(Duration::from_secs(15)).await;
                if view.update(cx, |view, cx| view.poll_status(cx)).is_err() { break; }
            }
        }).detach();
    }

    fn poll_status(&mut self, cx: &mut Context<Self>) {
        if self.demo || self.scanning || self.applying || self.dpi_drag.is_some() || self.light_drag.is_some()
            || self.dpi_input.is_some() || self.color_input.is_some() { return; }
        let Some(snapshot) = self.readout.clone() else {
            self.scanning = true;
            Self::scan(cx);
            cx.notify();
            return;
        };
        self.scanning = true;
        let work = cx.background_executor().spawn(async move { device::poll_status(&snapshot) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.scanning = false;
                match result {
                    device::StatusPoll::Status(status) => {
                        if let Some(data) = &mut view.readout {
                            data.battery = status.battery;
                            data.charging = status.charging;
                            data.sleeping = status.sleeping;
                        }
                    }
                    device::StatusPoll::Disconnected => {
                        view.readout = None;
                        view.error = Some("Receiver disconnected".into());
                    }
                    device::StatusPoll::Unsupported(error) => {
                        view.readout = None;
                        view.error = Some(error);
                    }
                    device::StatusPoll::Unavailable => {
                        if let Some(data) = &mut view.readout { data.battery = None; data.charging = false; data.sleeping = false; }
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
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

    fn save_profiles(&self) -> Result<(), String> {
        if self.demo { return Ok(()); }
        if !self.profiles_writable { return Err("Saved profiles could not be loaded; the file was not changed.".into()); }
        let path = profile::profile_path().ok_or("Could not locate the profile folder")?;
        self.profiles.save(&path)
    }

    fn sync_profile_from_readout(&mut self, scope: ReadoutScope) -> Result<(), String> {
        let Some(readout) = self.readout.as_ref() else { return Ok(()); };
        let previous = self.profiles.clone();
        if let Err(error) = self.profiles.sync_scope(readout, scope).and_then(|_| self.save_profiles()) {
            self.profiles = previous;
            return Err(format!("Could not save synchronized profile: {error}"));
        }
        Ok(())
    }

    fn confirm_and_save(&mut self, scope: ReadoutScope, message: &'static str) {
        self.message = Some(match self.sync_profile_from_readout(scope) { Ok(()) => message.into(), Err(error) => error });
    }

    fn select_profile(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.scanning || self.applying { self.message = Some("Wait until the current device operation finishes.".into()); cx.notify(); return; }
        self.dpi_drag = None;
        self.color_input = None;
        self.color_stage = None;
        let previous = self.profiles.clone();
        let result = self.profiles.select(index).and_then(|_| self.save_profiles());
        if let Err(error) = result { self.profiles = previous; self.message = Some(error); }
        else { self.message = Some("Profile selected locally. Mouse settings were not changed.".into()); }
        self.show_profiles = false;
        cx.notify();
    }

    fn update_local_setting(&mut self, key: &'static str, value: serde_json::Value, cx: &mut Context<Self>) {
        let previous = self.profiles.clone();
        let result = self.profiles.set_local_setting(key, value).and_then(|_| self.save_profiles());
        if let Err(error) = result { self.profiles = previous; self.message = Some(error); }
        else { self.message = Some("Profile setting saved locally.".into()); }
        cx.notify();
    }

    fn import_profile(&mut self, cx: &mut Context<Self>) {
        if self.scanning || self.applying { self.message = Some("Wait until the current device operation finishes.".into()); cx.notify(); return; }
        if !self.profiles_writable { self.message = Some("Saved profiles could not be loaded; import is disabled.".into()); cx.notify(); return; }
        let picker = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Import profile".into()) });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |view, cx| {
                if view.scanning || view.applying { view.message = Some("Wait until the current device operation finishes, then import again.".into()); cx.notify(); return; }
                let path = match result {
                    Ok(Ok(Some(paths))) => paths.into_iter().next(),
                    Ok(Ok(None)) => return,
                    Ok(Err(error)) => { view.message = Some(format!("Could not open profile: {error}")); cx.notify(); return; }
                    Err(error) => { view.message = Some(format!("Profile picker failed: {error}")); cx.notify(); return; }
                };
                if let Some(path) = path {
                    let previous = view.profiles.clone();
                    let result = std::fs::metadata(&path).map_err(|e| e.to_string()).and_then(|meta| {
                        if meta.len() > 1_048_576 { Err("Profile file is too large".into()) }
                        else { std::fs::read_to_string(&path).map_err(|e| e.to_string()) }
                    }).and_then(|data| view.profiles.import(&data)).and_then(|_| view.save_profiles());
                    match result {
                        Ok(()) => {
                            view.color_input = None;
                            view.color_stage = None;
                            view.message = Some("Profile imported locally. Mouse settings were not changed.".into());
                        }
                        Err(error) => { view.profiles = previous; view.message = Some(format!("Could not import profile: {error}")); }
                    }
                    cx.notify();
                }
            });
        }).detach();
    }

    fn export_profile(&mut self, cx: &mut Context<Self>) {
        let Some(directory) = profile::profile_path().and_then(|path| path.parent().map(|p| p.to_path_buf())) else {
            self.message = Some("Could not locate the profile folder.".into()); cx.notify(); return;
        };
        let directory = if directory.exists() { directory } else { std::env::temp_dir() };
        let picker = cx.prompt_for_new_path(&directory, Some("nibble-profile.json"));
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |view, cx| {
                let path = match result {
                    Ok(Ok(Some(path))) => path,
                    Ok(Ok(None)) => return,
                    Ok(Err(error)) => { view.message = Some(format!("Could not export profile: {error}")); cx.notify(); return; }
                    Err(error) => { view.message = Some(format!("Profile picker failed: {error}")); cx.notify(); return; }
                };
                let result = view.profiles.export_current().and_then(|text| {
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| e.to_string())?;
                    file.write_all(text.as_bytes()).map_err(|e| e.to_string())
                });
                view.message = Some(match result { Ok(()) => "Profile exported.".into(), Err(error) => format!("Could not export profile: {error}") });
                cx.notify();
            });
        }).detach();
    }

    fn select_dpi(&mut self, index: usize, cx: &mut Context<Self>) {
        self.dpi_input = None;
        self.dpi_drag = None;
        self.color_input = None;
        self.color_stage = None;
        self.update_dpi(index, false, cx);
    }

    fn select_dpi_count(&mut self, count: usize, cx: &mut Context<Self>) {
        self.dpi_input = None;
        self.dpi_drag = None;
        self.color_input = None;
        self.color_stage = None;
        self.update_dpi(count, true, cx);
    }

    fn edit_dpi(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        self.dpi_drag = None;
        let Some(value) = self.readout.as_ref().and_then(|data| data.active_dpi.and_then(|i| data.dpi.get(i))) else { return; };
        self.dpi_input = Some(value.to_string());
        self.dpi_replace_input = true;
        window.focus(&self.dpi_focus);
        cx.notify();
    }

    fn dpi_for_pointer(&self, position: Point<Pixels>) -> Option<u16> {
        let bounds = self.dpi_track_bounds.get()?;
        let width: f32 = bounds.size.width.into();
        if width <= 0.0 { return None; }
        let offset: f32 = (position.x - bounds.origin.x).into();
        let step = if self.profiles.current()["settings"]["fineDpi"].as_bool() == Some(true) { 50 } else { 100 };
        Some(dpi_from_position(offset / width, step))
    }

    fn start_dpi_drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        let Some(stage) = self.readout.as_ref().and_then(|data| data.active_dpi) else { return; };
        if let Some(value) = self.dpi_for_pointer(position) {
            self.dpi_input = None;
            self.dpi_drag = Some((stage, value));
            cx.notify();
        }
    }

    fn move_dpi_drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((stage, _)) = self.dpi_drag else { return; };
        if let Some(value) = self.dpi_for_pointer(position) { self.dpi_drag = Some((stage, value)); cx.notify(); }
    }

    fn finish_dpi_drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((stage, pending)) = self.dpi_drag.take() else { return; };
        let value = self.dpi_for_pointer(position).unwrap_or(pending);
        let current = self.readout.as_ref().and_then(|data| data.active_dpi.map(|active| (active, data.dpi.get(active).copied())));
        if current.is_some_and(|(active, existing)| active == stage && existing != Some(value)) {
            self.set_dpi_value(value, cx);
        }
        cx.notify();
    }

    fn edit_color(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scanning || self.applying || !self.profiles_writable { return; }
        self.dpi_drag = None;
        let Some(stage) = self.readout.as_ref().and_then(|data| data.active_dpi) else { return; };
        let Some(color) = self.profiles.current()["dpiStages"][stage]["color"].as_str() else { return; };
        self.color_input = Some(color.to_string());
        self.color_stage = Some(stage);
        self.color_replace_input = true;
        window.focus(&self.color_focus);
        cx.notify();
    }

    fn handle_color_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(input) = self.color_input.as_mut() else { return; };
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.control && key.eq_ignore_ascii_case("a") {
            self.color_replace_input = true;
        } else if key == "escape" {
            self.color_input = None;
            self.color_stage = None;
        } else if key == "enter" || key == "return" {
            let value = input.clone();
            let stage = self.color_stage.take();
            self.color_input = None;
            if let Some(stage) = stage {
                let previous = self.profiles.clone();
                let result = self.profiles.set_dpi_color(stage, &value).and_then(|_| self.save_profiles());
                if let Err(error) = result { self.profiles = previous; self.message = Some(error); }
                else { self.message = Some("DPI color saved in the profile. Mouse colors were not changed.".into()); }
            }
        } else if key == "backspace" {
            if self.color_replace_input { input.clear(); self.color_replace_input = false; } else { input.pop(); }
        } else if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
            let typed = event.keystroke.key_char.as_deref().unwrap_or(key);
            if typed.len() == 1 && (typed.as_bytes()[0].is_ascii_hexdigit() || typed == "#") {
                if self.color_replace_input { input.clear(); self.color_replace_input = false; }
                if input.len() < 7 { input.push_str(typed); }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn apply_profile_colors(&mut self, cx: &mut Context<Self>) {
        if self.scanning || self.applying || self.color_input.is_some() || !self.profiles_writable { return; }
        let colors = match self.profiles.dpi_colors() {
            Ok(colors) => colors,
            Err(error) => { self.message = Some(error); cx.notify(); return; }
        };
        if self.demo {
            if let Some(data) = &mut self.readout {
                if let (Some(stage), Some(light)) = (data.active_dpi, data.lighting.as_mut()) {
                    if light.mode != LightMode::Off { light.color = Some(colors[stage]); }
                }
            }
            cx.notify(); return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Sending six profile colors…".into());
        let work = cx.background_executor().spawn(async move { device::set_dpi_colors(&snapshot, colors) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok(light) => {
                        if let Some(light) = light { if let Some(data) = &mut view.readout { data.lighting = Some(light); } }
                        view.message = Some("Six profile colors sent. The color table cannot be confirmed by the receiver.".into());
                    }
                    Err(error) => { if let Some(data) = &mut view.readout { data.lighting = None; } view.message = Some(error); }
                }
                cx.notify();
            });
        }).detach();
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
            let step = if self.profiles.current()["settings"]["fineDpi"].as_bool() == Some(true) { 50 } else { 100 };
            let value = input.parse::<u32>().ok().map(|value| ((value.min(26000) + step / 2) / step * step).clamp(50, 26000) as u16);
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
                        view.confirm_and_save(ReadoutScope::DpiValue(index), "DPI value confirmed by receiver.");
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
                        view.confirm_and_save(if is_count { ReadoutScope::DpiCount } else { ReadoutScope::DpiSelection }, "DPI settings confirmed by receiver.");
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
                        view.confirm_and_save(ReadoutScope::Rate, "Report rate confirmed by receiver.");
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
                        view.confirm_and_save(ReadoutScope::Sensor(change), "Sensor setting confirmed by receiver.");
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

    fn update_power(&mut self, power: PowerSettings, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        if self.demo {
            if let Err(error) = self.profiles.set_power(&power) { self.message = Some(error); }
            cx.notify(); return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Applying power settings…".into());
        let sent_power = power.clone();
        let work = cx.background_executor().spawn(async move { device::set_power(&snapshot, &sent_power) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok(()) => {
                        let previous = view.profiles.clone();
                        match view.profiles.set_power(&power).and_then(|_| view.save_profiles()) {
                            Ok(()) => view.message = Some("Power settings sent. This mouse does not report them back.".into()),
                            Err(error) => { view.profiles = previous; view.message = Some(format!("Power settings were sent, but the profile could not be saved: {error}")); }
                        }
                    }
                    Err(error) => view.message = Some(format!("{error} The mouse may have received the change.")),
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
                        view.confirm_and_save(ReadoutScope::Key(index), "Button assignment confirmed by receiver.");
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

    fn reset_keys(&mut self, cx: &mut Context<Self>) {
        if !self.confirm_reset_keys || self.scanning || self.applying { return; }
        self.confirm_reset_keys = false;
        if self.demo {
            if let Some(data) = &mut self.readout { data.keys = Some(device::default_keymap()); }
            self.confirm_and_save(ReadoutScope::Keys, "Button assignments reset in sample data.");
            cx.notify(); return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Resetting button assignments…".into());
        let work = cx.background_executor().spawn(async move { device::reset_keys(&snapshot) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok(keys) => {
                        if let Some(data) = &mut view.readout { data.keys = Some(keys); }
                        view.confirm_and_save(ReadoutScope::Keys, "Default button assignments confirmed by receiver.");
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

    fn update_lighting(&mut self, change: LightChange, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        if self.demo {
            if let Some(light) = self.readout.as_mut().and_then(|data| data.lighting.as_mut()) {
                match change { LightChange::Mode(mode) => light.mode = mode, LightChange::Brightness(value) => light.brightness = value, LightChange::Speed(value) => light.speed = value }
            }
            cx.notify();
            return;
        }
        if !self.experimental_write { return; }
        let Some(snapshot) = self.readout.clone() else { return; };
        self.applying = true;
        self.message = Some("Applying lighting…".into());
        let work = cx.background_executor().spawn(async move { device::set_lighting(&snapshot, change) });
        cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                view.applying = false;
                match result {
                    Ok(light) => {
                        if let Some(data) = &mut view.readout { data.lighting = Some(light); }
                        view.confirm_and_save(ReadoutScope::Lighting(change), "Lighting settings confirmed by receiver.");
                    }
                    Err(error) => {
                        if let Some(data) = &mut view.readout { data.lighting = None; }
                        view.message = Some(error);
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn light_value_at(&self, speed: bool, position: Point<Pixels>) -> Option<u8> {
        let bounds = if speed { &self.light_speed_bounds } else { &self.light_brightness_bounds }.get()?;
        let width: f32 = bounds.size.width.into();
        if width <= 0.0 { return None; }
        let offset: f32 = (position.x - bounds.origin.x).into();
        Some(light_slider_value(offset / width, speed))
    }

    fn start_light_drag(&mut self, speed: bool, position: Point<Pixels>, cx: &mut Context<Self>) {
        if self.scanning || self.applying { return; }
        if let Some(value) = self.light_value_at(speed, position) { self.light_drag = Some((speed, value)); cx.notify(); }
    }

    fn move_light_drag(&mut self, speed: bool, position: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.light_drag.is_some_and(|(active, _)| active == speed) { return; }
        if let Some(value) = self.light_value_at(speed, position) { self.light_drag = Some((speed, value)); cx.notify(); }
    }

    fn finish_light_drag(&mut self, speed: bool, position: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.light_drag.is_some_and(|(active, _)| active == speed) { return; }
        let Some((active, pending)) = self.light_drag.take() else { return; };
        if active != speed { cx.notify(); return; }
        let value = self.light_value_at(speed, position).unwrap_or(pending);
        let light = self.readout.as_ref().and_then(|data| data.lighting.as_ref());
        let allowed = light.is_some_and(|light| if speed { light.mode == LightMode::Breathing && light.speed != value }
            else { light.mode != LightMode::Off && light.brightness != value });
        if allowed { self.update_lighting(if speed { LightChange::Speed(value) } else { LightChange::Brightness(value) }, cx); }
        cx.notify();
    }

    fn light_slider(&self, p: Palette, cx: &mut Context<Self>, speed: bool, current: Option<u8>, enabled: bool) -> Div {
        let value = self.light_drag.filter(|(active, _)| *active == speed).map(|(_, value)| value).or(current);
        let fraction = value.map(|value| if speed { f32::from(value.saturating_sub(1)) / 9.0 } else { f32::from(value) / 100.0 }).unwrap_or(0.0);
        let bounds = if speed { self.light_speed_bounds.clone() } else { self.light_brightness_bounds.clone() };
        let focus = if speed { &self.light_speed_focus } else { &self.light_brightness_focus };
        let mut track = div().id(if speed { "light-speed" } else { "light-brightness" }).track_focus(focus).relative().flex_1().h(px(28.0))
            .child(canvas(move |rect, _, _| { bounds.set(Some(rect)); }, |_, _, _, _| {}).absolute().size_full())
            .child(div().absolute().top(px(12.0)).w_full().h(px(4.0)).rounded_full().bg(rgb(p.hover)))
            .child(div().absolute().top(px(12.0)).w(relative(fraction)).h(px(4.0)).rounded_full().bg(rgb(p.accent)))
            .child(div().absolute().left(relative(fraction)).top(px(7.0)).ml(px(-7.0)).size(px(14.0)).rounded_full()
                .bg(rgb(if enabled { p.accent } else { p.muted })));
        if enabled {
            track = track.cursor_pointer()
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                    window.focus(if speed { &this.light_speed_focus } else { &this.light_brightness_focus });
                    this.start_light_drag(speed, event.position, cx);
                }))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                    let current = this.readout.as_ref().and_then(|data| data.lighting.as_ref()).map(|light| if speed { light.speed } else { light.brightness });
                    if let Some(current) = current {
                        let step: u8 = if speed { 1 } else { 25 };
                        let min = if speed { 1 } else { 0 };
                        let max = if speed { 10 } else { 100 };
                        let next = match event.keystroke.key.as_str() {
                            "right" | "up" if speed => Some(next_reported_speed(current, true)),
                            "left" | "down" if speed => Some(next_reported_speed(current, false)),
                            "right" | "up" => Some(current.saturating_add(step).min(max)),
                            "left" | "down" => Some(current.saturating_sub(step).max(min)),
                            "home" => Some(min), "end" => Some(max), _ => None,
                        };
                        if let Some(next) = next {
                            if next != current { this.update_lighting(if speed { LightChange::Speed(next) } else { LightChange::Brightness(next) }, cx); }
                            cx.stop_propagation();
                        }
                    }
                }))
                .on_mouse_move(cx.listener(move |this, event: &gpui::MouseMoveEvent, _, cx| {
                    if event.pressed_button == Some(MouseButton::Left) { this.move_light_drag(speed, event.position, cx); }
                    else if this.light_drag.take().is_some() { cx.notify(); }
                }))
                .on_mouse_up(MouseButton::Left, cx.listener(move |this, event: &gpui::MouseUpEvent, _, cx| this.finish_light_drag(speed, event.position, cx)))
                .on_mouse_up_out(MouseButton::Left, cx.listener(move |this, event: &gpui::MouseUpEvent, _, cx| this.finish_light_drag(speed, event.position, cx)));
        }
        let display = value.map(|value| if speed { value.to_string() } else { format!("{value}%") }).unwrap_or_else(|| "—".into());
        div().flex().items_center().gap(px(12.0)).text_size(px(12.0)).text_color(rgb(p.muted))
            .child(div().w(px(100.0)).child(if speed { "Speed" } else { "Brightness" }))
            .child(track).child(div().w(px(40.0)).text_right().child(display))
    }

    fn header(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let status = if self.demo {
            "Sample data · No receiver access".to_string()
        } else if self.scanning {
            "Checking receiver".to_string()
        } else if let Some(data) = &self.readout {
            format!("{} · {:04X}:{:04X} · Last read", if data.model_confirmed { "AJAZZ AJ179 / AJ179P" } else { "Compatible receiver · model unconfirmed" }, data.vid, data.pid)
        } else {
            "Disconnected · Connect mouse receiver to configure".into()
        };
        let battery = self.readout.as_ref().and_then(|data| data.battery).map(|value| format!("{value}%")).unwrap_or_else(|| "—".into());
        let mut profile_menu = div().absolute().top(px(40.0)).right(px(0.0)).w(px(180.0)).rounded(px(8.0)).bg(rgb(p.card)).p(px(5.0)).flex().flex_col();
        if self.show_profiles {
            for (index, name) in self.profiles.names().into_iter().enumerate() {
                profile_menu = profile_menu.child(div().id(("profile-option", index)).cursor_pointer().rounded(px(6.0)).px(px(10.0)).py(px(8.0)).child(name.to_string())
                    .on_click(cx.listener(move |this, _, _, cx| this.select_profile(index, cx))));
            }
        }
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
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.screen != index { this.content_scroll.set_offset(point(px(0.0), px(0.0))); }
                        this.screen = index; this.dpi_input = None; this.dpi_drag = None; this.light_drag = None; this.color_input = None; this.color_stage = None; this.confirm_reset_keys = false; cx.notify();
                    })),
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
                .child(div().relative().font_weight(FontWeight::SEMIBOLD)
                    .child(div().id("profile-menu").cursor_pointer().child(format!("{}  ⌄", self.profiles.current()["name"].as_str().unwrap_or("Profile")))
                        .on_click(cx.listener(|this, _, _, cx| { this.show_profiles = !this.show_profiles; cx.notify(); })))
                    .child(profile_menu))
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
        let connection = if self.profiles.current()["settings"]["mode"].as_str() == Some("24g") { "2.4G" } else { "USB" };
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
            .child(div().flex().h(px(185.0)).child(stat(p, "CONNECTION", connection.into(), if data.is_some() { "Receiver found".into() } else { "Not connected".into() }, false))
                .child(stat(p, "BATTERY", battery, battery_sub.into(), false)));
        div().flex().w_full().gap(px(12.0)).child(hero).child(stats)
    }

    fn dpi(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let data = self.readout.as_ref();
        let profile_colors: Vec<u32> = self.profiles.current()["dpiStages"].as_array().into_iter().flatten()
            .map(|stage| stage["color"].as_str().and_then(|color| color.strip_prefix('#'))
                .and_then(|color| u32::from_str_radix(color, 16).ok()).unwrap_or(p.muted)).collect();
        let stage_count = data.and_then(|d| d.dpi_count).unwrap_or(6);
        let dpi_editable = (self.demo || self.experimental_write) && data.is_some_and(|d| d.dpi_count.is_some() && d.dpi.len() == 6);
        let rate_editable = (self.demo || self.experimental_write) && data.and_then(|d| d.rate).is_some();
        let mut stage_rows = div().flex().flex_col().gap(px(8.0));
        for pair in [[Some(0), Some(1)], [Some(2), Some(3)], [Some(4), None], [Some(5), None]] {
            let mut row = div().flex().gap(px(8.0));
            for i in pair.into_iter().flatten() {
                if i >= stage_count { continue; }
                let value = self.dpi_drag.filter(|(stage, _)| *stage == i).map(|(_, value)| value)
                    .or_else(|| data.and_then(|d| d.dpi.get(i).copied())).map(|value| value.to_string()).unwrap_or_else(|| "—".into());
                let active = data.and_then(|d| d.active_dpi) == Some(i);
                let mut stage = div().id(("dpi-stage", i)).rounded(px(8.0)).bg(rgb(if active { p.accent_soft } else { p.inset })).px(px(10.0)).py(px(6.0)).flex().items_center().gap(px(8.0))
                    .child(div().w(px(28.0)).h(px(28.0)).rounded(px(6.0)).bg(rgb(profile_colors.get(i).copied().unwrap_or(p.muted))))
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
        let bounds = self.dpi_track_bounds.clone();
        let mut track = div().id("dpi-track").relative().w_full().h(px(66.0))
            .child(canvas(move |rect, _, _| { bounds.set(Some(rect)); }, |_, _, _, _| {}).absolute().size_full())
            .child(div().absolute().top(px(41.0)).w_full().h(px(2.0)).bg(rgb(p.muted)));
        if dpi_editable {
            track = track.cursor_pointer()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                    if let Some(bounds) = this.dpi_track_bounds.get() {
                        if event.position.y >= bounds.origin.y + px(30.0) { this.start_dpi_drag(event.position, cx); }
                    }
                }))
                .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                    if event.pressed_button == Some(MouseButton::Left) { this.move_dpi_drag(event.position, cx); }
                    else if this.dpi_drag.take().is_some() { cx.notify(); }
                }))
                .on_mouse_up(MouseButton::Left, cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| this.finish_dpi_drag(event.position, cx)))
                .on_mouse_up_out(MouseButton::Left, cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| this.finish_dpi_drag(event.position, cx)));
        }
        if let Some(data) = data {
            for (i, value) in data.dpi.iter().enumerate().take(stage_count) {
                let value = self.dpi_drag.filter(|(stage, _)| *stage == i).map(|(_, value)| value).unwrap_or(*value);
                let position = dpi_position(value);
                let active = data.active_dpi == Some(i);
                let color = profile_colors.get(i).copied().unwrap_or(p.accent);
                let mut number = div().id(("dpi-number", i)).absolute().left(relative(position)).top(px(0.0)).ml(px(-20.0))
                    .text_size(px(18.0)).font_weight(FontWeight::BOLD)
                    .text_color(rgb(if active { color } else { p.text })).child(value.to_string());
                let mut dot = div().id(("dpi-node", i)).absolute().left(relative(position)).top(px(36.0)).ml(px(-6.0)).size(px(12.0))
                    .rounded_full().bg(rgb(if active { color } else { p.text }));
                if dpi_editable {
                    number = number.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.select_dpi(i, cx)));
                    dot = dot.cursor_pointer().on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.stop_propagation()))
                        .on_click(cx.listener(move |this, _, _, cx| this.select_dpi(i, cx)));
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
        let selected_value = self.dpi_input.clone().or_else(|| self.dpi_drag.map(|(_, value)| value.to_string())).or_else(|| data.and_then(|d| d.active_dpi.and_then(|i| d.dpi.get(i)))
            .map(u16::to_string)).unwrap_or_else(|| "—".into());
        let selected_color = data.and_then(|d| d.active_dpi)
            .and_then(|i| profile_colors.get(i)).copied().unwrap_or(p.muted);
        let mut color_control = div().id("dpi-color").track_focus(&self.color_focus)
            .rounded(px(4.0)).bg(rgb(selected_color)).size(px(26.0));
        if self.profiles_writable && data.and_then(|d| d.active_dpi).is_some() {
            color_control = color_control.cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| this.edit_color(window, cx)))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| this.handle_color_key(event, cx)));
        }
        if let Some(value) = &self.color_input {
            color_control = color_control.w(px(90.0)).h(px(38.0)).px(px(6.0)).flex().items_center()
                .bg(rgb(p.inset)).border_1().border_color(rgb(p.accent))
                .font_family("JetBrains Mono").text_color(rgb(p.text)).child(value.clone());
        }
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
            .child(color_control);
        let mut apply_colors = div().id("apply-dpi-colors").rounded(px(7.0)).px(px(8.0)).py(px(5.0))
            .text_size(px(11.0)).text_color(rgb(p.muted)).child("Apply profile colors");
        if (self.demo || self.experimental_write) && self.profiles_writable && data.is_some() {
            apply_colors = apply_colors.cursor_pointer().bg(rgb(p.inset)).text_color(rgb(p.text))
                .on_click(cx.listener(|this, _, _, cx| this.apply_profile_colors(cx)));
        }
        let hero = card(p).flex_1().min_w(px(0.0)).h(px(520.0)).px(px(36.0)).py(px(28.0)).flex().flex_col()
            .child(div().flex().items_center().justify_between()
                .child(div().flex().items_center().gap(px(20.0)).child(heading("DPI Sensitivity")).child(levels))
                .child(div().flex().items_center().gap(px(8.0)).child(apply_colors).child(muted(p, "Reset DPI"))))
            .child(chart)
            .child(div().mt(px(25.0)).flex().items_center().justify_between().gap(px(20.0))
                .child(stage_rows)
                .child(fine_controls))
            .child(muted(p, "Apply profile colors replaces all six colors on the mouse."));
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
        let mut reset = div().id("reset-keys").rounded(px(6.0)).px(px(8.0)).py(px(6.0)).text_size(px(12.0))
            .text_color(rgb(p.muted)).child(if self.confirm_reset_keys { "Confirm reset" } else { "Reset all keys" });
        if (self.demo || self.experimental_write) && keys.is_some() && !self.scanning && !self.applying {
            reset = reset.cursor_pointer().text_color(rgb(p.accent)).on_click(cx.listener(|this, _, _, cx| {
                if this.confirm_reset_keys { this.reset_keys(cx); } else { this.confirm_reset_keys = true; cx.notify(); }
            }));
        }
        let mut reset_actions = div().flex().items_center().child(reset);
        if self.confirm_reset_keys {
            reset_actions = reset_actions.child(div().id("cancel-reset-keys").cursor_pointer().px(px(8.0)).text_size(px(12.0)).child("Cancel")
                .on_click(cx.listener(|this, _, _, cx| { this.confirm_reset_keys = false; cx.notify(); })));
        }
        let left = div().flex_1().h(px(620.0)).flex().flex_col()
            .child(div().flex().justify_between().child(div().child(heading("Mouse Buttons")).child(muted(p, "Click a callout to rebind. Grayed options are not available yet."))).child(reset_actions))
            .child(mouse);
        let groups: [(&str, &[&str]); 4] = [
            ("MOUSE FEATURES", &["Left-Click", "Right-Click", "Middle-Click", "Forward", "Backward", "Double-Click", "Scroll Up", "Scroll Down", "DPI Loop", "Fire button", "Disable Key"]),
            ("MULTIMEDIA", &["Volume +", "Volume -", "Mute", "Play/Pause", "Previous", "Next", "Stop"]),
            ("SYSTEM SHORTCUT", &["Calculator", "My Computer", "Mail", "Refresh (F5)", "Switch Application", "Copy", "Paste", "Screen Brightness +", "Screen Brightness -"]),
            ("LIGHTING / MACRO", &["Lighting on/off", "Macro"]),
        ];
        let mut functions = div().id("key-functions").flex_1().min_h(px(0.0)).overflow_y_scroll().flex().flex_col().gap(px(14.0));
        for (group_index, (title, items)) in groups.into_iter().enumerate() {
            let mut group = div().flex().flex_col().gap(px(6.0)).child(muted(p, title));
            for (row_index, pair) in items.chunks(2).enumerate() {
                let mut row = div().flex().gap(px(6.0));
                for (item_index, label) in pair.iter().enumerate() {
                    let label = *label;
                    let enabled = (self.demo || self.experimental_write) && keys.is_some() && self.selected_key != 0 && device::key_wire(label).is_some();
                    let selected = keys.and_then(|keys| device::key_wire(label).map(|wire| keys[self.selected_key] == wire)).unwrap_or(false);
                    let mut button = div().id(("key-function", group_index * 100 + row_index * 2 + item_index)).flex_1().rounded(px(8.0)).px(px(12.0)).py(px(11.0))
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

    fn lighting(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let image = div().w(px(410.0)).h(px(560.0)).flex().flex_col().items_center().justify_center()
            .child(img(self.mouse.clone()).w(px(280.0)).h(px(500.0)).object_fit(ObjectFit::Contain))
            .child(div().mt(px(18.0)).child(muted(p, "Live preview of the selected effect")));
        let mut effects = div().flex().flex_col().gap(px(8.0));
        let lighting = self.readout.as_ref().and_then(|d| d.lighting.as_ref());
        let light_editable = (self.demo || self.experimental_write) && lighting.is_some();
        for (row_index, row) in [["Flowing light", "Breathing", "Constant light"], ["Neon", "Colorful waves", "Close"]].into_iter().enumerate() {
            let mut line = div().flex().gap(px(8.0));
            for (item_index, item) in row.into_iter().enumerate() {
                let selected = lighting.is_some_and(|light| matches!((item, light.mode), ("Breathing", LightMode::Breathing) | ("Constant light", LightMode::Constant) | ("Close", LightMode::Off)));
                let mode = match item { "Breathing" => Some(LightMode::Breathing), "Constant light" => Some(LightMode::Constant), "Close" => Some(LightMode::Off), _ => None };
                let mut button = div().id(("light-mode", row_index * 3 + item_index)).flex_1().rounded(px(10.0)).bg(rgb(if selected { p.accent_soft } else { p.inset }))
                    .text_color(rgb(if selected { p.accent } else if mode.is_some() { p.text } else { p.muted })).px(px(12.0)).py(px(16.0)).text_center().child(item);
                if let Some(mode) = mode.filter(|_| (self.demo || self.experimental_write) && lighting.is_some()) {
                    button = button.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.update_lighting(LightChange::Mode(mode), cx)));
                }
                line = line.child(button);
            }
            effects = effects.child(line);
        }
        let brightness = self.light_slider(p, cx, false, lighting.map(|light| light.brightness),
            light_editable && lighting.is_some_and(|light| light.mode != LightMode::Off));
        let speed = self.light_slider(p, cx, true, lighting.map(|light| light.speed),
            light_editable && lighting.is_some_and(|light| light.mode == LightMode::Breathing));
        let mut card = card(p).flex_1().flex().flex_col().gap(px(14.0))
            .child(div().flex().justify_between().child(div().child(heading("Lighting Effect"))
                .child(muted(p, "Constant mode color = active DPI stage color (change colors on the DPI tab)")))
                .child(muted(p, "Reset lighting")))
            .child(effects)
            .child(muted(p, if self.demo || self.experimental_write { "Select an effect. Live changes require a readable current color." } else { "Lighting controls are not available in the preview." }))
            .child(brightness);
        if lighting.is_some_and(|light| light.mode == LightMode::Breathing) { card = card.child(speed); }
        let current_color = lighting.and_then(|light| light.color).map(|[r, g, b]| (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b));
        let swatch = div().w(px(48.0)).h(px(36.0)).rounded(px(7.0)).bg(rgb(p.inset)).p(px(5.0))
            .child(div().size_full().rounded(px(3.0)).bg(rgb(current_color.unwrap_or(p.muted))));
        card = card.child(div().flex().items_center().gap(px(12.0)).text_size(px(12.0)).text_color(rgb(p.muted))
            .child(div().w(px(100.0)).child("Color")).child(swatch));
        div().flex().gap(px(16.0)).items_start().child(image).child(card)
    }

    fn settings(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let local = &self.profiles.current()["settings"];
        let high_speed = local["highSpeed"].as_bool().unwrap_or(false);
        let fine_dpi = local["fineDpi"].as_bool().unwrap_or(false);
        let mode_24g = local["mode"].as_str() == Some("24g");
        let sensor = self.readout.as_ref().and_then(|data| data.sensor.as_ref());
        let power = self.profiles.power();
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
        let debounce = power.as_ref().map(|power| format!("{} ms", power.debounce)).unwrap_or_else(|| "—".into());
        let mut debounce_control = div().flex().gap(px(6.0)).items_center()
            .child(div().id("debounce-down").rounded(px(6.0)).bg(rgb(p.inset)).px(px(10.0)).py(px(6.0)).child("−")
                .on_click(cx.listener(|this, _, _, cx| { if let Some(mut power) = this.profiles.power() { power.debounce = power.debounce.saturating_sub(1); this.update_power(power, cx); } })))
            .child(div().min_w(px(40.0)).text_center().child(debounce))
            .child(div().id("debounce-up").rounded(px(6.0)).bg(rgb(p.inset)).px(px(10.0)).py(px(6.0)).child("+")
                .on_click(cx.listener(|this, _, _, cx| { if let Some(mut power) = this.profiles.power() { power.debounce = power.debounce.saturating_add(1).min(30); this.update_power(power, cx); } })));
        if !(self.demo || self.experimental_write) || power.is_none() { debounce_control = debounce_control.text_color(rgb(p.muted)); }
        let mouse = card(p).flex_1().child(heading("Mouse Settings"))
            .child(lod_field).child(angle_field).child(ripple_field)
            .child(div().flex().justify_between().items_center().min_h(px(74.0))
                .child(div().flex().flex_col().child("Debounce Time").child(muted(p, "ms · higher reduces double-clicks")))
                .child(debounce_control))
            .child(field(p, "High-Speed Mode", "Local only", if high_speed { "On" } else { "Off" }).id("local-high-speed").cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.update_local_setting("highSpeed", serde_json::json!(!high_speed), cx))));
        let sleep_label = power.as_ref().map(|power| match power.sleep_wire { 1 => "10 sec", 3 => "30 sec", 6 => "1 min", 12 => "2 min", 30 => "5 min", 60 => "10 min", 120 => "20 min", 180 => "30 min", _ => "—" }).unwrap_or("—");
        let wake = power.as_ref().map(|power| if power.move_wake { "On" } else { "Off" }).unwrap_or("—");
        let off_light = power.as_ref().map(|power| if power.move_close_light { "On" } else { "Off" }).unwrap_or("—");
        let mut sleep_field = field(p, "Sleep after", "Inactivity timeout", sleep_label).id("power-sleep");
        let mut wake_field = field(p, "Move wake-up", "", wake).id("power-wake");
        let mut light_field = field(p, "Move to turn off light", "", off_light).id("power-move-light");
        if power.is_some() && (self.demo || self.experimental_write) {
            sleep_field = sleep_field.cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                if let Some(mut power) = this.profiles.power() {
                    let options = [1, 3, 6, 12, 30, 60, 120, 180];
                    let index = options.iter().position(|value| *value == power.sleep_wire).unwrap_or(0);
                    power.sleep_wire = options[(index + 1) % options.len()]; this.update_power(power, cx);
                }
            }));
            wake_field = wake_field.cursor_pointer().on_click(cx.listener(|this, _, _, cx| { if let Some(mut power) = this.profiles.power() { power.move_wake = !power.move_wake; this.update_power(power, cx); } }));
            light_field = light_field.cursor_pointer().on_click(cx.listener(|this, _, _, cx| { if let Some(mut power) = this.profiles.power() { power.move_close_light = !power.move_close_light; this.update_power(power, cx); } }));
        }
        let power = card(p).flex_1().child(heading("Power & Sleep"))
            .child(sleep_field).child(wake_field).child(light_field)
            .child(field(p, "Connection mode (display)", "UI only", if mode_24g { "2.4G" } else { "USB" }).id("local-mode").cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.update_local_setting("mode", serde_json::json!(if mode_24g { "usb" } else { "24g" }), cx))));
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
            .child(div().flex().justify_between().items_center().gap(px(12.0)).min_h(px(74.0))
                .child(div().flex().flex_col().gap(px(3.0)).child(div().font_weight(FontWeight::MEDIUM).child("Profile backup"))
                    .child(muted(p, "Save or load this profile as a JSON file")))
                .child(div().flex().gap(px(8.0))
                    .child(div().id("profile-export").cursor_pointer().rounded(px(8.0)).bg(rgb(p.inset)).px(px(12.0)).py(px(8.0)).child("Export")
                        .on_click(cx.listener(|this, _, _, cx| this.export_profile(cx))))
                    .child(div().id("profile-import").cursor_pointer().rounded(px(8.0)).bg(rgb(p.inset)).px(px(12.0)).py(px(8.0)).child("Import")
                        .on_click(cx.listener(|this, _, _, cx| this.import_profile(cx))))))
            .child(muted(p, "Restore factory settings"));
        let experimental = card(p).mt(px(12.0)).child(heading("Experimental Features"))
            .child(muted(p, "Software-level unlocks that bypass OEM app limitations."))
            .child(field(p, "50-Step DPI Granularity", "Unlock 50-step increments (default is 100)", if fine_dpi { "On" } else { "Off" }).id("local-fine-dpi").cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.update_local_setting("fineDpi", serde_json::json!(!fine_dpi), cx))));
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
            0 => self.home(p), 1 => self.keys(p, cx), 2 => self.dpi(p, cx), 3 => self.lighting(p, cx),
            4 => div().h(px(440.0)).flex().justify_center().items_center().text_color(rgb(p.muted)).child("Macros are not available in this version."),
            5 => self.settings(p, cx), _ => self.about(p),
        };
        let notice = if self.dpi_drag.is_some() { "Release to apply the DPI value.".to_string() }
            else if self.color_input.is_some() { "Enter a profile color as #RRGGBB, then press Enter to save or Escape to cancel. This does not change the mouse.".to_string() }
            else if self.dpi_input.is_some() { "Enter a DPI value, then press Enter to apply or Escape to cancel.".to_string() }
            else if self.demo { "Sample data · The receiver is not accessed in demo mode.".to_string() }
            else if self.scanning { "Reading the receiver…".to_string() }
            else if let Some(error) = &self.error { format!("{error}. Close other mouse software, then refresh.") }
            else if let Some(message) = &self.message { message.clone() }
            else if self.experimental_write { "Experimental mode · DPI, report rate, and sensor controls can change the receiver. Close other mouse software.".into() }
            else { "GPUI preview · Settings are read-only and may be unavailable.".to_string() };
        div().size_full().bg(rgb(p.bg)).text_color(rgb(p.text)).font_family("Outfit").text_size(px(14.0))
            .flex().flex_col().px(px(20.0)).pt(px(8.0))
            .child(self.header(p, cx))
            .child(div().id("content").track_scroll(&self.content_scroll).flex_1().min_h(px(0.0)).overflow_y_scroll().pt(px(5.0)).child(page))
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
        let on = value == "On";
        let switch = div().w(px(48.0)).h(px(28.0)).rounded_full().bg(rgb(if on { p.accent } else { p.hover }))
            .flex().items_center().px(px(3.0));
        (if on { switch.justify_end() } else { switch.justify_start() })
            .child(div().size(px(22.0)).rounded_full().bg(rgb(p.card)))
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

fn dpi_from_position(position: f32, step: u16) -> u16 {
    let min = 100.0f32.ln();
    let raw = (min + position.clamp(0.0, 1.0) * (26000.0f32.ln() - min)).exp();
    ((raw / f32::from(step)).round() * f32::from(step)).clamp(50.0, 26000.0) as u16
}

fn light_slider_value(position: f32, speed: bool) -> u8 {
    let fraction = position.clamp(0.0, 1.0);
    if speed { 1 + (fraction * 9.0).round() as u8 } else { (fraction * 4.0).round() as u8 * 25 }
}

fn next_reported_speed(current: u8, increasing: bool) -> u8 {
    let speeds = [1, 3, 6, 8, 10];
    if increasing { speeds.into_iter().find(|value| *value > current).unwrap_or(10) }
    else { speeds.into_iter().rev().find(|value| *value < current).unwrap_or(1) }
}

#[cfg(test)]
mod dpi_track_tests {
    use super::*;

    #[test]
    fn track_uses_logarithmic_scale_and_respects_step_and_limits() {
        assert_eq!(dpi_from_position(-1.0, 100), 100);
        assert_eq!(dpi_from_position(2.0, 100), 26000);
        assert_eq!(dpi_from_position(dpi_position(2400), 100), 2400);
        assert_eq!(dpi_from_position(dpi_position(2450), 50), 2450);
    }

    #[test]
    fn lighting_sliders_use_original_ranges_and_brightness_steps() {
        assert_eq!(light_slider_value(-1.0, false), 0);
        assert_eq!(light_slider_value(0.5, false), 50);
        assert_eq!(light_slider_value(2.0, false), 100);
        assert_eq!(light_slider_value(0.0, true), 1);
        assert_eq!(light_slider_value(0.5, true), 6);
        assert_eq!(light_slider_value(1.0, true), 10);
        assert_eq!(next_reported_speed(3, true), 6);
        assert_eq!(next_reported_speed(6, false), 3);
        assert_eq!(next_reported_speed(10, true), 10);
    }
}

