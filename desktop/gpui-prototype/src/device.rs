use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct Readout {
    pub battery: Option<u8>,
    pub charging: bool,
    pub sleeping: bool,
    pub dpi: Vec<u16>,
    pub active_dpi: Option<usize>,
    pub dpi_count: Option<usize>,
    pub rate: Option<u16>,
    pub lighting: Option<Lighting>,
    pub sensor: Option<Sensor>,
    pub keys: Option<[[u8; 3]; 6]>,
    pub model_confirmed: bool,
    pub vid: u16,
    pub pid: u16,
    pub path: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lighting {
    pub mode: LightMode,
    pub brightness: u8,
    pub speed: u8,
    pub color: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightMode { Off, Breathing, Constant }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sensor { pub lod: u8, pub angle_snap: bool, pub ripple: bool }

// Only GET requests are sent. The handle is closed when the scan finishes.
pub fn read_mouse() -> Result<Readout, String> {
    let api = HidApi::new().map_err(|error| error.to_string())?;
    let info = api.device_list().find(|device| {
        matches!(device.vendor_id(), 0x248a | 0x249a)
            && matches!(device.product_id(), 0x5c2e | 0x5c2f)
            && ((device.usage_page() == 1 && device.usage() == 0)
                || device.usage_page() >= 0xff00)
    }).ok_or("AJ179P receiver not found")?;
    let mouse = info.open_device(&api).map_err(|error| format!("Receiver found, but could not open it: {error}"))?;
    let mut result = Readout { vid: info.vendor_id(), pid: info.product_id(), path: info.path().to_bytes().to_vec(), ..Default::default() };

    if let Some(reply) = query(&mouse, 0x10) {
        check_model(&reply)?;
        result.model_confirmed = model_id(&reply) == Some(*b"M179");
        parse_battery(&reply, &mut result);
    }
    if let Some(reply) = query(&mouse, 0x12) {
        result.rate = parse_rate(&reply);
    }
    if let Some(reply) = query(&mouse, 0x13) {
        if let Some((active, count, stages)) = parse_dpi(&reply) {
            result.active_dpi = Some(active);
            result.dpi_count = Some(count);
            result.dpi = stages;
        }
    }
    if let Some(reply) = query(&mouse, 0x15) {
        result.lighting = parse_light(&reply);
    }
    if let Some(reply) = query(&mouse, 0x16) {
        result.sensor = parse_sensor(&reply);
    }
    if let Some(reply) = query(&mouse, 0x19) {
        result.keys = parse_keymap(&reply);
    }
    Ok(result)
}

fn open_original_mouse(snapshot: &Readout) -> Result<(HidApi, HidDevice), String> {
    let api = HidApi::new().map_err(|error| error.to_string())?;
    let info = api.device_list().find(|device| {
        device.vendor_id() == snapshot.vid && device.product_id() == snapshot.pid
            && device.path().to_bytes() == snapshot.path.as_slice()
    }).ok_or("Receiver changed or disconnected. Refresh before trying again.")?;
    let mouse = info.open_device(&api).map_err(|error| format!("Could not open the receiver: {error}"))?;
    if let Some(reply) = query(&mouse, 0x10) { check_model(&reply)?; }
    Ok((api, mouse))
}

fn model_id(reply: &[u8]) -> Option<[u8; 4]> {
    let offset = (0..reply.len().min(3)).find(|&index| reply[index] == 0x10)?;
    let id: [u8; 4] = reply.get(offset + 4..offset + 8)?.try_into().ok()?;
    if id[0] == b'M' && id[1..].iter().all(u8::is_ascii_alphanumeric) { Some(id) } else { None }
}

fn check_model(reply: &[u8]) -> Result<(), String> {
    match model_id(reply) {
        Some(id) if id != *b"M179" => Err(format!("Receiver reports model {}. This preview only supports AJ179 / AJ179P.", String::from_utf8_lossy(&id))),
        _ => Ok(()),
    }
}

pub fn set_active_dpi(snapshot: &Readout, index: usize) -> Result<(usize, usize, Vec<u16>), String> {
    set_dpi(snapshot, DpiChange::Active(index))
}

pub fn set_dpi_count(snapshot: &Readout, count: usize) -> Result<(usize, usize, Vec<u16>), String> {
    set_dpi(snapshot, DpiChange::Count(count))
}

pub fn set_dpi_value(snapshot: &Readout, index: usize, value: u16) -> Result<(usize, usize, Vec<u16>), String> {
    set_dpi(snapshot, DpiChange::Value(index, value))
}

// The receiver does not expose a reliable readback for its complete color table.
// The caller supplies all six profile colors and reports the table as sent only.
pub fn set_dpi_colors(snapshot: &Readout, colors: [[u8; 3]; 6]) -> Result<Option<Lighting>, String> {
    let (_api, mouse) = open_original_mouse(snapshot)?;
    let (active, _, _) = query(&mouse, 0x13).and_then(|reply| parse_dpi(&reply))
        .ok_or("Could not read the active DPI stage. No colors were sent.")?;
    let mut light = query(&mouse, 0x15).and_then(|reply| parse_light(&reply))
        .ok_or("Could not read lighting settings. No colors were sent.")?;
    let light_update = if light.mode == LightMode::Off { None } else {
        light.color = Some(colors[active]);
        Some(light_packet(&light)?)
    };
    send_setting(&mouse, &dpi_color_packet(&colors)).map_err(|error| format!("{error} The mouse may have received the color table."))?;
    if let Some(packet) = light_update {
        send_setting(&mouse, &packet).map_err(|error| format!("DPI colors were sent, but lighting may not have updated: {error}"))?;
        let readback = query(&mouse, 0x15).and_then(|reply| parse_light(&reply))
            .ok_or("DPI colors were sent, but the lighting response was unavailable.")?;
        if readback.mode != light.mode || readback.color != light.color || readback.brightness != light.brightness {
            return Err("DPI colors were sent, but the mouse returned different lighting settings.".into());
        }
        Ok(Some(readback))
    } else { Ok(None) }
}

fn dpi_color_packet(colors: &[[u8; 3]; 6]) -> [u8; 33] {
    let mut packet = [0u8; 33];
    packet[1] = 4; packet[3] = 1; packet[4] = 0x12;
    for (index, color) in colors.iter().enumerate() {
        packet[5 + index * 3..8 + index * 3].copy_from_slice(color);
    }
    packet[32] = packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    packet
}

enum DpiChange { Active(usize), Count(usize), Value(usize, u16) }

fn apply_dpi_change(current: usize, current_count: usize, mut stages: Vec<u16>, change: DpiChange) -> Result<(usize, usize, Vec<u16>), String> {
    if stages.len() != 6 || !(1..=6).contains(&current_count) || current >= current_count {
        return Err("The DPI table is incomplete. No settings were sent.".into());
    }
    let (active, count) = match change {
        DpiChange::Active(index) if index < current_count => (index, current_count),
        DpiChange::Active(_) => return Err("Unsupported DPI stage. No settings were sent.".into()),
        DpiChange::Count(count) if (1..=6).contains(&count) => (current.min(count - 1), count),
        DpiChange::Count(_) => return Err("Unsupported DPI level count. No settings were sent.".into()),
        DpiChange::Value(index, value) if index < current_count && (50..=26000).contains(&value) && value % 50 == 0 => {
            stages[index] = value;
            (current, current_count)
        }
        DpiChange::Value(_, _) => return Err("Unsupported DPI value. No settings were sent.".into()),
    };
    Ok((active, count, stages))
}

fn set_dpi(snapshot: &Readout, change: DpiChange) -> Result<(usize, usize, Vec<u16>), String> {
    let (_api, mouse) = open_original_mouse(snapshot)?;
    let (current, current_count, stages) = query(&mouse, 0x13)
        .and_then(|reply| parse_dpi(&reply))
        .ok_or("Could not read the full DPI table. No settings were sent.")?;
    let (active, count, stages) = apply_dpi_change(current, current_count, stages, change)?;
    let packet = dpi_packet(&stages, active, count)?;
    send_setting(&mouse, &packet)?;
    let readback = query(&mouse, 0x13).and_then(|reply| parse_dpi(&reply))
        .ok_or("DPI change was sent, but the mouse did not confirm it. Refresh before trying again.")?;
    if readback.0 != active || readback.1 != count || readback.2 != stages {
        return Err("The mouse returned different DPI settings. Refresh before trying again.".into());
    }
    Ok(readback)
}

pub fn set_rate(snapshot: &Readout, rate: u16) -> Result<u16, String> {
    let (_api, mouse) = open_original_mouse(snapshot)?;
    let packet = rate_packet(rate)?;
    if query(&mouse, 0x12).and_then(|reply| parse_rate(&reply)).is_none() {
        return Err("Could not read the report rate. No settings were sent.".into());
    }
    send_setting(&mouse, &packet)?;
    let actual = query(&mouse, 0x12).and_then(|reply| parse_rate(&reply))
        .ok_or("Report-rate change was sent, but the mouse did not confirm it. Refresh before trying again.")?;
    if actual != rate {
        return Err("The mouse returned a different report rate. Refresh before trying again.".into());
    }
    Ok(actual)
}

#[derive(Clone, Copy)]
pub enum SensorChange { Lod(u8), AngleSnap(bool), Ripple(bool) }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerSettings { pub sleep_wire: u8, pub move_wake: bool, pub move_close_light: bool, pub debounce: u8 }

pub fn set_power(snapshot: &Readout, power: &PowerSettings) -> Result<(), String> {
    let packet = power_packet(power)?;
    let (_api, mouse) = open_original_mouse(snapshot)?;
    send_setting(&mouse, &packet)
}

fn power_packet(power: &PowerSettings) -> Result<[u8; 33], String> {
    if ![1, 3, 6, 12, 30, 60, 120, 180].contains(&power.sleep_wire) || power.debounce > 30 {
        return Err("Unsupported power settings. No settings were sent.".into());
    }
    let mut packet = [0u8; 33];
    packet[1] = 7; packet[3] = 1; packet[4] = 4;
    packet[5] = power.sleep_wire; packet[6] = u8::from(power.move_wake);
    packet[7] = u8::from(power.move_close_light); packet[8] = power.debounce;
    packet[32] = packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    Ok(packet)
}

pub fn set_sensor(snapshot: &Readout, change: SensorChange) -> Result<Sensor, String> {
    let (_api, mouse) = open_original_mouse(snapshot)?;
    let mut sensor = query(&mouse, 0x16).and_then(|reply| parse_sensor(&reply))
        .ok_or("Could not read sensor settings. No settings were sent.")?;
    match change {
        SensorChange::Lod(value) if (1..=2).contains(&value) => sensor.lod = value,
        SensorChange::Lod(_) => return Err("Unsupported lift-off distance. No settings were sent.".into()),
        SensorChange::AngleSnap(value) => sensor.angle_snap = value,
        SensorChange::Ripple(value) => sensor.ripple = value,
    }
    let packet = sensor_packet(&sensor);
    send_setting(&mouse, &packet)?;
    let readback = query(&mouse, 0x16).and_then(|reply| parse_sensor(&reply))
        .ok_or("Sensor change was sent, but the mouse did not confirm it. Refresh before trying again.")?;
    if readback != sensor { return Err("The mouse returned different sensor settings. Refresh before trying again.".into()); }
    Ok(readback)
}

pub fn set_key(snapshot: &Readout, index: usize, label: &str) -> Result<[[u8; 3]; 6], String> {
    if index == 0 { return Err("Left-click is locked to prevent losing mouse control.".into()); }
    let wire = key_wire(label).ok_or("This button function is not available.")?;
    let (_api, mouse) = open_original_mouse(snapshot)?;
    let mut keys = query(&mouse, 0x19).and_then(|reply| parse_keymap(&reply))
        .ok_or("Could not read the complete keymap. No settings were sent.")?;
    let slot = keys.get_mut(index).ok_or("Unsupported mouse button. No settings were sent.")?;
    *slot = wire;
    let packet = keymap_packet(&keys);
    send_setting(&mouse, &packet)?;
    let readback = query(&mouse, 0x19).and_then(|reply| parse_keymap(&reply))
        .ok_or("Key assignment was sent, but the mouse did not confirm it. Refresh before trying again.")?;
    if readback != keys { return Err("The mouse returned a different keymap. Refresh before trying again.".into()); }
    Ok(readback)
}

pub fn default_keymap() -> [[u8; 3]; 6] {
    [[0x10, 1, 0], [0x10, 2, 0], [0x10, 4, 0], [0x10, 8, 0], [0x10, 16, 0], [0x40, 1, 0]]
}

pub fn reset_keys(snapshot: &Readout) -> Result<[[u8; 3]; 6], String> {
    let (_api, mouse) = open_original_mouse(snapshot)?;
    if query(&mouse, 0x19).and_then(|reply| parse_keymap(&reply)).is_none() {
        return Err("Could not read the complete keymap. No settings were sent.".into());
    }
    let keys = default_keymap();
    send_setting(&mouse, &keymap_packet(&keys))?;
    let readback = query(&mouse, 0x19).and_then(|reply| parse_keymap(&reply))
        .ok_or("Key reset was sent, but the mouse did not confirm it. Refresh before trying again.")?;
    if readback != keys { return Err("The mouse returned a different keymap. Refresh before trying again.".into()); }
    Ok(readback)
}

pub fn key_wire(label: &str) -> Option<[u8; 3]> {
    Some(match label {
        "Left-Click" => [0x10, 1, 0], "Right-Click" => [0x10, 2, 0], "Middle-Click" => [0x10, 4, 0],
        "Backward" => [0x10, 8, 0], "Forward" => [0x10, 16, 0], "Double-Click" => [0x30, 0x32, 3],
        "Scroll Up" => [0x20, 1, 0], "Scroll Down" => [0x20, 0xff, 0], "DPI Loop" => [0x40, 1, 0],
        "Disable Key" => [0x60, 0, 0], "Lighting on/off" => [0x60, 4, 0],
        "Volume +" => [0x80, 0xe9, 0], "Volume -" => [0x80, 0xea, 0], "Mute" => [0x80, 0xe2, 0],
        "Play/Pause" => [0x80, 0xcd, 0], "Previous" => [0x80, 0xb6, 0], "Next" => [0x80, 0xb5, 0], "Stop" => [0x80, 0xb7, 0],
        "Calculator" => [0x80, 0x92, 1], "My Computer" => [0x80, 0x94, 1], "Mail" => [0x80, 0x8a, 1],
        "Screen Brightness +" => [0x80, 0x6f, 0], "Screen Brightness -" => [0x80, 0x70, 0],
        "Refresh (F5)" => [0x70, 8, 7], "Switch Application" => [0x70, 8, 0x2b], "Copy" => [0x70, 1, 6], "Paste" => [0x70, 1, 0x19],
        _ => return None,
    })
}

pub fn key_label(wire: [u8; 3]) -> &'static str {
    const LABELS: [&str; 25] = ["Left-Click", "Right-Click", "Middle-Click", "Backward", "Forward", "Double-Click", "Scroll Up", "Scroll Down", "DPI Loop", "Disable Key", "Lighting on/off", "Volume +", "Volume -", "Mute", "Play/Pause", "Previous", "Next", "Stop", "Calculator", "My Computer", "Mail", "Screen Brightness +", "Screen Brightness -", "Refresh (F5)", "Switch Application"];
    LABELS.into_iter().chain(["Copy", "Paste"]).find(|label| key_wire(label) == Some(wire)).unwrap_or("Unknown")
}

fn keymap_packet(keys: &[[u8; 3]; 6]) -> [u8; 33] {
    let mut packet = [0u8; 33];
    packet[1] = 9; packet[3] = 1; packet[4] = 0x0f;
    for (i, triple) in keys.iter().enumerate() { packet[5 + 3 * i..8 + 3 * i].copy_from_slice(triple); }
    packet[32] = packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    packet
}

fn sensor_packet(sensor: &Sensor) -> [u8; 33] {
    let mut packet = [0u8; 33];
    packet[1] = 0x06;
    packet[3] = 1;
    packet[4] = 5;
    packet[5] = sensor.lod;
    packet[6] = u8::from(sensor.angle_snap);
    packet[7] = u8::from(sensor.ripple);
    packet[32] = packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    packet
}

#[derive(Clone, Copy)]
pub enum LightChange { Mode(LightMode), Brightness(u8), Speed(u8) }

pub fn set_lighting(snapshot: &Readout, change: LightChange) -> Result<Lighting, String> {
    let (_api, mouse) = open_original_mouse(snapshot)?;
    let mut light = query(&mouse, 0x15).and_then(|reply| parse_light(&reply))
        .ok_or("Could not read lighting settings. No settings were sent.")?;
    if light.color.is_none() {
        light.color = snapshot.lighting.as_ref().and_then(|previous| previous.color);
    }
    match change {
        LightChange::Mode(mode) => light.mode = mode,
        LightChange::Brightness(value) if light.mode != LightMode::Off && value <= 100 && value % 25 == 0 => light.brightness = value,
        LightChange::Speed(value) if light.mode == LightMode::Breathing && (1..=10).contains(&value) => light.speed = value,
        _ => return Err("Unsupported lighting value. No settings were sent.".into()),
    }
    let packet = light_packet(&light)?;
    send_setting(&mouse, &packet)?;
    let mut readback = query(&mouse, 0x15).and_then(|reply| parse_light(&reply))
        .ok_or("Lighting change was sent, but the mouse did not confirm it. Refresh before trying again.")?;
    let expected_speed_wire = ((u16::from(light.speed - 1) * 4 + 4) / 9) as u8;
    let expected_speed = 1 + (expected_speed_wire * 9 + 2) / 4;
    if readback.mode != light.mode || (light.mode != LightMode::Off && (readback.brightness != light.brightness || readback.color != light.color))
        || (light.mode == LightMode::Breathing && readback.speed != expected_speed) {
        return Err("The mouse returned different lighting settings. Refresh before trying again.".into());
    }
    if readback.color.is_none() { readback.color = light.color; }
    Ok(readback)
}

fn light_packet(light: &Lighting) -> Result<[u8; 33], String> {
    let mut packet = [0u8; 33];
    packet[1] = 5; packet[3] = 1;
    if light.mode == LightMode::Off { packet[4] = 2; return Ok(packet); }
    let color = light.color.ok_or("The current lighting color is unavailable. No settings were sent.")?;
    if light.brightness > 100 || light.brightness % 25 != 0 || !(1..=10).contains(&light.speed) {
        return Err("Unsupported lighting values. No settings were sent.".into());
    }
    let speed = ((u16::from(light.speed - 1) * 4 + 4) / 9) as u8;
    packet[6] = ((light.brightness / 25) << 4) | speed;
    match light.mode {
        LightMode::Constant => { packet[4] = 5; packet[5] = 3; packet[7..10].copy_from_slice(&color); }
        LightMode::Breathing => {
            packet[4] = 0x18; packet[5] = 2; packet[7] = 7;
            for i in 0..7 { packet[8 + i * 3..11 + i * 3].copy_from_slice(&color); }
        }
        LightMode::Off => unreachable!(),
    }
    packet[32] = packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    Ok(packet)
}

fn send_setting(mouse: &HidDevice, packet: &[u8; 33]) -> Result<(), String> {
    match mouse.write(packet) {
        Ok(33) => std::thread::sleep(Duration::from_millis(90)),
        Ok(_) => return Err("The receiver accepted an incomplete packet. Refresh before trying again.".into()),
        Err(error) => return Err(format!("Could not send the setting: {error}")),
    }
    Ok(())
}

fn dpi_packet(stages: &[u16], active: usize, count: usize) -> Result<[u8; 33], String> {
    if stages.len() != 6 || !(1..=6).contains(&count) || active >= count
        || stages.iter().any(|dpi| !(50..=26000).contains(dpi) || dpi % 50 != 0) {
        return Err("The DPI table is incomplete or unsupported. No settings were sent.".into());
    }
    let mut packet = [0u8; 33];
    packet[1] = 0x03;
    packet[3] = 1;
    packet[4] = 0x25;
    packet[5] = ((active as u8) << 4) | count as u8;
    for (index, dpi) in stages.iter().enumerate() {
        let wire = (dpi / 50 - 1).to_le_bytes();
        let offset = 6 + index * 4;
        packet[offset..offset + 2].copy_from_slice(&wire);
        packet[offset + 2..offset + 4].copy_from_slice(&wire);
    }
    packet[32] = packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    Ok(packet)
}

fn rate_packet(rate: u16) -> Result<[u8; 33], String> {
    let wire = match rate { 125 => 8, 250 => 4, 500 => 2, 1000 => 1,
        _ => return Err("Unsupported report rate. No settings were sent.".into()) };
    let mut packet = [0u8; 33];
    packet[1] = 0x02;
    packet[3] = 1;
    packet[4] = 1;
    packet[5] = wire;
    packet[32] = wire;
    Ok(packet)
}

fn query(device: &HidDevice, command: u8) -> Option<[u8; 33]> {
    let mut request = [0u8; 33];
    request[1] = command;
    if command == 0x16 { request[3] = 1; }
    for _ in 0..5 {
        if device.write(&request).is_err() {
            continue;
        }
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now()).as_millis().max(1) as i32;
            let mut raw = [0u8; 65];
            let n = match device.read_timeout(&mut raw, remaining) {
                Ok(n) => n,
                Err(_) => break,
            };
            if n == 0 { continue; }
            let reply = normalize_reply(&raw[..n]);
            if matches_reply(command, &reply) {
                return Some(reply);
            }
        }
    }
    None
}

fn normalize_reply(raw: &[u8]) -> [u8; 33] {
    let mut reply = [0u8; 33];
    if raw.len() >= 33 {
        reply.copy_from_slice(&raw[..33]);
    } else {
        reply[1..1 + raw.len()].copy_from_slice(raw);
    }
    reply
}

fn matches_reply(command: u8, reply: &[u8; 33]) -> bool {
    let response = if reply[0] == 0 { reply[1] } else { reply[0] };
    if response != command && response != 0 && !(command == 0x10 && response == 0xc0) {
        return false;
    }
    match command {
        0x10 => response == 0x10 || response == 0xc0,
        0x12 => parse_rate(reply).is_some(),
        0x13 => parse_dpi(reply).is_some(),
        0x15 => parse_light(reply).is_some(),
        0x16 => parse_sensor(reply).is_some(),
        0x19 => parse_keymap(reply).is_some(),
        _ => false,
    }
}

fn parse_keymap(buf: &[u8]) -> Option<[[u8; 3]; 6]> {
    if buf.len() < 23 || buf[4] != 0x0f { return None; }
    let mut keys = [[0u8; 3]; 6];
    for (i, key) in keys.iter_mut().enumerate() {
        key.copy_from_slice(&buf[5 + 3 * i..8 + 3 * i]);
        if *key == [0, 0, 0] { return None; }
    }
    Some(keys)
}

fn parse_sensor(buf: &[u8]) -> Option<Sensor> {
    if buf.len() < 8 || buf[4] != 5 || !(1..=2).contains(&buf[5]) || buf[6] > 1 || buf[7] > 1 {
        return None;
    }
    Some(Sensor { lod: buf[5], angle_snap: buf[6] == 1, ripple: buf[7] == 1 })
}

fn parse_light(buf: &[u8]) -> Option<Lighting> {
    if buf.len() < 7 { return None; }
    let mode = match (buf[4], buf[5] & 0x0f) {
        (0x02, 0) => LightMode::Off,
        (0x18, 2) => LightMode::Breathing,
        (0x05, 3) => LightMode::Constant,
        _ => return None,
    };
    let brightness = buf[6] >> 4;
    let speed = buf[6] & 0x0f;
    if brightness > 4 || speed > 4 { return None; }
    let color = match mode {
        LightMode::Off => None,
        LightMode::Constant if buf.len() >= 10 => Some([buf[7], buf[8], buf[9]]),
        LightMode::Breathing if buf.len() >= 11 => Some([buf[8], buf[9], buf[10]]),
        _ => None,
    };
    Some(Lighting { mode, brightness: brightness * 25, speed: 1 + (speed * 9 + 2) / 4, color })
}

fn parse_battery(buf: &[u8], result: &mut Readout) {
    for index in 0..buf.len().min(3) {
        if buf[index] == 0xc0 && index + 2 < buf.len() {
            result.sleeping = buf[index + 1] == 0;
            if !result.sleeping {
                let raw = buf[index + 2];
                result.charging = raw == 255 || buf.get(index + 3).is_some_and(|flag| (1..=3).contains(flag));
                result.battery = battery_percent(raw);
            }
            return;
        }
        if buf[index] == 0x10 && index + 13 < buf.len() {
            let raw = buf[index + 13];
            result.charging = raw == 255 || (1..=3).contains(&buf[index + 12]);
            result.battery = battery_percent(raw);
            return;
        }
    }
}

fn battery_percent(raw: u8) -> Option<u8> {
    match raw {
        0 | 255 => None,
        1..=100 => Some(raw),
        _ => Some(((raw as f32 / 255.0) * 100.0).round() as u8),
    }
}

fn parse_rate(buf: &[u8]) -> Option<u16> {
    if buf.len() < 6 || buf[4] != 1 { return None; }
    match buf[5] { 8 => Some(125), 4 => Some(250), 2 => Some(500), 1 => Some(1000), _ => None }
}

fn parse_dpi(buf: &[u8]) -> Option<(usize, usize, Vec<u16>)> {
    if buf.len() < 30 { return None; }
    // A normal GET reply places metadata at offset 4, even when the firmware
    // reports 0x19 instead of 0x25. Do not mistake a DPI value of 0x25 for metadata.
    let meta = if buf[0] == 0 && (buf[1] == 0x13 || buf[1] == 0) {
        4
    } else {
        buf.iter().take(8).position(|byte| *byte == 0x25).unwrap_or(4)
    };
    let packed = *buf.get(meta + 1)?;
    let active = usize::from(packed >> 4);
    let count = usize::from(packed & 0x0f);
    if count == 0 || count > 6 || active >= count { return None; }
    let mut stages = Vec::new();
    for index in 0..6 {
        let offset = meta + 2 + index * 4;
        if offset + 3 >= buf.len() { break; }
        let wire = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        if wire > 600 { return None; }
        stages.push((wire + 1) * 50);
    }
    if active >= stages.len() || stages.len() < count { None } else { Some((active, count, stages)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_model_identity_without_treating_sleep_reports_as_identity() {
        let mut reply = [0u8; 33];
        reply[1] = 0x10;
        reply[5..9].copy_from_slice(b"M179");
        assert_eq!(model_id(&reply), Some(*b"M179"));
        assert!(check_model(&reply).is_ok());
        reply[5..9].copy_from_slice(b"M620");
        assert!(check_model(&reply).is_err());
        reply[1] = 0xc0;
        assert_eq!(model_id(&reply), None);
        assert!(check_model(&reply).is_ok());
        assert_eq!(model_id(&[0x10, 0, 0]), None);
    }

    #[test]
    fn default_button_map_matches_original_device_order() {
        let keys = default_keymap();
        assert_eq!(keys.map(key_label), ["Left-Click", "Right-Click", "Middle-Click", "Backward", "Forward", "DPI Loop"]);
        let packet = keymap_packet(&keys);
        assert_eq!(&packet[..5], &[0, 9, 0, 1, 0x0f]);
        for (index, key) in keys.iter().enumerate() { assert_eq!(&packet[5 + index * 3..8 + index * 3], key); }
        assert_eq!(parse_keymap(&packet), Some(keys));
        assert_eq!(packet[32], packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)));
    }

    #[test]
    fn color_packet_sends_all_six_profile_colors_in_order() {
        let colors = [[255, 0, 0], [0, 255, 0], [0x12, 0xab, 0xef], [0, 255, 255], [255, 255, 0], [128, 0, 128]];
        let packet = dpi_color_packet(&colors);
        assert_eq!(&packet[..5], &[0, 4, 0, 1, 0x12]);
        assert_eq!(&packet[5..23], &[255, 0, 0, 0, 255, 0, 0x12, 0xab, 0xef, 0, 255, 255, 255, 255, 0, 128, 0, 128]);
        assert!(packet[23..32].iter().all(|byte| *byte == 0));
        assert_eq!(packet[32], packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)));
    }

    #[test]
    fn decodes_dpi_table_and_rejects_invalid_active_stage() {
        let mut reply = [0u8; 33];
        reply[4] = 0x25;
        reply[5] = 0x16;
        for (index, wire) in [7u16, 15, 23, 31, 47, 63].into_iter().enumerate() {
            let offset = 6 + index * 4;
            reply[offset..offset + 2].copy_from_slice(&wire.to_le_bytes());
        }
        assert_eq!(parse_dpi(&reply), Some((1, 6, vec![400, 800, 1200, 1600, 2400, 3200])));
        reply[5] = 0x66;
        assert_eq!(parse_dpi(&reply), None);
    }

    #[test]
    fn battery_and_rate_do_not_treat_invalid_values_as_live_readings() {
        assert_eq!(battery_percent(80), Some(80));
        assert_eq!(battery_percent(255), None);
        assert_eq!(parse_rate(&[0, 0x12, 0, 0, 1, 1]), Some(1000));
        assert_eq!(parse_rate(&[0, 0x12, 0, 0, 0, 1]), None);
    }

    #[test]
    fn decodes_the_aj179p_receiver_response_and_ignores_bad_acknowledgements() {
        let raw = [0x13, 0, 1, 0x19, 0x56, 7, 0, 7, 0, 15, 0, 15, 0, 23, 0, 23, 0,
            31, 0, 31, 0, 47, 0, 47, 0, 199, 0, 199, 0, 0, 0, 0xda];
        let reply = normalize_reply(&raw);
        assert!(matches_reply(0x13, &reply));
        assert_eq!(parse_dpi(&reply), Some((5, 6, vec![400, 800, 1200, 1600, 2400, 10000])));
        assert!(!matches_reply(0x12, &reply));
        assert!(!matches_reply(0x13, &[0; 33]));

        let mut value_containing_25 = reply;
        value_containing_25[6] = 0x25;
        value_containing_25[8] = 0x25;
        assert_eq!(parse_dpi(&value_containing_25).unwrap().2[0], 1900);
    }

    #[test]
    fn settings_packets_match_the_existing_protocol_and_reject_incomplete_tables() {
        let packet = dpi_packet(&[400, 800, 1200, 1600, 2400, 10000], 2, 6).unwrap();
        assert_eq!(packet, [0, 3, 0, 1, 0x25, 0x26, 7, 0, 7, 0, 15, 0, 15, 0,
            23, 0, 23, 0, 31, 0, 31, 0, 47, 0, 47, 0, 199, 0, 199, 0, 0, 0, 0xaa]);
        let two_levels = dpi_packet(&[400, 800, 1200, 1600, 2400, 10000], 1, 2).unwrap();
        assert_eq!(two_levels, [0, 3, 0, 1, 0x25, 0x12, 7, 0, 7, 0, 15, 0, 15, 0,
            23, 0, 23, 0, 31, 0, 31, 0, 47, 0, 47, 0, 199, 0, 199, 0, 0, 0, 0x96]);
        assert!(dpi_packet(&[400, 800], 0, 2).is_err());
        assert!(dpi_packet(&[400, 800, 1200, 1600, 2400, 10001], 0, 6).is_err());
        assert!(dpi_packet(&[400, 800, 1200, 1600, 2400, 10000], 2, 2).is_err());
        let rate = rate_packet(500).unwrap();
        assert_eq!(&rate[..6], &[0, 2, 0, 1, 1, 2]);
        assert_eq!(rate[32], 2);
        assert!(rate_packet(8000).is_err());
    }

    #[test]
    fn editing_one_dpi_value_preserves_the_other_stages_and_count() {
        let stages = vec![400, 800, 1200, 1600, 2400, 10000];
        let (active, count, updated) = apply_dpi_change(5, 6, stages.clone(), DpiChange::Value(2, 1800)).unwrap();
        assert_eq!((active, count), (5, 6));
        assert_eq!(updated, vec![400, 800, 1800, 1600, 2400, 10000]);
        assert!(apply_dpi_change(1, 2, stages.clone(), DpiChange::Value(2, 1800)).is_err());
        assert!(apply_dpi_change(5, 6, stages, DpiChange::Value(2, 1801)).is_err());
    }

    #[test]
    fn decodes_known_lighting_replies_without_assuming_unknown_states() {
        let mut reply = [0u8; 33];
        reply[1] = 0x15;
        reply[4] = 0x18;
        reply[5] = 2;
        reply[6] = 0x42;
        assert_eq!(parse_light(&reply), Some(Lighting { mode: LightMode::Breathing, brightness: 100, speed: 6, color: Some([0, 0, 0]) }));
        assert!(matches_reply(0x15, &reply));
        reply[6] = 0x52;
        assert_eq!(parse_light(&reply), None);
        reply[4] = 0;
        assert!(!matches_reply(0x15, &reply));
    }

    #[test]
    fn lighting_packets_keep_the_reported_color() {
        let light = Lighting { mode: LightMode::Breathing, brightness: 100, speed: 6, color: Some([1, 2, 3]) };
        let packet = light_packet(&light).unwrap();
        assert_eq!(&packet[..11], &[0, 5, 0, 1, 0x18, 2, 0x42, 7, 1, 2, 3]);
        assert_eq!(&packet[26..29], &[1, 2, 3]);
        assert_eq!(packet[32], packet[5..32].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)));
        assert!(light_packet(&Lighting { color: None, ..light }).is_err());
    }

    #[test]
    fn rejects_unrecognised_sensor_readings() {
        let mut reply = [0u8; 33];
        reply[1] = 0x16;
        reply[4..8].copy_from_slice(&[5, 2, 1, 0]);
        assert_eq!(parse_sensor(&reply), Some(Sensor { lod: 2, angle_snap: true, ripple: false }));
        assert!(matches_reply(0x16, &reply));
        reply[6] = 2;
        assert!(!matches_reply(0x16, &reply));
        let packet = sensor_packet(&Sensor { lod: 2, angle_snap: true, ripple: false });
        assert_eq!(&packet[..8], &[0, 6, 0, 1, 5, 2, 1, 0]);
        assert_eq!(packet[32], 3);
    }

    #[test]
    fn power_packet_matches_existing_protocol_and_rejects_bad_values() {
        let power = PowerSettings { sleep_wire: 30, move_wake: true, move_close_light: false, debounce: 4 };
        let packet = power_packet(&power).unwrap();
        assert_eq!(&packet[..9], &[0, 7, 0, 1, 4, 30, 1, 0, 4]);
        assert_eq!(packet[32], 35);
        assert!(power_packet(&PowerSettings { sleep_wire: 2, ..power.clone() }).is_err());
        assert!(power_packet(&PowerSettings { debounce: 31, ..power }).is_err());
    }

    #[test]
    fn keymap_packet_preserves_other_buttons_and_unknown_functions() {
        let mut keys = [[0x10, 1, 0], [0x10, 2, 0], [0x10, 4, 0], [0x10, 8, 0], [0x71, 42, 1], [0x40, 1, 0]];
        keys[1] = key_wire("Volume +").unwrap();
        let packet = keymap_packet(&keys);
        assert_eq!(&packet[4..8], &[0x0f, 0x10, 1, 0]);
        assert_eq!(&packet[17..20], &[0x71, 42, 1]);
        assert_eq!(packet[32], packet[5..23].iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)));
        let mut reply = packet; reply[1] = 0x19;
        assert_eq!(parse_keymap(&reply), Some(keys));
        assert_eq!(key_label(keys[4]), "Unknown");
        assert!(key_wire("Fire button").is_none());
    }
}

