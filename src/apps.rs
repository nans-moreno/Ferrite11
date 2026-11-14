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
        .ok_or_else(|| "Le dossier des applications locales est indisponible.".into())
}
fn windows() -> PathBuf {
    PathBuf::from(env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
}
fn powershell() -> Command {
    let mut cmd = Command::new(windows().join(r"System32\WindowsPowerShell\v1.0\powershell.exe"));
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
fn create_shortcut(tool: &Tool) -> Result<()> {
    // Existing shortcuts belonging to the user must keep their own destination.
    let script = r#"
$ErrorActionPreference='Stop'
$dir=Join-Path ([Environment]::GetFolderPath('Programs')) 'Ferrite11'
[void][IO.Directory]::CreateDirectory($dir)
$path=Join-Path $dir ($env:FERRITE_TASK + '.lnk')
$shell=New-Object -ComObject WScript.Shell
$link=$shell.CreateShortcut($path)
$argsWanted='--outil ' + $env:FERRITE_ID
if ([IO.File]::Exists($path) -and ($link.Description -ne 'Lanceur Ferrite11' -or $link.Arguments -ne $argsWanted)) {
    throw 'Un autre raccourci utilise déjà ce nom.'
}
$link.TargetPath=$env:FERRITE_EXE
$link.Arguments=$argsWanted
$link.Description='Lanceur Ferrite11'
$link.Save()
"#;
    execute(
        powershell()
            .arg(script)
            .env("FERRITE_TASK", tool.task)
            .env("FERRITE_ID", tool.id)
            .env(
                "FERRITE_EXE",
                env::current_exe().map_err(|e| e.to_string())?,
            ),
    )
}
fn managed(root: &Path, tool: &Tool) -> bool {
    fs::read_to_string(root.join(tool.id).join(MARKER)).is_ok_and(|s| s == tool.id)
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
        return Err("Ce dossier existe déjà. Son contenu est conservé.".into());
    }
    let stage = new_stage(root, tool)?;
    let result = (|| {
        fetch(tool, &stage)?;
        for name in tool.files {
            if !stage.join(name).is_file() {
                return Err(format!("Fichier manquant : {name}"));
            }
        }
        fs::write(stage.join(MARKER), tool.id).map_err(|e| e.to_string())?;
        fs::rename(&stage, target)
            .map_err(|e| format!("Activation de {} impossible. {e}", tool.name))
    })();
    if result.is_err() {
        for name in tool.files.iter().copied().chain([MARKER, "download.tmp"]) {
            let _ = fs::remove_file(stage.join(name));
        }
        let _ = fs::remove_dir(stage);
    }
    result
}
fn rollback(root: &Path, tool: &Tool) -> Result<()> {
    let folder = root.join(tool.id);
    if !managed(root, tool) {
        return Err("Cette installation n’appartient pas à Ferrite11.".into());
    }
    // Acquire delete access to every file before touching the first one.
    // Keep the receipt until all owned binaries have been removed; retries are idempotent.
    let mut held = Vec::new();
    for name in tool.files.iter().copied().chain([MARKER]) {
        let handle = fs::OpenOptions::new()
            .access_mode(0x00010000)
            .share_mode(4)
            .open(folder.join(name));
        match handle {
            Ok(file) => held.push(file),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(format!("Ferme {} puis réessaie. {e}", tool.name)),
        }
    }
    for name in tool.files.iter().rev() {
        match fs::remove_file(folder.join(name)) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(format!("Ferme {} puis réessaie. {e}", tool.name)),
        }
    }
    fs::remove_file(folder.join(MARKER)).map_err(|e| e.to_string())?;
    drop(held);
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
                return Err("Le dossier des outils est redirigé.".into());
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
        .map_err(|_| "Une autre installation Ferrite11 est en cours.".to_string())?;
    if remove {
        rollback(&root, tool)?;
        return Ok(format!(
            "{} retiré. Le lanceur ouvre de nouveau {}.",
            tool.name, tool.original
        ));
    }
    install(&root, tool, download)?;
    match create_shortcut(tool) {
        Ok(()) => Ok(format!(
            "{} est prêt. Menu Démarrer › Ferrite11 › {}.",
            tool.name, tool.task
        )),
        Err(e) => Ok(format!(
            "{} installé. Utilise « Ouvrir » dans Ferrite11. Création du raccourci impossible : {e}",
            tool.name
        )),
    }
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
        "oculante" => {
            let mut c = Command::new(windows().join("explorer.exe"));
            c.arg("ms-photos:");
            c
        }
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
            .ok_or_else(|| "Outil inconnu.".to_string())
            .and_then(launch),
    )
}

#[derive(Default)]
pub struct Apps {
    pending: Option<Receiver<Result<String>>>,
    confirm: Option<(&'static str, bool)>,
    message: String,
}
impl Apps {
    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.pending else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Disconnected) => Err("Opération interrompue.".into()),
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(200));
                return;
            }
        };
        self.pending = None;
        self.message = result.unwrap_or_else(|e| format!("Opération impossible. {e}"));
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, simulation: bool) {
        ui.heading("Applications de remplacement");
        ui.label("Chaque installation crée un lanceur dans le menu Démarrer, sous Ferrite11. L’outil Windows d’origine reste accessible. Après le retour arrière, le même lanceur ouvre de nouveau l’outil Windows.");
        if simulation {
            ui.colored_label(egui::Color32::LIGHT_BLUE, "Mode simulation actif. Décoche-le en bas pour installer ou annuler un remplacement.");
            self.confirm = None;
        }
        if self.pending.is_some() {
            ui.spinner();
        }
        ui.strong(&self.message);
        let root = match local_root() {
            Ok(r) => r,
            Err(e) => {
                ui.label(e);
                return;
            }
        };
        for tool in TOOLS {
            let active = managed(&root, tool);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading(tool.task);
                ui.label(format!("{} → {}   ·   En service : {}",tool.original,tool.name,if active {tool.name} else {tool.original}));
                ui.hyperlink_to(format!("Version {}",tool.release.rsplit('/').next().unwrap()), tool.url(false));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Ouvrir").clicked() {
                        if let Err(e)=launch(tool) { self.message = format!("Ouverture impossible. {e}"); }
                    }
                    let action = if active {"Retour arrière"} else {"Installer et remplacer"};
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new(action)).clicked() {
                        if simulation {
                            self.message = format!("Simulation · {} : {}. Décoche « Mode simulation » pour effectuer cette action.",tool.name,action);
                        } else { self.confirm=Some((tool.id,active)); }
                    }
                });
                if self.confirm == Some((tool.id,active)) {
                    if active { ui.label(format!("Retirer la copie de {} installée par Ferrite11 et revenir à {} ?",tool.name,tool.original)); }
                    else { ui.label(format!("Télécharger {} depuis son projet officiel et l’utiliser pour « {} » ?",tool.name,tool.task)); }
                    ui.horizontal(|ui| {
                        if ui.button("Valider").clicked() {
                            let (tx,rx)=mpsc::channel();
                            self.pending=Some(rx); self.confirm=None;
                            self.message=format!("{} · opération en cours…",tool.name);
                            let ctx=ui.ctx().clone();
                            std::thread::spawn(move || { let _=tx.send(transact(tool,active)); ctx.request_repaint(); });
                        }
                        if ui.button("Annuler").clicked() { self.confirm=None; }
                    });
                }
            });
        }
        ui.heading("Ouvrir les fichiers par double-clic");
        ui.label("Pour un type de fichier, note l’application actuelle dans les paramètres Windows. Utilise ensuite « Ouvrir avec » sur un fichier pour choisir le nouvel exécutable. Pour annuler, resélectionne l’application précédente avant de retirer la copie portable.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Applications par défaut").clicked() {
                if let Err(e) = Command::new(windows().join("explorer.exe"))
                    .arg("ms-settings:defaultapps")
                    .spawn()
                {
                    self.message = e.to_string();
                }
            }
            if root.is_dir() && ui.button("Afficher les exécutables installés").clicked() {
                if let Err(e) = Command::new(windows().join("explorer.exe"))
                    .arg(&root)
                    .spawn()
                {
                    self.message = e.to_string();
                }
            }
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
