use crate::device::{self, LightMode, PowerSettings, Readout};
use serde_json::{Value, json};
use std::{fs, io::{self, Write}, path::{Path, PathBuf}};

const DEFAULT_COLORS: [&str; 6] = ["#FF0000", "#00FF00", "#0000FF", "#00FFFF", "#FFFF00", "#800080"];
const DEFAULT_DPI: [u16; 6] = [400, 800, 1200, 1600, 2400, 3200];

pub fn default_profile() -> Value {
    let stages: Vec<_> = DEFAULT_DPI.into_iter().zip(DEFAULT_COLORS).map(|(value, color)| json!({"value":value,"color":color})).collect();
    json!({
        "name":"Default Profile", "deviceId":"aj179", "activeDpi":1, "dpiStageCount":6,
        "dpiStages":stages, "reportRateIndex":3,
        "light":{"mode":"breathe","brightness":100,"speed":5,"color":"#00FF00"},
        "keys":{"201":"left","202":"right","203":"middle","204":"forward","205":"backward","206":"dpi_loop"},
        "settings":{"lod":"low","angleSnap":false,"ripple":false,"sleepWire":30,"moveWake":true,"moveCloseLight":false,"debounce":4,"lmbLock":true,"fineDpi":false}
    })
}

#[derive(Clone, Debug)]
pub struct Profiles { document: Value }

#[derive(Clone, Copy)]
pub enum ReadoutScope {
    DpiValue(usize), DpiSelection, DpiCount, Rate,
    Sensor(device::SensorChange), Key(usize), Keys, Lighting(device::LightChange),
}

impl Profiles {
    pub fn new() -> Self { Self { document: json!({"profileIndex":0,"profiles":[default_profile()]}) } }

    pub fn from_json(input: &str) -> Result<Self, String> {
        let document: Value = serde_json::from_str(input).map_err(|e| format!("Invalid profile JSON: {e}"))?;
        validate_document(&document)?;
        Ok(Self { document })
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(contents) => Self::from_json(&contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::new()),
            Err(error) => Err(format!("Could not read saved profiles: {error}")),
        }
    }

    pub fn current(&self) -> &Value {
        &self.document["profiles"][self.document["profileIndex"].as_u64().unwrap_or(0) as usize]
    }

    pub fn names(&self) -> Vec<&str> {
        self.document["profiles"].as_array().into_iter().flatten()
            .filter_map(|profile| profile["name"].as_str()).collect()
    }

    pub fn select(&mut self, index: usize) -> Result<(), String> {
        if index >= self.names().len() { return Err("Profile selection is invalid".into()); }
        self.document["profileIndex"] = json!(index);
        Ok(())
    }

    pub fn set_local_setting(&mut self, key: &str, value: Value) -> Result<(), String> {
        match key {
            "fineDpi" | "highSpeed" | "lmbLock" if value.is_boolean() => (),
            "mode" if value.as_str().is_some_and(|mode| mode == "usb" || mode == "24g") => (),
            _ => return Err("Invalid local setting".into()),
        }
        let index = self.document["profileIndex"].as_u64().unwrap_or(0) as usize;
        self.document["profiles"][index]["settings"][key] = value;
        Ok(())
    }

    pub fn set_dpi_color(&mut self, stage: usize, color: &str) -> Result<(), String> {
        let bytes = color.as_bytes();
        if stage >= 6 || bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
            return Err("Enter a color in #RRGGBB format".into());
        }
        let index = self.document["profileIndex"].as_u64().unwrap_or(0) as usize;
        self.document["profiles"][index]["dpiStages"][stage]["color"] = json!(color.to_ascii_uppercase());
        Ok(())
    }

    pub fn dpi_colors(&self) -> Result<[[u8; 3]; 6], String> {
        let stages = self.current()["dpiStages"].as_array().ok_or("DPI colors are missing")?;
        if stages.len() != 6 { return Err("DPI colors are incomplete".into()); }
        let mut colors = [[0u8; 3]; 6];
        for (stage, target) in stages.iter().zip(&mut colors) {
            let color = stage["color"].as_str().ok_or("DPI color is missing")?;
            let bytes = color.as_bytes();
            if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
                return Err("DPI color is invalid".into());
            }
            for (channel, part) in target.iter_mut().zip(bytes[1..].chunks_exact(2)) {
                let hex = std::str::from_utf8(part).map_err(|_| "DPI color is invalid")?;
                *channel = u8::from_str_radix(hex, 16).map_err(|_| "DPI color is invalid")?;
            }
        }
        Ok(colors)
    }

    pub fn power(&self) -> Option<PowerSettings> {
        let settings = &self.current()["settings"];
        let raw = match settings.get("sleepWire").filter(|value| !value.is_null()) {
            Some(value) => value.as_u64()?,
            None => settings.get("sleepMin")?.as_u64()?.checked_mul(6)?,
        };
        let sleep = if [1, 3, 6, 12, 30, 60, 120, 180].contains(&raw) { raw }
            else if (1..=30).contains(&raw) && [1, 3, 6, 12, 30, 60, 120, 180].contains(&(raw * 6)) { raw * 6 }
            else { return None; };
        let debounce = settings["debounce"].as_u64()?;
        if debounce > 30 { return None; }
        Some(PowerSettings { sleep_wire: sleep as u8, move_wake: settings["moveWake"].as_bool()?,
            move_close_light: settings["moveCloseLight"].as_bool()?, debounce: debounce as u8 })
    }

    pub fn set_power(&mut self, power: &PowerSettings) -> Result<(), String> {
        if ![1, 3, 6, 12, 30, 60, 120, 180].contains(&power.sleep_wire) || power.debounce > 30 {
            return Err("Invalid power settings".into());
        }
        let index = self.document["profileIndex"].as_u64().unwrap_or(0) as usize;
        let settings = &mut self.document["profiles"][index]["settings"];
        settings["sleepWire"] = json!(power.sleep_wire); settings["moveWake"] = json!(power.move_wake);
        settings["moveCloseLight"] = json!(power.move_close_light); settings["debounce"] = json!(power.debounce);
        if let Some(settings) = settings.as_object_mut() { settings.remove("sleepMin"); }
        Ok(())
    }

    pub fn sync_readout(&mut self, data: &Readout) -> Result<(), String> {
        let index = self.document["profileIndex"].as_u64().unwrap_or(0) as usize;
        let current = &mut self.document["profiles"][index];
        if data.dpi.len() == 6 && data.dpi.iter().all(|value| (50..=26000).contains(value) && value % 50 == 0) {
            for (stage, value) in current["dpiStages"].as_array_mut().ok_or("DPI stages are missing")?.iter_mut().zip(&data.dpi) {
                stage["value"] = json!(value);
            }
            if let (Some(active), Some(count)) = (data.active_dpi, data.dpi_count) {
                if (1..=6).contains(&count) && active < count {
                    current["activeDpi"] = json!(active);
                    current["dpiStageCount"] = json!(count);
                }
            }
        }
        if let Some(index) = data.rate.and_then(|rate| match rate { 125 => Some(0), 250 => Some(1), 500 => Some(2), 1000 => Some(3), _ => None }) {
            current["reportRateIndex"] = json!(index);
        }
        if let Some(sensor) = &data.sensor {
            current["settings"]["lod"] = json!(if sensor.lod == 2 { "high" } else { "low" });
            current["settings"]["angleSnap"] = json!(sensor.angle_snap);
            current["settings"]["ripple"] = json!(sensor.ripple);
        }
        if let Some(keys) = &data.keys {
            for (slot, key) in keys.iter().enumerate() {
                let id = match device::key_label(*key) {
                    "Left-Click" => Some("left"), "Right-Click" => Some("right"), "Middle-Click" => Some("middle"),
                    "Backward" => Some("backward"), "Forward" => Some("forward"), "Double-Click" => Some("double"),
                    "Scroll Up" => Some("scroll_up"), "Scroll Down" => Some("scroll_down"), "DPI Loop" => Some("dpi_loop"),
                    "Disable Key" => Some("disable"), "Lighting on/off" => Some("light_toggle"),
                    "Volume +" => Some("vol_up"), "Volume -" => Some("vol_down"), "Mute" => Some("mute"),
                    "Play/Pause" => Some("play"), "Previous" => Some("prev"), "Next" => Some("next"), "Stop" => Some("stop"),
                    "Calculator" => Some("calc"), "My Computer" => Some("computer"), "Mail" => Some("mail"),
                    "Screen Brightness +" => Some("brightness_up"), "Screen Brightness -" => Some("brightness_down"),
                    "Refresh (F5)" => Some("refresh"), "Switch Application" => Some("switch_app"), "Copy" => Some("copy"), "Paste" => Some("paste"),
                    _ => None,
                };
                // Device order is left, right, middle, back, forward, DPI.
                if let Some(id) = id { current["keys"][["201", "202", "203", "205", "204", "206"][slot]] = json!(id); }
            }
        }
        if let Some(light) = &data.lighting {
            current["light"]["mode"] = json!(match light.mode { LightMode::Off => "off", LightMode::Breathing => "breathe", LightMode::Constant => "solid" });
            current["light"]["brightness"] = json!(light.brightness);
            current["light"]["speed"] = json!(light.speed);
            if let Some([r, g, b]) = light.color.filter(|color| *color != [0, 0, 0]) {
                current["light"]["color"] = json!(format!("#{r:02X}{g:02X}{b:02X}"));
            }
        }
        validate_profile(current)
    }

    pub fn sync_scope(&mut self, data: &Readout, scope: ReadoutScope) -> Result<(), String> {
        let mut selected = Readout::default();
        match scope {
            ReadoutScope::DpiValue(_) | ReadoutScope::DpiSelection | ReadoutScope::DpiCount => { selected.dpi = data.dpi.clone(); selected.active_dpi = data.active_dpi; selected.dpi_count = data.dpi_count; }
            ReadoutScope::Rate => selected.rate = data.rate,
            ReadoutScope::Sensor(_) => selected.sensor = data.sensor.clone(),
            ReadoutScope::Key(_) | ReadoutScope::Keys => selected.keys = data.keys,
            ReadoutScope::Lighting(_) => selected.lighting = data.lighting.clone(),
        }
        let mut observed = self.clone();
        observed.sync_readout(&selected)?;
        let index = self.document["profileIndex"].as_u64().unwrap_or(0) as usize;
        let mut updated = self.document["profiles"][index].clone();
        let target = &mut updated;
        let source = observed.current();
        match scope {
            ReadoutScope::DpiValue(stage) if stage < 6 => target["dpiStages"][stage]["value"] = source["dpiStages"][stage]["value"].clone(),
            ReadoutScope::DpiValue(_) => return Err("Invalid DPI stage".into()),
            ReadoutScope::DpiSelection => {
                let active = source["activeDpi"].as_u64().ok_or("Active DPI stage is missing")?;
                let count = target["dpiStageCount"].as_u64().unwrap_or(6).max(active + 1);
                target["activeDpi"] = json!(active);
                target["dpiStageCount"] = json!(count);
            }
            ReadoutScope::DpiCount => {
                target["dpiStageCount"] = source["dpiStageCount"].clone();
                target["activeDpi"] = source["activeDpi"].clone();
            }
            ReadoutScope::Rate => target["reportRateIndex"] = source["reportRateIndex"].clone(),
            ReadoutScope::Sensor(change) => {
                let key = match change { device::SensorChange::Lod(_) => "lod", device::SensorChange::AngleSnap(_) => "angleSnap", device::SensorChange::Ripple(_) => "ripple" };
                target["settings"][key] = source["settings"][key].clone();
            }
            ReadoutScope::Key(slot) => {
                let key = ["201", "202", "203", "205", "204", "206"].get(slot).ok_or("Invalid key slot")?;
                target["keys"][*key] = source["keys"][*key].clone();
            }
            ReadoutScope::Keys => target["keys"] = source["keys"].clone(),
            ReadoutScope::Lighting(change) => {
                let key = match change { device::LightChange::Mode(_) => "mode", device::LightChange::Brightness(_) => "brightness", device::LightChange::Speed(_) => "speed" };
                target["light"][key] = source["light"][key].clone();
            }
        }
        validate_profile(target)?;
        self.document["profiles"][index] = updated;
        Ok(())
    }

    pub fn import(&mut self, input: &str) -> Result<(), String> {
        let value: Value = serde_json::from_str(input).map_err(|e| format!("Invalid profile JSON: {e}"))?;
        let candidate = value.get("profile").unwrap_or(&value).clone();
        validate_profile(&candidate)?;
        let profiles = self.document["profiles"].as_array_mut().ok_or("Invalid profile list")?;
        profiles.push(candidate);
        let index = profiles.len() - 1;
        self.document["profileIndex"] = json!(index);
        Ok(())
    }

    pub fn export_current(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&json!({"app":"Nibble","version":"1.0.6","profile":self.current()})).map_err(|e| e.to_string())
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        validate_document(&self.document)?;
        let parent = path.parent().ok_or("Invalid profile path")?;
        fs::create_dir_all(parent).map_err(|e| format!("Could not create profile folder: {e}"))?;
        let data = serde_json::to_vec_pretty(&self.document).map_err(|e| e.to_string())?;
        // A temporary file is placed beside the destination so a completed write
        // is available before moving the previous copy aside.
        let temporary = path.with_extension("json.pending");
        let backup = path.with_extension("json.backup");
        if temporary.exists() || backup.exists() { return Err("A profile recovery file exists. Keep it and resolve it before saving.".into()); }
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)
            .map_err(|e| format!("Could not create profile recovery file: {e}"))?;
        file.write_all(&data).and_then(|_| file.sync_all()).map_err(|e| format!("Could not write profiles: {e}"))?;
        drop(file);
        let existed = path.exists();
        if existed { fs::rename(path, &backup).map_err(|e| format!("Could not protect existing profiles: {e}"))?; }
        if let Err(error) = fs::rename(&temporary, path) {
            if existed { let _ = fs::rename(&backup, path); }
            return Err(format!("Could not replace profiles: {error}"));
        }
        if existed { fs::remove_file(&backup).map_err(|e| format!("Profiles saved, but backup cleanup failed: {e}"))?; }
        Ok(())
    }
}

pub fn profile_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|path| PathBuf::from(path).join("Nibble-gpui").join("profiles.json"))
}

fn validate_document(document: &Value) -> Result<(), String> {
    let profiles = document.get("profiles").and_then(Value::as_array).ok_or("Profile list is missing")?;
    if profiles.is_empty() { return Err("Profile list is empty".into()); }
    let index = document.get("profileIndex").and_then(Value::as_u64).ok_or("Profile selection is missing")?;
    if index >= profiles.len() as u64 { return Err("Profile selection is invalid".into()); }
    for profile in profiles { validate_profile(profile)?; }
    Ok(())
}

fn validate_profile(profile: &Value) -> Result<(), String> {
    let object = profile.as_object().ok_or("Profile must be an object")?;
    if object.get("name").and_then(Value::as_str).is_none_or(|name| name.trim().is_empty() || name.len() > 120) { return Err("Profile name is invalid".into()); }
    if object.get("deviceId").and_then(Value::as_str) != Some("aj179") { return Err("This preview supports AJ179 / AJ179P profiles only".into()); }
    let stages = object.get("dpiStages").and_then(Value::as_array).ok_or("DPI stages are missing")?;
    if stages.len() != 6 { return Err("A profile must have six DPI stages".into()); }
    for stage in stages {
        let dpi = stage.get("value").and_then(Value::as_u64).ok_or("DPI value is missing")?;
        if !(50..=26000).contains(&dpi) || dpi % 50 != 0 { return Err("DPI value is out of range".into()); }
        let color = stage.get("color").and_then(Value::as_str).ok_or("DPI color is missing")?;
        if color.len() != 7 || !color.starts_with('#') || !color[1..].bytes().all(|b| b.is_ascii_hexdigit()) { return Err("DPI color is invalid".into()); }
    }
    let count = object.get("dpiStageCount").and_then(Value::as_u64).unwrap_or(6);
    let active = object.get("activeDpi").and_then(Value::as_u64).ok_or("Active DPI stage is missing")?;
    if !(1..=6).contains(&count) || active >= count { return Err("Active DPI stage is invalid".into()); }
    if object.get("reportRateIndex").and_then(Value::as_u64).is_none_or(|rate| rate > 3) { return Err("Report rate is invalid".into()); }
    if !object.get("keys").is_some_and(Value::is_object) || !object.get("settings").is_some_and(Value::is_object) || !object.get("light").is_some_and(Value::is_object) { return Err("Profile settings are incomplete".into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_sleep_minutes_are_converted_without_confusing_wire_units() {
        let mut legacy = default_profile();
        legacy["settings"].as_object_mut().unwrap().remove("sleepWire");
        legacy["settings"]["sleepMin"] = json!(1);
        let mut profiles = Profiles::new();
        profiles.import(&legacy.to_string()).unwrap();
        assert_eq!(profiles.power().unwrap().sleep_wire, 6);
        let power = profiles.power().unwrap();
        profiles.set_power(&power).unwrap();
        assert_eq!(profiles.current()["settings"]["sleepWire"], 6);
        assert!(profiles.current()["settings"].get("sleepMin").is_none());
        profiles.document["profiles"][1]["settings"]["sleepWire"] = json!(1);
        assert_eq!(profiles.power().unwrap().sleep_wire, 1);
    }

    #[test]
    fn updating_one_setting_group_preserves_other_profile_groups() {
        let mut profiles = Profiles::new();
        let before = profiles.current().clone();
        let data = Readout { rate: Some(500), dpi: vec![800; 6], active_dpi: Some(0), dpi_count: Some(6),
            sensor: Some(device::Sensor { lod: 2, angle_snap: true, ripple: true }), ..Default::default() };
        profiles.sync_scope(&data, ReadoutScope::Rate).unwrap();
        assert_eq!(profiles.current()["reportRateIndex"], 2);
        assert_eq!(profiles.current()["dpiStages"], before["dpiStages"]);
        assert_eq!(profiles.current()["settings"], before["settings"]);
    }

    #[test]
    fn changing_one_control_preserves_other_values_in_its_group() {
        let mut profiles = Profiles::new();
        let before = profiles.current().clone();
        let data = Readout {
            dpi: vec![1000; 6], active_dpi: Some(1), dpi_count: Some(6),
            sensor: Some(device::Sensor { lod: 2, angle_snap: true, ripple: true }),
            keys: Some([[0x10, 1, 0], [0x60, 0, 0], [0x60, 0, 0], [0x60, 0, 0], [0x60, 0, 0], [0x60, 0, 0]]),
            lighting: Some(device::Lighting { mode: LightMode::Constant, brightness: 25, speed: 3, color: Some([1, 2, 3]) }),
            ..Default::default()
        };
        profiles.sync_scope(&data, ReadoutScope::DpiValue(1)).unwrap();
        assert_eq!(profiles.current()["dpiStages"][1]["value"], 1000);
        assert_eq!(profiles.current()["dpiStages"][0], before["dpiStages"][0]);
        assert_eq!(profiles.current()["dpiStages"][2], before["dpiStages"][2]);
        profiles.sync_scope(&data, ReadoutScope::Sensor(device::SensorChange::Lod(2))).unwrap();
        assert_eq!(profiles.current()["settings"]["lod"], "high");
        assert_eq!(profiles.current()["settings"]["angleSnap"], before["settings"]["angleSnap"]);
        profiles.sync_scope(&data, ReadoutScope::Key(1)).unwrap();
        assert_eq!(profiles.current()["keys"]["202"], "disable");
        assert_eq!(profiles.current()["keys"]["203"], before["keys"]["203"]);
        profiles.sync_scope(&data, ReadoutScope::Lighting(device::LightChange::Brightness(25))).unwrap();
        assert_eq!(profiles.current()["light"]["brightness"], 25);
        assert_eq!(profiles.current()["light"]["mode"], before["light"]["mode"]);
        assert_eq!(profiles.current()["light"]["color"], before["light"]["color"]);
    }

    #[test]
    fn invalid_scoped_updates_leave_the_profile_unchanged() {
        let mut profiles = Profiles::new();
        let before = profiles.current().clone();
        let data = Readout { dpi: vec![1000; 6], active_dpi: Some(1), dpi_count: Some(6),
            keys: Some([[0x10, 1, 0]; 6]), ..Default::default() };
        assert!(profiles.sync_scope(&data, ReadoutScope::DpiValue(6)).is_err());
        assert_eq!(profiles.current(), &before);
        assert!(profiles.sync_scope(&data, ReadoutScope::Key(6)).is_err());
        assert_eq!(profiles.current(), &before);
    }

    #[test]
    fn editing_one_dpi_color_preserves_other_stages_and_rejects_invalid_input() {
        let mut profiles = Profiles::new();
        let original = profiles.current()["dpiStages"].clone();
        profiles.set_dpi_color(2, "#12abEF").unwrap();
        assert_eq!(profiles.current()["dpiStages"][2]["color"], "#12ABEF");
        assert_eq!(profiles.dpi_colors().unwrap()[2], [0x12, 0xab, 0xef]);
        for i in [0, 1, 3, 4, 5] { assert_eq!(profiles.current()["dpiStages"][i], original[i]); }
        let edited = profiles.current().clone();
        assert!(profiles.set_dpi_color(6, "#123456").is_err());
        assert!(profiles.set_dpi_color(2, "#12345Z").is_err());
        assert_eq!(profiles.current(), &edited);
        let exported = profiles.export_current().unwrap();
        let mut restored = Profiles::new();
        restored.import(&exported).unwrap();
        assert_eq!(restored.current()["dpiStages"][2]["color"], "#12ABEF");
    }

    #[test]
    fn legacy_export_round_trips_and_keeps_extra_fields() {
        let mut source = default_profile();
        source["name"] = json!("Gaming");
        source["settings"]["customSetting"] = json!(true);
        let mut state = Profiles::new();
        state.import(&json!({"app":"Nibble","profile":source}).to_string()).unwrap();
        let exported: Value = serde_json::from_str(&state.export_current().unwrap()).unwrap();
        assert_eq!(exported["profile"]["name"], "Gaming");
        assert_eq!(exported["profile"]["settings"]["customSetting"], true);
        assert!(Profiles::from_json(&state.document.to_string()).is_ok());
    }

    #[test]
    fn invalid_import_does_not_change_selected_profile() {
        let mut state = Profiles::new();
        let before = state.document.clone();
        let mut invalid = default_profile(); invalid["dpiStages"][0]["value"] = json!(99999);
        assert!(state.import(&invalid.to_string()).is_err());
        assert_eq!(state.document, before);
        assert!(Profiles::from_json("{bad").is_err());
        assert!(state.set_local_setting("fineDpi", json!("not a boolean")).is_err());
        assert_eq!(state.document, before);
        state.set_local_setting("fineDpi", json!(true)).unwrap();
        assert_eq!(state.current()["settings"]["fineDpi"], true);
    }

    #[test]
    fn sync_keeps_unread_settings_and_stage_colors() {
        let mut state = Profiles::new();
        state.set_local_setting("fineDpi", json!(true)).unwrap();
        let readout = Readout { dpi: vec![400, 800, 1200, 1600, 2400, 10000], active_dpi: Some(5), dpi_count: Some(6), rate: Some(1000), ..Default::default() };
        state.sync_readout(&readout).unwrap();
        assert_eq!(state.current()["dpiStages"][5]["value"], 10000);
        assert_eq!(state.current()["dpiStages"][5]["color"], "#800080");
        assert_eq!(state.current()["settings"]["fineDpi"], true);
        assert_eq!(state.current()["settings"]["sleepWire"], 30);
        let power = state.power().unwrap();
        assert_eq!(power.sleep_wire, 30);
        assert_eq!(power.debounce, 4);
    }

    #[test]
    fn saved_profiles_round_trip_and_refuse_to_overwrite_recovery_files() {
        let dir = std::env::temp_dir().join(format!("nibble-profiles-{}-{:?}", std::process::id(), std::thread::current().id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("profiles.json");
        let state = Profiles::new();
        state.save(&path).unwrap();
        state.save(&path).unwrap();
        assert_eq!(Profiles::load(&path).unwrap().current(), state.current());
        fs::write(path.with_extension("json.pending"), b"recover me").unwrap();
        assert!(state.save(&path).is_err());
        assert_eq!(fs::read(path.with_extension("json.pending")).unwrap(), b"recover me");
        fs::remove_dir_all(dir).unwrap();
    }
}

