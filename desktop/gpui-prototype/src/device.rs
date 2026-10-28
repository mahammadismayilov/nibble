use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct Readout {
    pub battery: Option<u8>,
    pub charging: bool,
    pub sleeping: bool,
    pub dpi: Vec<u16>,
    pub active_dpi: Option<usize>,
    pub rate: Option<u16>,
    pub vid: u16,
    pub pid: u16,
}

// Only GET requests are sent. The handle is closed when the scan finishes.
pub fn read_mouse() -> Result<Readout, String> {
    read_mouse_with_trace(&mut Vec::new())
}

pub fn read_mouse_with_trace(trace: &mut Vec<String>) -> Result<Readout, String> {
    let api = HidApi::new().map_err(|error| error.to_string())?;
    let info = api.device_list().find(|device| {
        matches!(device.vendor_id(), 0x248a | 0x249a)
            && matches!(device.product_id(), 0x5c2e | 0x5c2f)
            && ((device.usage_page() == 1 && device.usage() == 0)
                || device.usage_page() >= 0xff00)
    }).ok_or("AJ179P receiver not found")?;
    let mouse = info.open_device(&api).map_err(|error| format!("Receiver found, but could not open it: {error}"))?;
    trace.push(format!("[DEBUG-gpui] interface {:04X}:{:04X} page={:04X} usage={:04X}", info.vendor_id(), info.product_id(), info.usage_page(), info.usage()));
    let mut result = Readout { vid: info.vendor_id(), pid: info.product_id(), ..Default::default() };

    if let Some(reply) = query(&mouse, 0x10, trace) {
        parse_battery(&reply, &mut result);
    }
    if let Some(reply) = query(&mouse, 0x12, trace) {
        result.rate = parse_rate(&reply);
    }
    if let Some(reply) = query(&mouse, 0x13, trace) {
        if let Some((active, stages)) = parse_dpi(&reply) {
            result.active_dpi = Some(active);
            result.dpi = stages;
        }
    }
    Ok(result)
}

fn query(device: &HidDevice, command: u8, trace: &mut Vec<String>) -> Option<[u8; 33]> {
    let mut request = [0u8; 33];
    request[1] = command;
    for attempt in 0..5 {
        if let Err(error) = device.write(&request) {
            trace.push(format!("[DEBUG-gpui] {command:02X} attempt {attempt}: write error {error}"));
            continue;
        }
        trace.push(format!("[DEBUG-gpui] {command:02X} attempt {attempt}: wrote request"));
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now()).as_millis().max(1) as i32;
            let mut raw = [0u8; 65];
            let n = match device.read_timeout(&mut raw, remaining) {
                Ok(n) => n,
                Err(error) => { trace.push(format!("[DEBUG-gpui] {command:02X}: read error {error}")); break; }
            };
            if n == 0 { trace.push(format!("[DEBUG-gpui] {command:02X}: timeout")); continue; }
            trace.push(format!("[DEBUG-gpui] {command:02X}: {} bytes {}", n, raw[..n].iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(" ")));
            let mut reply = [0u8; 33];
            if n >= 33 { reply.copy_from_slice(&raw[..33]); }
            else { reply[1..1 + n.min(32)].copy_from_slice(&raw[..n.min(32)]); }
            let response = if reply[0] == 0 { reply[1] } else { reply[0] };
            if response == command || (reply[0] == 0 && reply[1] == 0)
                || (command == 0x10 && response == 0xc0) {
                return Some(reply);
            }
        }
    }
    None
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

fn parse_dpi(buf: &[u8]) -> Option<(usize, Vec<u16>)> {
    if buf.len() < 30 { return None; }
    let meta = buf.iter().take(8).position(|byte| *byte == 0x25).unwrap_or(4);
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
    if active >= stages.len() || stages.len() < count { None } else { Some((active, stages)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_dpi_table_and_rejects_invalid_active_stage() {
        let mut reply = [0u8; 33];
        reply[4] = 0x25;
        reply[5] = 0x16;
        for (index, wire) in [7u16, 15, 23, 31, 47, 63].into_iter().enumerate() {
            let offset = 6 + index * 4;
            reply[offset..offset + 2].copy_from_slice(&wire.to_le_bytes());
        }
        assert_eq!(parse_dpi(&reply), Some((1, vec![400, 800, 1200, 1600, 2400, 3200])));
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
}

