use windows_sys::Win32::Graphics::Gdi::*;

fn mode(output: &[u16; 32], index: u32) -> Option<DEVMODEW> {
    let mut m = DEVMODEW { dmSize: size_of::<DEVMODEW>() as u16, ..Default::default() };
    (unsafe { EnumDisplaySettingsW(output.as_ptr(), index, &mut m) } != 0).then_some(m)
}

fn matching_hz(current: &DEVMODEW, candidate: &DEVMODEW) -> Option<u32> {
    // Both unions are the display variants returned by EnumDisplaySettingsW.
    let same_scan = unsafe {
        current.Anonymous1.Anonymous2.dmDisplayOrientation == candidate.Anonymous1.Anonymous2.dmDisplayOrientation
            && current.Anonymous2.dmDisplayFlags == candidate.Anonymous2.dmDisplayFlags
    };
    (same_scan && current.dmPelsWidth == candidate.dmPelsWidth
        && current.dmPelsHeight == candidate.dmPelsHeight && current.dmBitsPerPel == candidate.dmBitsPerPel
        && candidate.dmDisplayFrequency > 1).then_some(candidate.dmDisplayFrequency)
}

pub fn summary() -> Vec<String> {
    let mut rows = Vec::new();
    for index in 0.. {
        let mut device = DISPLAY_DEVICEW { cb: size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
        if unsafe { EnumDisplayDevicesW(std::ptr::null(), index, &mut device, 0) } == 0 { break; }
        if device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 { continue; }
        let end = device.DeviceName.iter().position(|u| *u == 0).unwrap_or(device.DeviceName.len());
        let output = String::from_utf16_lossy(&device.DeviceName[..end]);
        let name = output.rsplit('\\').next().unwrap_or(&output);
        let Some(current) = mode(&device.DeviceName, ENUM_CURRENT_SETTINGS) else {
            rows.push(crate::local_format!("{name} · Mode actuel indisponible.", "{name} · Current display mode unavailable.")); continue;
        };
        let maximum = (0..).map_while(|i| mode(&device.DeviceName, i))
            .filter_map(|m| matching_hz(&current, &m)).max();
        let hz = |n: Option<u32>| n.filter(|v| *v > 1).map_or_else(|| crate::language::text("inconnue").to_owned(), |v| format!("{v} Hz"));
        rows.push(crate::local_format!("{name} · {} × {} · Fréquence actuelle {} · Maximale proposée à ce format {}", "{name} · {} × {} · Current refresh rate {} · Highest available in this format {}",
            current.dmPelsWidth, current.dmPelsHeight, hz(Some(current.dmDisplayFrequency)), hz(maximum)));
    }
    if rows.is_empty() { rows.push(crate::language::text("Aucune sortie active lisible dans cette session Windows.").into()); }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_only_matching_formats_and_known_rates() {
        let mode = DEVMODEW { dmPelsWidth: 1920, dmPelsHeight: 1080, dmBitsPerPel: 32, dmDisplayFrequency: 60, ..Default::default() };
        assert_eq!(matching_hz(&mode, &DEVMODEW { dmDisplayFrequency: 240, ..mode }), Some(240));
        assert_eq!(matching_hz(&mode, &DEVMODEW { dmPelsWidth: 1280, ..mode }), None);
        assert_eq!(matching_hz(&mode, &DEVMODEW { dmDisplayFrequency: 1, ..mode }), None);
    }
}
