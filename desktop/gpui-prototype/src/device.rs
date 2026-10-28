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
    pub vid: u16,
    pub pid: u16,
    pub path: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lighting {
    pub mode: LightMode,
    pub brightness: u8,
    pub speed: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightMode { Off, Breathing, Constant }

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
    Ok(result)
}

fn open_original_mouse(snapshot: &Readout) -> Result<(HidApi, HidDevice), String> {
    let api = HidApi::new().map_err(|error| error.to_string())?;
    let info = api.device_list().find(|device| {
        device.vendor_id() == snapshot.vid && device.product_id() == snapshot.pid
            && device.path().to_bytes() == snapshot.path.as_slice()
    }).ok_or("Receiver changed or disconnected. Refresh before trying again.")?;
    let mouse = info.open_device(&api).map_err(|error| format!("Could not open the receiver: {error}"))?;
    Ok((api, mouse))
}

pub fn set_active_dpi(snapshot: &Readout, index: usize) -> Result<(usize, usize, Vec<u16>), String> {
    set_dpi(snapshot, DpiChange::Active(index))
}

pub fn set_dpi_count(snapshot: &Readout, count: usize) -> Result<(usize, usize, Vec<u16>), String> {
    set_dpi(snapshot, DpiChange::Count(count))
}

enum DpiChange { Active(usize), Count(usize) }

fn set_dpi(snapshot: &Readout, change: DpiChange) -> Result<(usize, usize, Vec<u16>), String> {
    let (_api, mouse) = open_original_mouse(snapshot)?;
    let (current, current_count, stages) = query(&mouse, 0x13)
        .and_then(|reply| parse_dpi(&reply))
        .ok_or("Could not read the full DPI table. No settings were sent.")?;
    let (active, count) = match change {
        DpiChange::Active(index) => (index, current_count),
        DpiChange::Count(count) if (1..=6).contains(&count) => (current.min(count - 1), count),
        DpiChange::Count(_) => return Err("Unsupported DPI level count. No settings were sent.".into()),
    };
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
        _ => false,
    }
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
    Some(Lighting { mode, brightness: brightness * 25, speed: 1 + (speed * 9 + 2) / 4 })
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
    fn decodes_known_lighting_replies_without_assuming_unknown_states() {
        let mut reply = [0u8; 33];
        reply[1] = 0x15;
        reply[4] = 0x18;
        reply[5] = 2;
        reply[6] = 0x42;
        assert_eq!(parse_light(&reply), Some(Lighting { mode: LightMode::Breathing, brightness: 100, speed: 6 }));
        assert!(matches_reply(0x15, &reply));
        reply[6] = 0x52;
        assert_eq!(parse_light(&reply), None);
        reply[4] = 0;
        assert!(!matches_reply(0x15, &reply));
    }
}
