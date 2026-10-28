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

