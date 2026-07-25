// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

#[derive(Serialize, Clone)]
struct NativeHidDevice {
    vendor_id: u16,
    product_id: u16,
    product_name: String,
    path: String,
    interface_number: i32,
    usage_page: u16,
    usage: u16,
}

const VIDS: &[u16] = &[0x1D57, 0x248A, 0x249A, 0x258A, 0x1A2C, 0x3554];

// Channel-based HID worker to avoid Send issues with hidapi::HidDevice
enum HidCmd {
    Open(String, mpsc::Sender<Result<bool, String>>),
    Write(Vec<u8>, mpsc::Sender<Result<bool, String>>),
    Read(i32, mpsc::Sender<Result<Vec<u8>, String>>),
    Close(mpsc::Sender<Result<bool, String>>),
}

static HID_TX: Mutex<Option<mpsc::Sender<HidCmd>>> = Mutex::new(None);

fn ensure_hid_worker() {
    let mut guard = HID_TX.lock().unwrap();
    if guard.is_some() {
        return;
    }
    let (tx, rx) = mpsc::channel::<HidCmd>();
    *guard = Some(tx);

    thread::spawn(move || {
        let mut api = hidapi::HidApi::new().expect("Failed to init HidApi");
        let mut device: Option<hidapi::HidDevice> = None;

        for cmd in rx {
            match cmd {
                HidCmd::Open(path, reply) => {
                    let path_c = match std::ffi::CString::new(path) {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = reply.send(Err(e.to_string()));
                            continue;
                        }
                    };
                    // Refresh device list before opening
                    let _ = api.refresh_devices();
                    match api.open_path(&path_c) {
                        Ok(dev) => {
                            device = Some(dev);
                            let _ = reply.send(Ok(true));
                        }
                        Err(e) => {
                            let _ = reply.send(Err(e.to_string()));
                        }
                    }
                }
                HidCmd::Write(buf, reply) => {
                    if let Some(ref dev) = device {
                        match dev.write(&buf) {
                            Ok(_) => {
                                let _ = reply.send(Ok(true));
                            }
                            Err(e) => {
                                let _ = reply.send(Err(e.to_string()));
                            }
                        }
                    } else {
                        let _ = reply.send(Err("No HID device opened".into()));
                    }
                }
                HidCmd::Read(timeout_ms, reply) => {
                    if let Some(ref dev) = device {
                        let mut buf = [0u8; 65];
                        match dev.read_timeout(&mut buf, timeout_ms) {
                            Ok(n) => {
                                let _ = reply.send(Ok(buf[..n].to_vec()));
                            }
                            Err(e) => {
                                let _ = reply.send(Err(e.to_string()));
                            }
                        }
                    } else {
                        let _ = reply.send(Err("No HID device opened".into()));
                    }
                }
                HidCmd::Close(reply) => {
                    device = None;
                    let _ = reply.send(Ok(true));
                }
            }
        }
    });
}

fn hid_send(cmd: HidCmd) {
    ensure_hid_worker();
    let guard = HID_TX.lock().unwrap();
    if let Some(ref tx) = *guard {
        let _ = tx.send(cmd);
    }
}

#[cfg(target_os = "windows")]
fn apply_window_theme(window: &tauri::WebviewWindow, theme: &str) {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
    };

    if let Ok(hwnd) = window.hwnd() {
        let hwnd = hwnd.0 as HWND;
        let dark = theme == "dark";
        let use_dark: i32 = if dark { 1 } else { 0 };
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                &use_dark as *const _ as *const c_void,
                std::mem::size_of::<i32>() as u32,
            );

            // COLORREF in Win32: 0x00BBGGRR
            // Light background (#f4f4f6) -> BGR 0x00F6F4F4
            // Dark background (#0e0e11) -> BGR 0x00110E0E
            let color: u32 = if dark { 0x00110E0E } else { 0x00F6F4F4 };
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_CAPTION_COLOR as u32,
                &color as *const _ as *const c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

#[tauri::command]
fn set_window_theme(window: tauri::WebviewWindow, theme: String) {
    #[cfg(target_os = "windows")]
    {
        apply_window_theme(&window, &theme);
    }
    let _ = window.set_theme(if theme == "dark" {
        Some(tauri::Theme::Dark)
    } else {
        Some(tauri::Theme::Light)
    });
}

#[tauri::command]
fn list_hid_devices() -> Result<Vec<NativeHidDevice>, String> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for dev in api.device_list() {
        if VIDS.contains(&dev.vendor_id()) {
            let usage_page = dev.usage_page();
            let usage = dev.usage();
            // Filter out pure mouse (1, 2), keyboard (1, 6), and consumer (0x0c)
            if usage_page == 0x01 && (usage == 0x02 || usage == 0x06) {
                continue;
            }
            if usage_page == 0x0c {
                continue;
            }

            let name = dev
                .product_string()
                .unwrap_or("Wireless-Receiver")
                .to_string();
            let path = dev.path().to_string_lossy().to_string();
            list.push(NativeHidDevice {
                vendor_id: dev.vendor_id(),
                product_id: dev.product_id(),
                product_name: name,
                path,
                interface_number: dev.interface_number(),
                usage_page,
                usage,
            });
        }
    }
    Ok(list)
}

#[tauri::command]
fn open_hid_device(path: String) -> Result<bool, String> {
    let (tx, rx) = mpsc::channel();
    hid_send(HidCmd::Open(path, tx));
    rx.recv().map_err(|e| e.to_string())?
}

#[tauri::command]
fn close_hid_device() -> Result<bool, String> {
    let (tx, rx) = mpsc::channel();
    hid_send(HidCmd::Close(tx));
    rx.recv().map_err(|e| e.to_string())?
}

#[tauri::command]
fn send_hid_report(report_id: u8, data: Vec<u8>) -> Result<bool, String> {
    let mut buf = Vec::with_capacity(data.len() + 1);
    buf.push(report_id);
    buf.extend_from_slice(&data);
    let (tx, rx) = mpsc::channel();
    hid_send(HidCmd::Write(buf, tx));
    rx.recv().map_err(|e| e.to_string())?
}

#[tauri::command]
fn read_hid_report(timeout_ms: i32) -> Result<Vec<u8>, String> {
    let (tx, rx) = mpsc::channel();
    hid_send(HidCmd::Read(timeout_ms, tx));
    rx.recv().map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::AppleScript,
            Some(vec!["--autostart"]),
        ))
        .invoke_handler(tauri::generate_handler![
            set_window_theme,
            list_hid_devices,
            open_hid_device,
            close_hid_device,
            send_hid_report,
            read_hid_report
        ])
        .setup(|app| {
            // Build Tray Menu
            let toggle = MenuItem::with_id(app, "toggle", "Show/Hide Nibble", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Nibble", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &quit])?;

            // Build Tray Icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                {
                    apply_window_theme(&window, "light");
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent app from quitting when window X is clicked; hide to system tray instead!
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Nibble desktop application");
}
