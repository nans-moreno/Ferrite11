use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, fs, io, path::PathBuf};

#[path = "english.rs"]
mod english_text;

fn folder() -> io::Result<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("Ferrite11"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA"))
}
static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn load() {
    let english = folder()
        .and_then(|dir| fs::read_to_string(dir.join("language.txt")))
        .is_ok_and(|language| language.trim() == "en");
    ENGLISH.store(english, Ordering::Relaxed);
}

pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

pub fn select(english: bool) -> std::io::Result<()> {
    ENGLISH.store(english, Ordering::Relaxed);
    let dir = folder()?;
    fs::create_dir_all(&dir)?;
    let temp = dir.join(format!("language-{}.tmp", std::process::id()));
    fs::write(&temp, if english { "en" } else { "fr" })?;
    let result = fs::rename(&temp, dir.join("language.txt"));
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

pub fn text(source: &str) -> &str {
    if english() {
        english_text::lookup(source)
    } else {
        source
    }
}

#[macro_export]
macro_rules! local_format {
    ($fr:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::language::english() { format!($en $(, $arg)*) }
        else { format!($fr $(, $arg)*) }
    };
}
