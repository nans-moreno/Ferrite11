use std::{io, path::Path, ptr};
use windows_sys::Win32::UI::Shell::*;
use winreg::{HKCU, RegKey, enums::KEY_WRITE};

pub fn extensions(id: &str) -> &'static [&'static str] {
    match id {
        "kwil" | "micro" => &[".txt", ".log", ".md"],
        "pdfcraft" => &[".pdf"],
        "oculante" => &[".png", ".jpg", ".jpeg", ".bmp", ".webp"],
        "archiverclick" => &[".zip", ".7z"],
        "leanrows" => &[".csv", ".tsv", ".jsonl", ".ndjson"],
        "kitab" => &[".epub", ".fb2", ".mobi", ".cbz"],
        _ => &[],
    }
}

fn query(ext: &str, field: ASSOCSTR) -> io::Result<Option<String>> {
    let ext: Vec<_> = ext.encode_utf16().chain(Some(0)).collect();
    let mut size = 0;
    let probe = unsafe {
        AssocQueryStringW(
            0,
            field,
            ext.as_ptr(),
            ptr::null(),
            ptr::null_mut(),
            &mut size,
        )
    };
    if matches!(probe as u32, 0x80070483 | 0x80070002) {
        return Ok(None);
    } // ERROR_NO_ASSOCIATION or ERROR_FILE_NOT_FOUND
    if probe < 0 || size == 0 {
        return Err(io::Error::other(crate::local_format!("Association Windows illisible ({probe:#x})", "Could not read the Windows association ({probe:#x})"
        )));
    }
    let mut buf = vec![0; size as usize];
    let hr = unsafe {
        AssocQueryStringW(
            0,
            field,
            ext.as_ptr(),
            ptr::null(),
            buf.as_mut_ptr(),
            &mut size,
        )
    };
    if hr != 0 {
        return Err(io::Error::other(crate::local_format!("Lecture de l’association ({hr:#x})", "Could not read the file association ({hr:#x})"
        )));
    }
    Ok(Some(String::from_utf16_lossy(
        &buf[..buf.iter().position(|v| *v == 0).unwrap_or(buf.len())],
    )))
}

pub fn handler(ext: &str) -> String {
    query(ext, ASSOCSTR_FRIENDLYAPPNAME)
        .ok()
        .flatten()
        .or_else(|| query(ext, ASSOCSTR_PROGID).ok().flatten())
        .unwrap_or_else(|| crate::language::text("non identifié").into())
}

fn prog(id: &str) -> String {
    format!("Ferrite11.{id}.File")
}
fn caps(id: &str) -> String {
    format!(r"Software\Ferrite11\Associations\{id}")
}

fn notify() {
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            ptr::null(),
            ptr::null(),
        );
    }
}

pub fn register(id: &str, name: &str, exe: &Path) -> io::Result<()> {
    let pg = prog(id);
    let cap = caps(id);
    let tx = winreg::transaction::Transaction::new()?;
    let key = |path: &str| -> io::Result<RegKey> {
        HKCU.create_subkey_transacted_with_flags(path, &tx, winreg::enums::KEY_READ | KEY_WRITE)
            .map(|pair| pair.0)
    };
    let class = format!(r"Software\Classes\{pg}");
    key(&class)?.set_value("", &crate::local_format!("Fichier ouvert avec {name}", "File opened with {name}"))?;
    key(&format!(r"{class}\shell\open\command"))?
        .set_value("", &format!(r#""{}" "%1""#, exe.display()))?;
    let details = key(&cap)?;
    details.set_value("ApplicationName", &name)?;
    details.set_value(
        "ApplicationDescription",
        &crate::local_format!("Ouvrir directement les fichiers avec {name}", "Open files directly with {name}"),
    )?;
    let associations = key(&format!(r"{cap}\FileAssociations"))?;
    let before = key(&format!(r"{cap}\Previous"))?;
    for &ext in extensions(id) {
        if before
            .get_raw_value(ext)
            .is_err_and(|e| e.kind() == io::ErrorKind::NotFound)
        {
            before.set_value(ext, &handler(ext))?;
        }
        associations.set_value(ext, &pg)?;
    }
    key(r"Software\RegisteredApplications")?.set_value(format!("Ferrite11.{id}"), &cap)?;
    tx.commit()?;
    notify();
    Ok(())
}

pub fn previous(id: &str, ext: &str) -> Option<String> {
    HKCU.open_subkey(format!(r"{}\Previous", caps(id)))
        .ok()?
        .get_value(ext)
        .ok()
}

pub fn ensure_unused(id: &str, exe: &Path) -> io::Result<()> {
    for &ext in extensions(id) {
        let pg = query(ext, ASSOCSTR_PROGID)?;
        let direct = pg.as_deref().is_some_and(|p| {
            p.eq_ignore_ascii_case(&prog(id))
                || p.eq_ignore_ascii_case(&format!(
                    r"Applications\{}",
                    exe.file_name().unwrap_or_default().to_string_lossy()
                ))
        });
        let same_exe = query(ext, ASSOCSTR_EXECUTABLE)
            .ok()
            .flatten()
            .is_some_and(|s| s.eq_ignore_ascii_case(&exe.to_string_lossy()));
        if direct || same_exe {
            return Err(io::Error::other(crate::local_format!("Choisis une autre application pour {ext} dans Windows avant de retirer cet outil.", "Choose another application for {ext} in Windows before removing this tool."
            )));
        }
    }
    // Also respect formats assigned manually through "Open with" outside our catalogue.
    let choices =
        match HKCU.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts") {
            Ok(k) => k,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
    for ext in choices.enum_keys() {
        let ext = ext?;
        if !ext.starts_with('.') || extensions(id).contains(&ext.as_str()) {
            continue;
        }
        let selected: String = match choices
            .open_subkey(format!(r"{ext}\UserChoice"))
            .and_then(|k| k.get_value("ProgId"))
        {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        let basename = exe.file_name().unwrap_or_default().to_string_lossy();
        if selected.eq_ignore_ascii_case(&prog(id))
            || selected.eq_ignore_ascii_case(&format!(r"Applications\{basename}"))
        {
            return Err(io::Error::other(crate::local_format!("Rétablis également une application pour {ext}.", "Restore a default app for {ext} as well."
            )));
        }
    }
    Ok(())
}

pub fn unregister(id: &str) -> io::Result<()> {
    fn missing_ok(r: io::Result<()>) -> io::Result<()> {
        match r {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
    missing_ok(
        HKCU.open_subkey_with_flags(r"Software\RegisteredApplications", KEY_WRITE)
            .and_then(|k| k.delete_value(format!("Ferrite11.{id}"))),
    )?;
    missing_ok(HKCU.delete_subkey_all(format!(r"Software\Classes\{}", prog(id))))?;
    missing_ok(HKCU.delete_subkey_all(caps(id)))?;
    notify();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_file_handlers_can_be_registered() {
        assert_eq!(extensions("pdfcraft"), &[".pdf"]);
        assert_eq!(extensions("kwil"), extensions("micro"));
        assert!(extensions("micro").contains(&".txt"));
        assert!(extensions("oculante").contains(&".jpg"));
        for tool in ["yazi", "bottom", "fd", "rg"] {
            assert!(extensions(tool).is_empty());
        }
    }
}
