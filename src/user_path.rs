use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;
use winreg::enums::{KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SZ};
use winreg::{HKCU, RegValue, transaction::Transaction};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedPath {
    text: String,
    expandable: bool,
}

fn clean(text: &str) -> String {
    // Quoted paths may contain semicolons. Leave that representation intact.
    if text.contains('"') {
        return text.to_owned();
    }
    let mut seen = BTreeSet::new();
    text.split(';').filter(|entry| {
        !Path::new(entry).is_absolute() || entry.contains('%') || seen.insert(*entry)
    }).collect::<Vec<_>>().join(";")
}

pub fn snapshot() -> Result<Option<SavedPath>, String> {
    let result = HKCU.open_subkey_with_flags("Environment", KEY_QUERY_VALUE)
        .and_then(|key| key.get_raw_value("Path"));
    let raw = match result {
        Ok(raw) => raw,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    decode(raw).map(Some)
}

fn decode(raw: RegValue) -> Result<SavedPath, String> {
    let expandable = match raw.vtype {
        REG_EXPAND_SZ => true, REG_SZ => false,
        _ => return Err(crate::language::text("Type de PATH non pris en charge.").into()),
    };
    if raw.bytes.len() % 2 != 0 { return Err(crate::language::text("Encodage du PATH inconnu.").into()); }
    let mut units: Vec<_> = raw.bytes.chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
    if units.pop() != Some(0) || units.contains(&0) {
        return Err(crate::language::text("Format du PATH non pris en charge.").into());
    }
    let text = String::from_utf16(&units).map_err(|e| e.to_string())?;
    Ok(SavedPath { text, expandable })
}

pub fn satisfied() -> Result<bool, String> {
    Ok(snapshot()?.is_none_or(|saved| clean(&saved.text) == saved.text))
}

impl SavedPath {
    fn cleaned(&self) -> Self {
        Self { text: clean(&self.text), expandable: self.expandable }
    }

    pub fn check_restore(&self) -> Result<(), String> {
        let current = snapshot()?;
        if current.as_ref() == Some(self) || current == Some(self.cleaned()) { Ok(()) }
        else { Err(crate::language::text("Le PATH a changé depuis la sauvegarde. Les ajouts récents sont conservés.").into()) }
    }

    pub fn update(&self, restore: bool) -> Result<(), String> {
        let cleaned = self.cleaned();
        let (expected, desired) = if restore { (&cleaned, self) } else { (self, &cleaned) };
        let tx = Transaction::new().map_err(|e| e.to_string())?;
        let key = HKCU.open_subkey_transacted_with_flags("Environment", &tx, KEY_SET_VALUE | KEY_QUERY_VALUE)
            .map_err(|e| e.to_string())?;
        let current = decode(key.get_raw_value("Path").map_err(|e| e.to_string())?)?;
        if &current == desired { return Ok(()); }
        if &current != expected { return Err(crate::language::text("Le PATH a changé. Relance l’analyse.").into()); }
        let bytes: Vec<_> = desired.text.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect();
        key.set_raw_value("Path", &RegValue {
            vtype: if desired.expandable { REG_EXPAND_SZ } else { REG_SZ }, bytes: bytes.into()
        }).map_err(|e| e.to_string())?;
        drop(key);
        tx.commit().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_identical_literal_absolute_entries_are_removed() {
        assert_eq!(clean(r"C:\Bin;rel;;C:\Bin;%P%;rel;c:\Bin;%P%;"), r"C:\Bin;rel;;%P%;rel;c:\Bin;%P%;");
        let quoted = r#""C:\a;b";C:\Bin;C:\Bin"#;
        assert_eq!(clean(quoted), quoted);
    }
}
