use eframe::egui;
use std::{
    env, fs, io,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Tool {
    id: &'static str,
    name: &'static str,
    task: &'static str,
    original: &'static str,
    release: &'static str,
    asset: &'static str,
    hash: &'static str,
    files: &'static [&'static str],
}

const TOOLS: &[Tool] = &[
    Tool {
        id: "yazi",
        name: "Yazi",
        task: "Parcourir les fichiers",
        original: "Explorateur de fichiers",
        release: "sxyazi/yazi/v26.5.6",
        asset: "yazi-x86_64-pc-windows-msvc.zip",
        hash: "6C6C52A4B2648E179F917BDAA7C57E793D18561B380A8BFA025F10CD1B9B2AD1",
        files: &["yazi.exe", "ya.exe"],
    },
    Tool {
        id: "kwil",
        name: "Kwil (bêta)",
        task: "Lire et modifier du texte, y compris les gros journaux",
        original: "Bloc-notes",
        release: "rennerdo30/kwil/v0.1.0-beta.8",
        asset: "kwil-0.1.0-beta.8-windows-x64.zip",
        hash: "c8cb06aa3332d70c51967c2a3648452ecd0823e03bdd6502ae924240c9b4f510",
        files: &[
            "kwil.exe",
            "kwil.com",
            "LICENSE.txt",
            "THIRD-PARTY-NOTICES.txt",
        ],
    },
    Tool {
        id: "pdfcraft",
        name: "PdfCraft",
        task: "Lire et annoter les documents PDF",
        original: "Lecteur PDF de Microsoft Edge",
        release: "storytold/pdfcraft/v0.21.1",
        asset: "pdfcraft-0.21.1-windows-x64-portable.zip",
        hash: "dcbcc2ddc019d85cca90a8242d5e4dc380f1ff6bfb6b951a89152d1dbae14cb4",
        files: &[
            "pdfcraft.exe",
            "LICENSE-APACHE",
            "LICENSE-document.txt",
            "LICENSE-MIT",
            "NOTICE",
            "OFL-biz-ud-mincho.txt",
            "OFL-biz-ud-pgothic.txt",
            "OFL-lxgw-wenkai-lite.txt",
            "OFL-nanum-gothic.txt",
            "OFL-noto-sans-arabic.txt",
            "OFL-noto-sans-cjk-sc.txt",
            "OFL-noto-sans-devanagari.txt",
            "OFL-noto-sans-georgian.txt",
            "OFL-noto-sans-hebrew.txt",
            "OFL-noto-sans-symbols-2.txt",
            "OFL-noto-sans-symbols.txt",
            "OFL-noto-sans-thai.txt",
            "OFL-shippori-mincho.txt",
            "pdfcraft-cli.exe",
            "portable.txt",
            "README.md",
            "models/ATTRIBUTION.txt",
            "models/text-detection.onnx",
            "models/text-detection.onnx.LICENCE.txt",
            "models/text-recognition.onnx",
            "models/text-recognition.onnx.LICENCE.txt",
        ],
    },
    Tool {
        id: "micro",
        name: "Micro",
        task: "Éditer du texte",
        original: "Bloc-notes",
        release: "zyedidia/micro/v2.0.14",
        asset: "micro-2.0.14-win64.zip",
        hash: "ef95e181249d0547942558d65b73cf484d6e06490bcfc0c540085a04f1ee00f8",
        files: &["micro.exe"],
    },
    Tool {
        id: "oculante",
        name: "Oculante",
        task: "Visionner des images",
        original: "Photos Microsoft",
        release: "woelper/oculante/0.9.6",
        asset: "oculante.exe",
        hash: "85bf65b56faa3f353384a9cafbca83e969d73cf7d6d0e0c10e5cd85c0070a6d9",
        files: &["oculante.exe"],
    },
    Tool {
        id: "archiverclick",
        name: "ArchiveRclick",
        task: "Parcourir et extraire les archives ZIP et 7z",
        original: "Dossiers compressés de l’Explorateur",
        release: "kirinonakar/ArchiveRclick/v1.3.2",
        asset: "ArchiveRclick-v1.3.2.zip",
        hash: "b26aca2d1c32bc48fff2934edacf89ed68119b67d470cdedde3233f6b695da7c",
        files: &[
            "archive-rclick.exe",
            "7z.dll",
            "AppxManifest.xml",
            "Assets/FileAssociationLogo.png",
            "Assets/FileAssociationLogo.scale-200.png",
            "Assets/FileAssociationLogo.scale-400.png",
            "Assets/Square150x150Logo.png",
            "Assets/Square150x150Logo.scale-200.png",
            "Assets/Square44x44Logo.png",
            "Assets/Square44x44Logo.scale-200.png",
            "Assets/Square44x44Logo.scale-400.png",
            "Assets/Square44x44Logo.targetsize-16.png",
            "Assets/Square44x44Logo.targetsize-16_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-16_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-20.png",
            "Assets/Square44x44Logo.targetsize-20_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-20_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-24.png",
            "Assets/Square44x44Logo.targetsize-24_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-24_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-256.png",
            "Assets/Square44x44Logo.targetsize-256_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-256_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-30.png",
            "Assets/Square44x44Logo.targetsize-30_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-30_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-32.png",
            "Assets/Square44x44Logo.targetsize-32_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-32_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-36.png",
            "Assets/Square44x44Logo.targetsize-36_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-36_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-40.png",
            "Assets/Square44x44Logo.targetsize-40_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-40_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-44.png",
            "Assets/Square44x44Logo.targetsize-44_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-44_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-48.png",
            "Assets/Square44x44Logo.targetsize-48_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-48_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-60.png",
            "Assets/Square44x44Logo.targetsize-60_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-60_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-64.png",
            "Assets/Square44x44Logo.targetsize-64_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-64_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-72.png",
            "Assets/Square44x44Logo.targetsize-72_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-72_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-80.png",
            "Assets/Square44x44Logo.targetsize-80_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-80_altform-unplated.png",
            "Assets/Square44x44Logo.targetsize-96.png",
            "Assets/Square44x44Logo.targetsize-96_altform-lightunplated.png",
            "Assets/Square44x44Logo.targetsize-96_altform-unplated.png",
            "Assets/StoreLogo.png",
            "Assets/StoreLogo.scale-200.png",
            "CLI.md",
            "SHA256SUMS",
            "THIRD-PARTY-LICENSES.md",
            "THIRD-PARTY-NOTICES.md",
            "archive-rclick-cli.exe",
            "archive.dll",
            "archive_rclick_core.dll",
            "bz2.dll",
            "liblzma.dll",
            "licenses/7zip.txt",
            "licenses/bzip2.txt",
            "licenses/flate2-apache.txt",
            "licenses/flate2-mit.txt",
            "licenses/libarchive.txt",
            "licenses/liblzma.txt",
            "licenses/lz4.txt",
            "licenses/zip.txt",
            "licenses/zlib-ng.txt",
            "licenses/zlib-rs.txt",
            "licenses/zlib.txt",
            "licenses/zstd.txt",
            "lz4.dll",
            "resources.pri",
            "resources.scale-200.pri",
            "resources.scale-400.pri",
            "vcruntime140.dll",
            "z.dll",
            "zstd.dll",
        ],
    },
    Tool {
        id: "leanrows",
        name: "LeanRows",
        task: "Consulter de grands tableaux et des données ligne par ligne, en lecture seule",
        original: "L’application actuelle des fichiers de données",
        release: "abooodbah/leanrows/v0.6.0",
        asset: "LeanRows-v0.6.0-windows-x64-unsigned.zip",
        hash: "72d9bb2bd927dbaf299990000ed875b0a5508f3b846997933b5674514acf786b",
        files: &[
            "leanrows.exe",
            "LICENSE.txt",
            "UNSIGNED.txt",
            "manifest.json",
        ],
    },
    Tool {
        id: "kitab",
        name: "Kitab",
        task: "Lire des livres numériques et des bandes dessinées",
        original: "La liseuse actuelle",
        release: "t21dev/kitab/v0.5.0",
        asset: "Kitab_0.5.0_x64_portable.zip",
        hash: "d725671ae896a7a74eee45bd93938093d2c8ca7110d10a161256214ec24361f2",
        files: &[
            "kitab.exe",
            "kitab_shell.dll",
            "LICENSE.txt",
            "NOTICE.txt",
            "portable",
        ],
    },
    Tool {
        id: "bottom",
        name: "bottom",
        task: "Suivre les ressources",
        original: "Gestionnaire des tâches",
        release: "ClementTsang/bottom/0.14.9",
        asset: "bottom_x86_64-pc-windows-msvc.zip",
        hash: "01660721b0fdc72b191f2b27b78dc9a3087aeb7b3fad4a890c53870089d6b500",
        files: &["btm.exe"],
    },
    Tool {
        id: "rg",
        name: "ripgrep",
        task: "Chercher du texte dans des fichiers",
        original: "findstr",
        release: "BurntSushi/ripgrep/15.1.0",
        asset: "ripgrep-15.1.0-x86_64-pc-windows-msvc.zip",
        hash: "124510B94B6BAA3380D051FDF4650EAA80A302C876D611E9DBA0B2E18D87493A",
        files: &["rg.exe"],
    },
    Tool {
        id: "fd",
        name: "fd",
        task: "Retrouver un fichier par son nom",
        original: "where",
        release: "sharkdp/fd/v10.3.0",
        asset: "fd-v10.3.0-x86_64-pc-windows-msvc.zip",
        hash: "318AA2A6FA664325933E81FDA60D523FFF29444129E91EBF0726B5B3BCD8B059",
        files: &["fd.exe"],
    },
];

impl Tool {
    fn url(&self, download: bool) -> String {
        let (project, tag) = self.release.rsplit_once('/').unwrap();
        if download {
            format!(
                "https://github.com/{project}/releases/download/{tag}/{}",
                self.asset
            )
        } else {
            format!("https://github.com/{project}/releases/tag/{tag}")
        }
    }
}

type Result<T> = std::result::Result<T, String>;
const MARKER: &str = "ferrite11-installation.txt";

fn local_root() -> Result<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("Ferrite11").join("outils"))
        .ok_or_else(|| crate::language::text("Le dossier des applications locales est indisponible.").into())
}
fn windows() -> PathBuf {
    PathBuf::from(env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
}
fn powershell() -> Command {
    let mut cmd = Command::new(windows().join(r"System32\WindowsPowerShell\v1.0\powershell.exe"));
    cmd.env("FERRITE_LANGUAGE", if crate::language::english() { "en" } else { "fr" });
    cmd.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
        .creation_flags(0x08000000);
    cmd
}
fn execute(cmd: &mut Command) -> Result<()> {
    let output = cmd.output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .chars()
            .take(550)
            .collect())
    }
}
fn download(tool: &Tool, stage: &Path) -> Result<()> {
    execute(
        powershell()
            .arg(include_str!("portable.ps1"))
            .env("FERRITE_STAGE", stage)
            .env("FERRITE_URL", tool.url(true))
            .env("FERRITE_HASH", tool.hash)
            .env("FERRITE_FILES", tool.files.join(",")),
    )
}
fn remove_old_shortcut(tool: &Tool) -> Result<()> {
    let script = r#"
$ErrorActionPreference='Stop'
$dir=Join-Path ([Environment]::GetFolderPath('Programs')) 'Ferrite11'
$path=Join-Path $dir ($env:FERRITE_TASK + '.lnk')
if ([IO.File]::Exists($path)) {
    $lnk=(New-Object -ComObject WScript.Shell).CreateShortcut($path)
    if ($lnk.Description -eq 'Lanceur Ferrite11' -and $lnk.Arguments -eq ('--outil ' + $env:FERRITE_ID) -and [IO.Path]::GetFileName($lnk.TargetPath) -eq 'Ferrite11.exe') { [IO.File]::Delete($path) }
}
"#;
    execute(
        powershell()
            .arg(script)
            .env("FERRITE_TASK", tool.task)
            .env("FERRITE_ID", tool.id),
    )
}
fn managed(root: &Path, tool: &Tool) -> bool {
    fs::read_to_string(root.join(tool.id).join(MARKER)).is_ok_and(|s| s == tool.id)
}
// Only remove empty directories explicitly listed in the pinned asset manifest.
fn remove_empty_asset_dirs(folder: &Path, tool: &Tool) {
    for name in tool.files {
        if let Some(parent) = Path::new(name)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            let _ = fs::remove_dir(folder.join(parent));
        }
    }
}

fn new_stage(root: &Path, tool: &Tool) -> Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let stage = root.join(format!(".{}-{}-{stamp}", tool.id, std::process::id()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    Ok(stage)
}
fn install(root: &Path, tool: &Tool, fetch: impl FnOnce(&Tool, &Path) -> Result<()>) -> Result<()> {
    let target = root.join(tool.id);
    if target.try_exists().map_err(|e| e.to_string())? {
        return Err(crate::language::text("Ce dossier existe déjà. Son contenu est conservé.").into());
    }
    let stage = new_stage(root, tool)?;
    let result = (|| {
        fetch(tool, &stage)?;
        for name in tool.files {
            if !stage.join(name).is_file() {
                return Err(crate::local_format!("Fichier manquant : {name}", "Missing file: {name}"));
            }
        }
        fs::write(stage.join(MARKER), tool.id).map_err(|e| e.to_string())?;
        fs::rename(&stage, target)
            .map_err(|e| crate::local_format!("Activation de {} impossible. {e}", "Could not activate {}. {e}", tool.name))
    })();
    if result.is_err() {
        for name in tool.files.iter().copied().chain([MARKER, "download.tmp"]) {
            let _ = fs::remove_file(stage.join(name));
        }
        remove_empty_asset_dirs(&stage, tool);
        let _ = fs::remove_dir(stage);
    }
    result
}
fn rollback(root: &Path, tool: &Tool) -> Result<()> {
    let folder = root.join(tool.id);
    if !managed(root, tool) {
        return Err(crate::language::text("Cette installation n’appartient pas à Ferrite11.").into());
    }
    // Acquire delete access to every file before touching the first one.
    // Keep the receipt until all owned binaries have been removed; retries are idempotent.
    for name in tool.files {
        if let Some(parent) = Path::new(name)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            match fs::symlink_metadata(folder.join(parent)) {
                Ok(meta) if meta.file_attributes() & 0x400 != 0 => {
                    return Err(crate::language::text("Sous-dossier redirigé.").into());
                }
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.to_string()),
                _ => (),
            }
        }
    }
    let mut held = Vec::new();
    for name in tool.files.iter().copied().chain([MARKER]) {
        let handle = fs::OpenOptions::new()
            .access_mode(0x00010000)
            .share_mode(4)
            .open(folder.join(name));
        match handle {
            Ok(file) => held.push(file),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(crate::local_format!("Ferme {} puis réessaie. {e}", "Close {} and try again. {e}", tool.name)),
        }
    }
    for name in tool.files.iter().rev() {
        match fs::remove_file(folder.join(name)) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(crate::local_format!("Ferme {} puis réessaie. {e}", "Close {} and try again. {e}", tool.name)),
        }
    }
    fs::remove_file(folder.join(MARKER)).map_err(|e| e.to_string())?;
    drop(held);
    remove_empty_asset_dirs(&folder, tool);
    match fs::remove_dir(folder) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
fn transact(tool: &Tool, remove: bool) -> Result<String> {
    let root = local_root()?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    for folder in [root.parent().unwrap(), &root, &root.join(tool.id)] {
        match fs::symlink_metadata(folder) {
            Ok(m) if m.file_attributes() & 0x400 != 0 => {
                return Err(crate::language::text("Le dossier des outils est redirigé.").into());
            }
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.to_string()),
            _ => (),
        }
    }
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join("installation.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|_| crate::language::text("Une autre installation Ferrite11 est en cours.").to_string())?;
    let exe = root.join(tool.id).join(tool.files[0]);
    let native = !crate::associations::extensions(tool.id).is_empty();
    if remove {
        if native {
            crate::associations::ensure_unused(tool.id, &exe).map_err(|e| e.to_string())?;
            crate::associations::unregister(tool.id).map_err(|e| e.to_string())?;
        }
        remove_old_shortcut(tool)?;
        rollback(&root, tool)?;
        return Ok(crate::local_format!("Copie de {} retirée.", "Removed the managed copy of {}.", tool.name));
    }
    if !native {
        return Err(crate::language::text("Aucune association native disponible.").into());
    }
    if !managed(&root, tool) {
        install(&root, tool, download)?;
    }
    if !exe.is_file() {
        return Err(crate::language::text("Copie incomplète. Retire puis réinstalle l’outil.").into());
    }
    crate::associations::register(tool.id, tool.name, &exe).map_err(|e| e.to_string())?;
    remove_old_shortcut(tool)?;
    let page = format!(
        "ms-settings:defaultapps?registeredAppUser=Ferrite11.{}",
        tool.id
    );
    if crate::shell::open_target(&page).is_err() {
        crate::shell::open_target("ms-settings:defaultapps").map_err(|e| {
            crate::local_format!("{} est disponible. Ouvre Paramètres → Applications → Applications par défaut. {e}", "{} is available. Open Settings → Apps → Default apps. {e}",
                tool.name
            )
        })?;
    }
    Ok(crate::local_format!("Sélectionne {} dans Windows pour les extensions voulues, puis actualise la liste.", "Choose {} for the file types you want in Windows, then refresh the list.",
        tool.name
    ))
}

fn launch(tool: &Tool) -> Result<()> {
    let root = local_root()?;
    let active = managed(&root, tool);
    let bin = root.join(tool.id).join(tool.files[0]);
    let mut cmd = match tool.id {
        "rg" | "fd" => {
            let mut cmd =
                Command::new(windows().join(r"System32\WindowsPowerShell\v1.0\powershell.exe"));
            let init = if active && tool.id == "rg" {
                "Set-Alias rg $env:FERRITE_BIN; rg --version; Write-Host 'Exemple : rg texte .'"
            } else if active {
                "Set-Alias fd $env:FERRITE_BIN; fd --version; Write-Host 'Exemple : fd photo .'"
            } else if tool.id == "rg" {
                "findstr.exe /?"
            } else {
                "where.exe /?"
            };
            cmd.args(["-NoProfile", "-NoExit", "-Command", init])
                .env("FERRITE_BIN", &bin)
                .creation_flags(0x10);
            cmd
        }
        _ if active => {
            let mut cmd = Command::new(bin);
            if tool.id != "oculante" {
                cmd.creation_flags(0x10);
            }
            cmd
        }
        "oculante" => return crate::shell::open_target("ms-photos:"),
        "bottom" => Command::new(windows().join(r"System32\Taskmgr.exe")),
        "micro" => Command::new(windows().join(r"System32\notepad.exe")),
        _ => Command::new(windows().join("explorer.exe")),
    };
    if let Some(profile) = env::var_os("USERPROFILE") {
        cmd.current_dir(profile);
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}
pub fn launch_from_args() -> Option<Result<()>> {
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--outil")) {
        return None;
    }
    let id = args.next();
    Some(
        TOOLS
            .iter()
            .find(|t| Some(std::ffi::OsStr::new(t.id)) == id.as_deref())
            .ok_or_else(|| crate::language::text("Outil inconnu.").to_string())
            .and_then(launch),
    )
}

#[derive(Default)]
pub struct Apps {
    pending: Option<Receiver<Result<String>>>,
    confirm: Option<(&'static str, bool)>,
    message: String,
    handlers: std::collections::BTreeMap<(&'static str, &'static str), (String, Option<String>)>,
}
impl Apps {
    pub fn language_changed(&mut self) { self.message.clear(); self.handlers.clear(); }
    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.pending else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Disconnected) => Err(crate::language::text("Opération interrompue.").into()),
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(200));
                return;
            }
        };
        self.pending = None;
        self.handlers.clear();
        self.message = result.unwrap_or_else(|e| crate::local_format!("Opération impossible. {e}", "Operation failed. {e}"));
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, simulation: bool) {
        ui.heading(crate::language::text("Applications par défaut de Windows"));
        ui.label(crate::language::text("Windows ouvre directement les fichiers avec le programme choisi. Ferrite11 peut être fermé."));
        ui.label(crate::language::text("L’application utilisée pour le double-clic est indiquée pour chaque extension. Les gains de mémoire ou de temps d’ouverture dépendent de ton usage."));
        if ui.button(crate::language::text("Actualiser les associations")).clicked() {
            self.handlers.clear();
        }
        if simulation {
            self.confirm = None;
            ui.label(crate::language::text("Mode simulation actif."));
        }
        if self.pending.is_some() {
            ui.spinner();
        }
        ui.label(&self.message);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading(crate::language::text("Gestionnaire des tâches · FastTask"));
            ui.label(crate::language::text("Ctrl + Maj + Échap ouvrira FastTask pour tous les utilisateurs de ce PC. Windows demandera une autorisation administrateur. Rétablis le gestionnaire Windows avant de désinstaller FastTask."));
            ui.add_enabled_ui(!simulation && self.pending.is_none(), |ui| {
                if ui.button(crate::language::text("Remplacer par FastTask")).clicked() {
                    self.confirm = Some(("fasttask", false));
                }
                if ui.button(crate::language::text("Rétablir le Gestionnaire des tâches Windows")).clicked() {
                    self.confirm = Some(("fasttask", true));
                }
                if let Some(("fasttask", restore)) = self.confirm {
                    ui.label(if restore { crate::language::text("Rendre le raccourci au Gestionnaire des tâches Windows ?") } else { crate::language::text("Installer FastTask 1.1.0 si nécessaire et lui confier le raccourci système ?") });
                    if ui.button(crate::language::text("Valider")).clicked() {
                        self.confirm = None;
                        let (tx, rx) = mpsc::channel();
                        self.pending = Some(rx);
                        self.message = crate::language::text("En attente de la réponse de Windows…").into();
                        let ctx = ui.ctx().clone();
                        std::thread::spawn(move || {
                            let result = execute(powershell().arg(include_str!("fasttask.ps1"))
                                .env("FERRITE_TASK_ACTION", if restore { "restore" } else { "replace" }))
                                .map(|_| if restore { crate::language::text("Ctrl + Maj + Échap ouvre le Gestionnaire des tâches Windows.").into() } else { crate::language::text("FastTask est le gestionnaire actif. Essaie Ctrl + Maj + Échap.").into() });
                            let _ = tx.send(result);
                            ctx.request_repaint();
                        });
                    }
                    if ui.button(crate::language::text("Annuler")).clicked() { self.confirm = None; }
                }
            });
        });
        let root = match local_root() {
            Ok(root) => root,
            Err(e) => {
                ui.label(e);
                return;
            }
        };
        for tool in TOOLS {
            let active = managed(&root, tool);
            let formats = crate::associations::extensions(tool.id);
            if (formats.is_empty() || tool.id == "micro") && !active {
                continue;
            }
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.heading(crate::language::text(tool.name));
                if tool.id == "micro" { ui.label(crate::language::text("Ancienne installation de Micro (Go). Tu peux rétablir ton choix dans Windows ou choisir Kwil pour le texte.")); }
                if !formats.is_empty() { ui.label(crate::local_format!("Usage : {} · Alternative à {}", "Use: {} · Alternative to {}",crate::language::text(tool.task),crate::language::text(tool.original))); }
                if formats.is_empty() { ui.label(crate::language::text("Ancienne installation par raccourci. Tu peux la retirer.")); }
                for &ext in formats {
                    let (name,previous) = self.handlers.entry((tool.id,ext)).or_insert_with(|| (crate::associations::handler(ext),crate::associations::previous(tool.id,ext)));
                    ui.label(crate::local_format!("{ext} · Application actuelle : {name}", "{ext} · Current app: {name}"));
                    if let Some(old) = previous { ui.small(crate::local_format!("Application précédente : {old}", "Previous app: {old}")); }
                }
                ui.add_enabled_ui(!simulation && self.pending.is_none(), |ui| {
                    if !formats.is_empty() && ui.button(if active { crate::language::text("Choisir ou rétablir dans Windows") } else { crate::language::text("Installer et choisir les extensions") }).clicked() {
                        self.confirm=Some((tool.id,false));
                    }
                    if active && ui.button(crate::language::text("Retirer cette installation")).clicked() { self.confirm=Some((tool.id,true)); }
                    if let Some((id,remove)) = self.confirm && id == tool.id {
                        ui.label(if remove { crate::language::text("Rétablis d’abord les extensions dans Windows avant de retirer la copie.") } else { crate::language::text("Windows te demandera quels types de fichiers confier à cette application.") });
                        if ui.button(crate::language::text("Valider")).clicked() {
                            self.confirm=None;
                            let (tx,rx)=mpsc::channel();
                            self.pending=Some(rx);
                            self.message=crate::local_format!("{} · opération en cours…", "{} · working…",crate::language::text(tool.name));
                            let ctx=ui.ctx().clone();
                            std::thread::spawn(move || { let _=tx.send(transact(tool,remove)); ctx.request_repaint(); });
                        }
                        if ui.button(crate::language::text("Annuler")).clicked() { self.confirm=None; }
                    }
                });
            });
        }
        ui.collapsing(crate::language::text("Changer le gestionnaire de dossiers pour Win + E"), |ui| {
            ui.label(crate::language::text("Files propose dans ses paramètres avancés une option expérimentale de gestionnaire par défaut. Sauvegarde les réglages comme indiqué par l’éditeur, active cette option, puis teste Win + E et l’ouverture d’un dossier. Désactive-la avant de retirer Files. Explorer continue de gérer le bureau et la barre des tâches."));
            ui.hyperlink_to(crate::language::text("Fonctionnement et limites chez l’éditeur de Files"), "https://files.community/docs/customize-settings/advanced");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> PathBuf {
        // Run these filesystem tests on the build volume. A virtual Windows TEMP may
        // accept file writes yet reject atomic directory renames.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        fs::create_dir_all(&dir).unwrap();
        new_stage(&dir, &TOOLS[0]).unwrap()
    }

    fn fetch_test_files(tool: &Tool, dir: &Path) -> Result<()> {
        for name in tool.files {
            fs::write(dir.join(name), b"test portable").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[test]
    fn install_and_undo_preserve_files_from_other_sources() {
        let root = workspace();
        let tool = &TOOLS[0];
        let other = root.join("existing-yazi.exe");
        fs::write(&other, "installed before Ferrite11").unwrap();
        install(&root, tool, fetch_test_files).unwrap();
        assert!(managed(&root, tool));
        assert!(install(&root, tool, |_, _| panic!("duplicate download")).is_err());
        let extra = root.join(tool.id).join("notes.txt");
        fs::write(&extra, "mine").unwrap();
        rollback(&root, tool).unwrap();
        assert!(!managed(&root, tool));
        assert!(!root.join(tool.id).join(tool.files[0]).exists());
        assert_eq!(
            fs::read_to_string(&other).unwrap(),
            "installed before Ferrite11"
        );
        assert_eq!(fs::read_to_string(&extra).unwrap(), "mine");
        assert!(rollback(&root, tool).is_err());
        fs::remove_file(extra).unwrap();
        fs::remove_dir(root.join(tool.id)).unwrap();
        fs::remove_file(other).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn interrupted_install_leaves_destination_untouched_and_can_retry() {
        let root = workspace();
        let tool = &TOOLS[0];
        assert!(
            install(&root, tool, |_, dir| {
                fs::write(dir.join("download.tmp"), b"partial").unwrap();
                Err("network interrupted".into())
            })
            .is_err()
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        install(&root, tool, fetch_test_files).unwrap();
        rollback(&root, tool).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn unrelated_destination_is_never_replaced_or_removed() {
        let root = workspace();
        let tool = &TOOLS[0];
        let folder = root.join(tool.id);
        fs::create_dir(&folder).unwrap();
        let old = folder.join(tool.files[0]);
        fs::write(&old, "user installation").unwrap();
        assert!(install(&root, tool, |_, _| panic!("download should not run")).is_err());
        assert!(rollback(&root, tool).is_err());
        assert_eq!(fs::read_to_string(&old).unwrap(), "user installation");
        fs::remove_file(old).unwrap();
        fs::remove_dir(folder).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn locked_helper_keeps_main_executable_available_until_retry() {
        let root = workspace();
        let tool = &TOOLS[0];
        install(&root, tool, fetch_test_files).unwrap();
        let busy = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(root.join(tool.id).join(tool.files[1]))
            .unwrap();
        assert!(rollback(&root, tool).is_err());
        assert!(root.join(tool.id).join(tool.files[0]).is_file());
        assert!(managed(&root, tool));
        drop(busy);
        rollback(&root, tool).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    #[ignore = "requires verified upstream portable binaries"]
    fn round_trip_with_upstream_binaries() {
        let fixtures = PathBuf::from(env::var_os("FERRITE11_VERIFIED_BINARIES").unwrap());
        let root = workspace();
        for tool in TOOLS {
            install(&root, tool, |tool, stage| {
                for name in tool.files {
                    fs::create_dir_all(stage.join(name).parent().unwrap())
                        .map_err(|e| e.to_string())?;
                    fs::copy(fixtures.join(tool.id).join(name), stage.join(name))
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            })
            .unwrap();
            for name in tool.files {
                assert_eq!(
                    fs::read(root.join(tool.id).join(name)).unwrap(),
                    fs::read(fixtures.join(tool.id).join(name)).unwrap()
                );
            }
            rollback(&root, tool).unwrap();
            assert!(!root.join(tool.id).exists());
        }
        fs::remove_dir(root).unwrap();
    }
}
