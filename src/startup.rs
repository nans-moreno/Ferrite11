use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_REPARSE_POINT, GetDriveTypeW};
use winreg::enums::{KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SZ};
use winreg::transaction::Transaction;
use winreg::{HKCU, RegKey, RegValue};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    name: String,
    expandable: bool,
    bytes: Vec<u8>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Saved(Vec<Entry>);

fn install_roots() -> Result<Vec<PathBuf>, String> {
    let drive = std::env::var("SystemDrive").map_err(|e| e.to_string())?;
    let bytes = drive.as_bytes();
    if bytes.len() != 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return Err(crate::language::text("Lecteur système inconnu.").into());
    }
    let root: Vec<_> = format!("{drive}\\").encode_utf16().chain(Some(0)).collect();
    // GetDriveTypeW returns DRIVE_FIXED (3) for a local fixed disk.
    if unsafe { GetDriveTypeW(root.as_ptr()) } != 3 {
        return Err(crate::language::text("Le lecteur système n’est pas un disque local fixe.").into());
    }
    let mut roots: Vec<_> = ["ProgramFiles", "ProgramFiles(x86)"].iter()
        .filter_map(std::env::var_os).map(PathBuf::from).collect();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Programs"));
    }
    roots.retain(|p| p.is_absolute() && p.to_str().is_some_and(|s| {
        s.get(..2).is_some_and(|prefix| prefix.eq_ignore_ascii_case(&drive)) && s.as_bytes().get(2) == Some(&b'\\')
    }));
    Ok(roots)
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.encode_utf16().count() <= 256 && !name.chars().any(char::is_control)
}

fn exe_path(raw: &[u8], roots: &[PathBuf]) -> Option<PathBuf> {
    if raw.len() > 2 * 261 || raw.len() % 2 != 0 { return None; }
    let mut words: Vec<_> = raw.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
    if words.pop() != Some(0) || words.contains(&0) { return None; }
    let command = String::from_utf16(&words).ok()?;
    // Accept a single quoted literal with no arguments. Leave all launcher and
    // environment-variable commands untouched, even if a part looks like a path.
    let path = command.strip_prefix('"')?.strip_suffix('"')?;
    let bytes = path.as_bytes();
    if bytes.len() < 7 || !bytes[0].is_ascii_alphabetic() || &bytes[1..3] != b":\\" { return None; }
    for component in path[3..].split('\\') {
        if component.is_empty() || component.ends_with(['.', ' '])
            || component.contains(['"', '/', ':', '%', '?', '*', '<', '>', '|'])
            || component.chars().any(char::is_control) { return None; }
        let stem = component.split('.').next()?.trim_end().to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
            || ["COM", "LPT"].iter().any(|p| stem.strip_prefix(p).is_some_and(
                |suffix| "0123456789¹²³".chars().any(|d| d.to_string() == suffix)
            )) { return None; }
    }
    let candidate = Path::new(path);
    if !candidate.extension()?.to_str()?.eq_ignore_ascii_case("exe") { return None; }
    if !roots.iter().any(|root| candidate.ancestors().skip(1).any(|a|
        root.to_str().zip(a.to_str()).is_some_and(|(x,y)| x.eq_ignore_ascii_case(y))
    )) { return None; }
    Some(candidate.to_owned())
}

fn confirmed_absent(path: &Path) -> bool {
    let chain: Vec<_> = path.ancestors().collect();
    for part in chain.into_iter().rev() {
        match fs::symlink_metadata(part) {
            Ok(m) if m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 => return false,
            Ok(m) if part != path && !m.is_dir() => return false,
            Ok(_) => {}
            Err(e) => return e.kind() == ErrorKind::NotFound,
        }
    }
    false
}

pub fn snapshot() -> Result<Saved, String> {
    let roots = install_roots()?;
    let key = match HKCU.open_subkey_with_flags(RUN, KEY_QUERY_VALUE) {
        Ok(key) => key,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Saved::default()),
        Err(e) => return Err(e.to_string()),
    };
    let mut entries = Vec::new();
    for rec in key.enum_values_os_string() {
        let (name, raw) = rec.map_err(|e| e.to_string())?;
        let Some(name) = name.to_str().filter(|s| valid_name(s)) else { continue };
        let expandable = match raw.vtype { REG_SZ => false, REG_EXPAND_SZ => true, _ => continue };
        if exe_path(&raw.bytes, &roots).is_some_and(|p| confirmed_absent(&p)) {
            entries.push(Entry { name: name.to_owned(), expandable, bytes: raw.bytes.into_owned() });
        }
    }
    entries.sort_by(|a,b| a.name.cmp(&b.name));
    Ok(Saved(entries))
}

impl Entry {
    fn raw(&self) -> RegValue<'_> {
        RegValue { vtype: if self.expandable { REG_EXPAND_SZ } else { REG_SZ }, bytes: (&self.bytes[..]).into() }
    }

    fn identical_or_absent(&self, key: &RegKey) -> Result<bool, String> {
        match key.get_raw_value(&self.name) {
            Ok(raw) if raw.vtype == self.raw().vtype && raw.bytes.as_ref() == self.bytes => Ok(true),
            Ok(_) => Err(crate::local_format!("L’entrée « {} » a changé dans Windows.", "The entry “{}” has changed in Windows.", self.name)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Saved {
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn names(&self) -> Vec<String> { self.0.iter().map(|e| e.name.clone()).collect() }

    fn validate(&self) -> Result<(), String> {
        let roots = install_roots()?;
        let mut names = BTreeSet::new();
        for entry in &self.0 {
            if !valid_name(&entry.name) || !names.insert(entry.name.to_lowercase())
                || !exe_path(&entry.bytes, &roots).is_some_and(|p| confirmed_absent(&p)) {
                return Err(crate::local_format!("Le programme de « {} » est revenu, ou la sauvegarde ne correspond plus à un démarrage orphelin.", "The program for “{}” is present again or the backup no longer matches a missing startup program.", entry.name));
            }
        }
        Ok(())
    }

    pub fn check_restore(&self) -> Result<(), String> {
        self.validate()?;
        match HKCU.open_subkey_with_flags(RUN, KEY_QUERY_VALUE) {
            Ok(key) => { for e in &self.0 { e.identical_or_absent(&key)?; } Ok(()) }
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn apply(&self) -> Result<(), String> { self.update(false) }
    pub fn restore(&self) -> Result<(), String> { self.update(true) }

    fn update(&self, restore: bool) -> Result<(), String> {
        if self.is_empty() { return Ok(()); }
        self.validate()?;
        let tx = Transaction::new().map_err(|e| e.to_string())?;
        let flags = KEY_QUERY_VALUE | KEY_SET_VALUE;
        let key = if restore {
            HKCU.create_subkey_transacted_with_flags(RUN, &tx, flags).map(|p| p.0)
        } else { HKCU.open_subkey_transacted_with_flags(RUN, &tx, flags) }.map_err(|e| e.to_string())?;
        for e in &self.0 {
            let present = e.identical_or_absent(&key)?;
            if restore {
                if !present { key.set_raw_value(&e.name, &e.raw()).map_err(|e| e.to_string())?; }
            } else if present {
                key.delete_value(&e.name).map_err(|e| e.to_string())?;
            } else {
                return Err(crate::local_format!("L’entrée « {} » a déjà disparu. Actualise l’analyse.", "The entry “{}” is already gone. Refresh the scan.", e.name));
            }
        }
        drop(key);
        tx.commit().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn path(command: &str) -> Option<PathBuf> {
        let raw = command.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect::<Vec<_>>();
        exe_path(&raw, &[PathBuf::from(r"C:\Program Files")])
    }
    #[test]
    fn only_unambiguous_application_paths_are_accepted() {
        assert!(path(r#""C:\Program Files\App\agent.exe""#).is_some());
        for command in [r"C:\Program Files\App\agent.exe", r#""C:\Program Files\App.exe" --start"#,
            r#""C:\Program Files (other)\a.exe""#, r#""C:\Program Files\..\Windows\a.exe""#,
            r#""C:\Program Files\%name%\a.exe""#, r#""\\pc\Program Files\a.exe""#,
            r#""C:\Program Files\dir.\a.exe""#, r#""C:\Program Files\AUX.exe""#] {
            assert!(path(command).is_none(), "{command}");
        }
    }
    #[test]
    fn missing_file_is_distinguished_from_an_existing_file() {
        let file = std::env::current_exe().unwrap();
        assert!(!confirmed_absent(&file));
        // Existence checks do not create or modify any file.
        let root = PathBuf::from(std::env::var_os("SystemRoot").unwrap());
        assert!(confirmed_absent(&root.join("Ferrite11-missing-test-6c57d78f.exe")));
    }
}
