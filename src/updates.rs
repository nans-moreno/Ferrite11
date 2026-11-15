use std::{
    env,
    io::{BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
};

use eframe::egui;

enum Event {
    Download(String),
    Current,
    Ready,
    Error(String),
}

#[derive(Default)]
pub struct Updates {
    pending: Option<Receiver<Event>>,
    status: Option<Event>,
}

fn check(sender: &Sender<Event>, ctx: &egui::Context) -> Result<(), String> {
    let powershell = PathBuf::from(env::var_os("SystemRoot").ok_or("SystemRoot")?)
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut helper = Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "[Console]::InputEncoding = [Text.UTF8Encoding]::new($false); & ([ScriptBlock]::Create([Console]::In.ReadToEnd()))"])
        .env("FERRITE_UPDATE_PID", std::process::id().to_string())
        .env("FERRITE_UPDATE_VERSION", env!("CARGO_PKG_VERSION"))
        .env("FERRITE_UPDATE_LANG", if crate::language::english() { "en" } else { "fr" })
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    helper
        .stdin
        .take()
        .unwrap()
        .write_all(include_bytes!("update.ps1"))
        .map_err(|e| e.to_string())?;
    for line in BufReader::new(helper.stdout.take().unwrap()).lines() {
        let line = line.map_err(|e| e.to_string())?;
        let event = if line == "CURRENT" {
            Event::Current
        } else if line == "READY" {
            Event::Ready
        } else if let Some(version) = line.strip_prefix("DOWNLOAD ") {
            Event::Download(version.into())
        } else if let Some(error) = line.strip_prefix("ERROR ") {
            Event::Error(error.into())
        } else {
            continue;
        };
        let finished = !matches!(event, Event::Download(_));
        if sender.send(event).is_err() {
            return Ok(());
        }
        ctx.request_repaint();
        if finished {
            return Ok(());
        }
    }
    Err(crate::language::text("Le service de mise à jour s’est arrêté.").into())
}

impl Updates {
    pub fn busy(&self) -> bool {
        self.pending.is_some() || matches!(self.status, Some(Event::Ready))
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, can_restart: bool) {
        let text = crate::language::text;
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(event) => {
                    if !matches!(event, Event::Download(_)) {
                        self.pending = None;
                    }
                    if matches!(event, Event::Ready) {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    self.status = Some(event);
                    ui.ctx().request_repaint();
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.status = Some(Event::Error(
                        text("Le service de mise à jour s’est arrêté.").into(),
                    ));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if ui
            .add_enabled(
                can_restart && !self.busy(),
                egui::Button::new(text("Mettre à jour")),
            )
            .on_hover_text(text(
                "Installer la dernière version et redémarrer Ferrite11.",
            ))
            .clicked()
        {
            let (tx, rx) = mpsc::channel();
            self.pending = Some(rx);
            self.status = None;
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                if let Err(e) = check(&tx, &ctx) {
                    let _ = tx.send(Event::Error(e));
                    ctx.request_repaint();
                }
            });
        }
        match &self.status {
            Some(Event::Current) => {
                ui.label(format!(
                    "{} · v{}",
                    text("Ferrite11 est à jour"),
                    env!("CARGO_PKG_VERSION")
                ));
            }
            Some(Event::Download(v)) => {
                ui.spinner();
                ui.label(format!("{} v{v}…", text("Téléchargement de la version")));
            }
            Some(Event::Ready) => {
                ui.label(text("Redémarrage de Ferrite11…"));
            }
            Some(Event::Error(detail)) => {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("{} {detail}", text("Mise à jour impossible.")),
                );
            }
            None if self.busy() => {
                ui.spinner();
                ui.label(text("Recherche d’une mise à jour…"));
            }
            None => {}
        }
    }
}
