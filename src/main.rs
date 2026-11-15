#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("Ferrite11 fonctionne exclusivement sous Windows 11.");
}

#[cfg(windows)]
mod apps;
#[cfg(windows)]
mod associations;
#[cfg(windows)]
mod display_modes;
#[cfg(windows)]
mod user_path;
#[cfg(windows)]
mod startup;
#[cfg(windows)]
mod app {
    use eframe::egui::{self, Color32, RichText};
    use serde::{Deserialize, Serialize};
    use std::{collections::{BTreeMap, BTreeSet}, fs, io, path::PathBuf, process::Command, sync::mpsc, time::Duration};
    use winreg::{HKCU, HKLM};
    use winreg::enums::{KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD, REG_SZ};

    const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    const ADVANCED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
    const GAMEDVR: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
    const GAMECONFIG: &str = r"System\GameConfigStore";
    const WINDOWMETRICS: &str = r"Control Panel\Desktop\WindowMetrics";

    #[derive(Clone, serde::Deserialize)]
    struct Machine {
        cpu: String,
        gpu: String,
        ram_gb: f64,
        os: String,
        build: String,
    }
    fn get_machine() -> Result<Machine, String> {
        // Détection locale en lecture seule, sans réseau ni envoi de données.
        let script = r#"
$ErrorActionPreference = 'Stop'
$p = Get-CimInstance Win32_Processor | Select-Object -First 1
$g = @(Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name }) -join ', '
$m = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
$w = Get-CimInstance Win32_OperatingSystem
[pscustomobject]@{ cpu=$p.Name; gpu=$g; ram_gb=[math]::Round($m/1GB,1); os=$w.Caption; build=$w.BuildNumber } | ConvertTo-Json -Compress
"#;
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output().map_err(|e| format!("PowerShell : {e}"))?;
        if !output.status.success() {
            return Err("Impossible d'interroger les caractéristiques matérielles via Windows.".into());
        }
        serde_json::from_slice::<Machine>(&output.stdout)
            .map_err(|e| format!("Réponse matérielle illisible : {e}"))
    }
    fn windows11_supported() -> bool {
        HKLM.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
            .ok()
            .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber").ok())
            .and_then(|n| n.parse::<u32>().ok())
            .is_some_and(|n| n >= 22000)
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Page { Accueil, Windows, Jeux, Demarrage, Systeme, Materiel, Outils, Journal, Privacy, Updates, Avance, Nvidia, Latence }
    impl Page {
        const ALL: [Page; 13] = [Page::Accueil, Page::Windows, Page::Updates, Page::Privacy, Page::Jeux, Page::Nvidia, Page::Latence, Page::Demarrage, Page::Systeme, Page::Materiel, Page::Avance, Page::Outils, Page::Journal];
        fn label(self) -> &'static str {
            match self {
                Self::Accueil => "Tableau de bord", Self::Windows => "Interface Windows",
                Self::Jeux => "Gaming & GPU", Self::Demarrage => "Applications & démarrage", Self::Systeme => "Système & stockage",
                Self::Materiel => "CPU, RAM & BIOS", Self::Outils => "Applications de remplacement",
                Self::Journal => "Journal & sauvegardes",
                Self::Updates => "Windows Update", Self::Privacy => "Confidentialité & comptes",
                Self::Avance => "Services & outils avancés", Self::Nvidia => "Pilotes NVIDIA",
                Self::Latence => "Latence & alimentation",
            }
        }
    }

    #[derive(Clone)]
    enum Desired { Dword(u32), Text(&'static str) }
    #[derive(Clone)]
    struct RegChange { path: &'static str, name: &'static str, desired: Desired }
    #[derive(Clone)]
    enum Operation { Registry(Vec<RegChange>), BalancedPower, CleanPath, OrphanStartup }
    #[derive(Clone)]
    struct Tweak {
        id: &'static str,
        page: Page,
        title: &'static str,
        description: &'static str,
        effect: &'static str,
        risk: &'static str,
        restart: &'static str,
        operation: Operation,
    }
    fn reg(path: &'static str, name: &'static str, value: u32) -> RegChange {
        RegChange { path, name, desired: Desired::Dword(value) }
    }
    fn tweaks() -> Vec<Tweak> {
        vec![
            Tweak {
                id: "transparency", page: Page::Windows,
                title: "Désactiver la transparence", risk: "Faible", restart: "Souvent immédiat",
                description: "Retire les effets transparents des surfaces Windows. N'arrête aucun service ni processus.",
                effect: "Gain graphique généralement faible ; interface plus sobre.",
                operation: Operation::Registry(vec![reg(PERSONALIZE, "EnableTransparency", 0)]),
            },
            Tweak {
                id: "animations", page: Page::Windows,
                title: "Réduire l'animation des fenêtres", risk: "Faible", restart: "Reconnexion parfois nécessaire",
                description: "Désactive l'animation d'ouverture/réduction des fenêtres via le paramètre utilisateur MinAnimate.",
                effect: "Sensation de réactivité ; pas de gain FPS démontré.",
                operation: Operation::Registry(vec![RegChange { path: WINDOWMETRICS, name: "MinAnimate", desired: Desired::Text("0") }]),
            },
            Tweak {
                id: "widgets", page: Page::Windows,
                title: "Masquer le bouton Widgets", risk: "Faible", restart: "Reconnexion parfois nécessaire",
                description: "Masque Widgets dans la barre des tâches, sans désinstaller le composant Windows.",
                effect: "Allège l'interface ; ne promet pas de libérer la RAM des Widgets.",
                operation: Operation::Registry(vec![reg(ADVANCED, "TaskbarDa", 0)]),
            },
            Tweak {
                id: "capture", page: Page::Jeux,
                title: "Désactiver les captures Game DVR", risk: "Modéré", restart: "Reconnexion ou redémarrage conseillé",
                description: "Désactive les options utilisateur de capture Xbox Game Bar (enregistrement et captures via Game DVR). Ne touche pas aux optimisations plein écran.",
                effect: "Réduit les captures non désirées ; tu perds certaines fonctions d'enregistrement Xbox jusqu'à restauration.",
                operation: Operation::Registry(vec![reg(GAMEDVR, "AppCaptureEnabled", 0), reg(GAMECONFIG, "GameDVR_Enabled", 0)]),
            },
            Tweak {
                id: "orphan-startup", page: Page::Demarrage,
                title: "Retirer les démarrages dont le programme a disparu", risk: "Faible", restart: "À la prochaine connexion",
                description: "Vérifie les entrées de démarrage de ton compte qui pointent directement vers un programme installé sur le disque système.",
                effect: "Windows cesse de chercher ces exécutables absents à chaque connexion. La liste est affichée avant validation.",
                operation: Operation::OrphanStartup,
            },
            Tweak {
                id: "path-duplicates", page: Page::Systeme,
                title: "Retirer les doublons du PATH utilisateur", risk: "Faible", restart: "À la prochaine connexion Windows",
                description: "Garde la première occurrence de chaque chemin absolu strictement identique dans la liste des dossiers où Windows cherche les commandes.",
                effect: "Élimine les recherches répétées dans ces dossiers. Les autres entrées gardent leur ordre.",
                operation: Operation::CleanPath,
            },
            Tweak {
                id: "balanced", page: Page::Systeme,
                title: "Revenir au plan d'alimentation Équilibré", risk: "Faible", restart: "Immédiat",
                description: "Active le plan Équilibré intégré de Windows. Ne force pas de fréquence CPU ni de tension.",
                effect: "Évite de maintenir inutilement certaines politiques haute performance ; FPS à comparer.",
                operation: Operation::BalancedPower,
            },
            Tweak {
                id: "file-extensions", page: Page::Windows,
                title: "Afficher les extensions de fichiers", risk: "Faible", restart: "À la prochaine ouverture de session",
                description: "Affiche les extensions connues, par exemple .pdf, .zip ou .exe, dans l’Explorateur.",
                effect: "Le type réel d’un fichier reste visible même si son nom contient plusieurs points.",
                operation: Operation::Registry(vec![reg(ADVANCED, "HideFileExt", 0)]),
            },
            Tweak {
                id: "gamebar-controller", page: Page::Jeux,
                title: "Empêcher la manette d’ouvrir la Game Bar", risk: "Faible", restart: "À la prochaine utilisation",
                description: "Désactive le lancement de la surcouche par le bouton Xbox de la manette. Win + G reste disponible.",
                effect: "Évite une ouverture accidentelle en jeu. Le Mode Jeu et les captures conservent leurs réglages.",
                operation: Operation::Registry(vec![reg(r"Software\Microsoft\GameBar", "UseNexusForGameBarEnabled", 0)]),
            },
            Tweak {
                id: "app-launch-tracking", page: Page::Privacy,
                title: "Couper le suivi des lancements d’applications", risk: "Faible", restart: "À la prochaine connexion si nécessaire",
                description: "Désactive le suivi des applications que tu ouvres pour personnaliser Démarrer et la recherche.",
                effect: "Les recommandations fondées sur tes habitudes de lancement deviennent moins personnalisées.",
                operation: Operation::Registry(vec![reg(ADVANCED, "Start_TrackProgs", 0)]),
            },
            Tweak {
                id: "website-language-list", page: Page::Privacy,
                title: "Limiter l’accès des sites à la liste de langues Windows", risk: "Faible", restart: "Après réouverture des applications concernées",
                description: "Désactive l’utilisation de ta liste de langues Windows pour proposer du contenu localisé aux sites.",
                effect: "Certains sites peuvent demander de choisir une langue. Le navigateur possède aussi ses propres préférences.",
                operation: Operation::Registry(vec![reg(r"Control Panel\International\User Profile", "HttpAcceptLanguageOptOut", 1)]),
            },
            Tweak {
                id: "game-mode", page: Page::Jeux,
                title: "Activer le Mode Jeu de Windows", risk: "Faible", restart: "Au prochain lancement du jeu",
                description: "Active le réglage Mode Jeu pour cette session utilisateur.",
                effect: "Windows peut donner la priorité au jeu. L’effet sur la fluidité dépend du jeu et de la charge en arrière-plan.",
                operation: Operation::Registry(vec![reg(r"Software\Microsoft\GameBar", "AutoGameModeEnabled", 1)]),
            },
        ]
    }

    #[derive(Clone, Serialize, Deserialize)]
    enum OldValue { Missing, Dword(u32), Text(String) }
    #[derive(Clone, Serialize, Deserialize)]
    struct RegBackup { path: String, name: String, old: OldValue }
    #[derive(Clone, Serialize, Deserialize)]
    enum Snapshot { Registry(Vec<RegBackup>), PowerPlan(String), UserPath(crate::user_path::SavedPath), Startup(crate::startup::Saved) }
    #[derive(Default, Serialize, Deserialize)]
    struct Store {
        backups: BTreeMap<String, Snapshot>,
        manual_done: BTreeSet<String>,
        log: Vec<String>,
    }
    fn storage_path() -> PathBuf {
        let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        root.join("Ferrite11").join("state.json")
    }
    fn load_store() -> Result<Store, String> {
        let path = storage_path();
        if !path.exists() { return Ok(Store::default()); }
        let data = fs::read(&path).map_err(|e| format!("Lecture impossible : {e}"))?;
        serde_json::from_slice(&data).map_err(|e| format!("Sauvegarde illisible : {e}"))
    }
    fn save_store(store: &Store) -> Result<(), String> {
        let path = storage_path();
        if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        let bytes = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
        // Écriture avant toute modification système : le fichier conserve les valeurs initiales.
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, bytes).map_err(|e| format!("Échec d'écriture {} : {e}", temporary.display()))?;
        fs::rename(&temporary, &path).map_err(|e| format!("Échec de remplacement du fichier de sauvegarde {} : {e}", path.display()))
    }
    fn read_value(c: &RegChange) -> Result<OldValue, String> {
        let k = match HKCU.open_subkey_with_flags(c.path, KEY_QUERY_VALUE) {
            Ok(k) => k,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(OldValue::Missing),
            Err(e) => return Err(format!("{} : {e}", c.path)),
        };
        let raw = match k.get_raw_value(c.name) {
            Ok(value) => value,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(OldValue::Missing),
            Err(e) => return Err(format!("{}\\{} : {e}", c.path, c.name)),
        };
        // Accept only values that the snapshot can reproduce byte-for-byte.
        decode_setting(&c.desired, &raw).map_err(|reason| format!("{}\\{} : {reason}", c.path, c.name))
    }
    fn decode_setting(desired: &Desired, raw: &winreg::RegValue<'_>) -> Result<OldValue, &'static str> {
        match (desired, &raw.vtype) {
            (Desired::Dword(_), &REG_DWORD) if raw.bytes.len() == 4 =>
                Ok(OldValue::Dword(u32::from_le_bytes(raw.bytes[..].try_into().unwrap()))),
            (Desired::Text(_), &REG_SZ) if raw.bytes.len() % 2 == 0 => {
                let mut words: Vec<u16> = raw.bytes.chunks_exact(2)
                    .map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
                if words.pop() != Some(0) || words.contains(&0) {
                    return Err("Chaîne de registre inhabituelle, réglage préservé.");
                }
                String::from_utf16(&words).map(OldValue::Text)
                    .map_err(|_| "Texte de registre illisible, réglage préservé.")
            }
            _ => Err("Format de registre inattendu, réglage préservé."),
        }
    }
    fn matches(c: &RegChange, old: &OldValue) -> bool {
        match (&c.desired, old) {
            (Desired::Dword(a), OldValue::Dword(b)) => a == b,
            (Desired::Text(a), OldValue::Text(b)) => *a == b,
            _ => false,
        }
    }
    fn write_desired(c: &RegChange) -> Result<(), String> {
        let (k, _) = HKCU.create_subkey_with_flags(c.path, KEY_SET_VALUE).map_err(|e| e.to_string())?;
        match c.desired {
            Desired::Dword(x) => k.set_value(c.name, &x).map_err(|e| e.to_string()),
            Desired::Text(x) => k.set_value(c.name, &x).map_err(|e| e.to_string()),
        }
    }
    fn restore_entry(b: &RegBackup) -> Result<(), String> {
        let k = match HKCU.open_subkey_with_flags(&b.path, KEY_SET_VALUE) {
            Ok(k) => k,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if matches!(b.old, OldValue::Missing) { return Ok(()); }
                HKCU.create_subkey_with_flags(&b.path, KEY_SET_VALUE).map_err(|e| e.to_string())?.0
            }
            Err(e) => return Err(e.to_string()),
        };
        match &b.old {
            OldValue::Dword(x) => k.set_value(&b.name, x).map_err(|e| e.to_string()),
            OldValue::Text(s) => k.set_value(&b.name, &s.as_str()).map_err(|e| e.to_string()),
            OldValue::Missing => match k.delete_value(&b.name) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.to_string()),
            },
        }
    }
    fn parse_guid(output: &str) -> Option<String> {
        output.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
            .find(|s| s.len() == 36 && [8, 13, 18, 23].iter().all(|i| s.as_bytes()[*i] == b'-')
                  && s.chars().enumerate().all(|(i, c)| [8, 13, 18, 23].contains(&i) || c.is_ascii_hexdigit()))
            .map(str::to_owned)
    }
    fn power_current() -> Result<String, String> {
        let out = Command::new("powercfg.exe").arg("/getactivescheme").output().map_err(|e| e.to_string())?;
        if !out.status.success() { return Err("powercfg /getactivescheme a échoué".into()); }
        parse_guid(&String::from_utf8_lossy(&out.stdout))
            .ok_or_else(|| "Impossible de lire le GUID du plan d'alimentation".into())
    }
    fn power_set(arg: &str) -> Result<(), String> {
        let out = Command::new("powercfg.exe").args(["/setactive", arg]).output().map_err(|e| e.to_string())?;
        if out.status.success() { Ok(()) }
        else { Err(format!("powercfg a échoué (code {:?})", out.status.code())) }
    }
    const BALANCED_GUID: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
    fn snapshot(t: &Tweak) -> Result<Snapshot, String> {
        match &t.operation {
            Operation::Registry(changes) => {
                let mut backup = Vec::new();
                for c in changes {
                    backup.push(RegBackup { path: c.path.to_owned(), name: c.name.to_owned(), old: read_value(c)? });
                }
                Ok(Snapshot::Registry(backup))
            }
            Operation::OrphanStartup => crate::startup::snapshot().map(Snapshot::Startup),
            Operation::CleanPath => crate::user_path::snapshot()?.map(Snapshot::UserPath).ok_or_else(|| "Aucun PATH utilisateur.".to_owned()),
            Operation::BalancedPower => Ok(Snapshot::PowerPlan(power_current()?)),
        }
    }
    fn tweak_satisfied(t: &Tweak) -> Result<bool, String> {
        match &t.operation {
            Operation::Registry(changes) => {
                for c in changes { if !matches(c, &read_value(c)?) { return Ok(false); } }
                Ok(true)
            }
            Operation::OrphanStartup => crate::startup::snapshot().map(|entries| entries.is_empty()),
            Operation::CleanPath => crate::user_path::satisfied(),
            Operation::BalancedPower => Ok(power_current()?.eq_ignore_ascii_case(BALANCED_GUID)),
        }
    }
    fn scan_statuses() -> BTreeMap<String, Result<bool, String>> {
        tweaks().into_iter().map(|t| (t.id.to_owned(), tweak_satisfied(&t))).collect()
    }
    fn do_change(t: &Tweak, backup: &Snapshot) -> Result<(), String> {
        match &t.operation {
            Operation::Registry(changes) => {
                let Snapshot::Registry(entries) = backup else {
                    return Err("Sauvegarde de registre incompatible.".into());
                };
                for c in changes {
                    let previous = entries.iter().find(|b| b.path == c.path && b.name == c.name)
                        .ok_or_else(|| format!("Valeur {} absente de la sauvegarde.", c.name))?;
                    let now = read_value(c)?;
                    if !same_old(&now, &previous.old) {
                        return Err(format!("{} a changé depuis la sauvegarde. Application interrompue.", c.name));
                    }
                    if !matches(c, &now) { write_desired(c)?; }
                    if !matches(c, &read_value(c)?) {
                        return Err(format!("La valeur relue de {} ne correspond pas au réglage demandé.", c.name));
                    }
                }
                Ok(())
            }
            Operation::OrphanStartup => match backup {
                Snapshot::Startup(previous) => previous.apply(),
                _ => Err("Sauvegarde de démarrage incompatible.".into()),
            },
            Operation::CleanPath => match backup {
                Snapshot::UserPath(previous) => previous.update(false),
                _ => Err("Sauvegarde incompatible.".into()),
            },
            Operation::BalancedPower => power_set(BALANCED_GUID),
        }
    }
    fn undo_snapshot(s: &Snapshot) -> Result<(), String> {
        match s {
            Snapshot::Registry(entries) => {
                let mut errors = Vec::new();
                for b in entries { if let Err(e) = restore_entry(b) { errors.push(format!("{}: {e}", b.name)); } }
                if errors.is_empty() { Ok(()) } else { Err(errors.join(" ; ")) }
            }
            Snapshot::Startup(previous) => previous.restore(),
            Snapshot::UserPath(previous) => previous.update(true),
            Snapshot::PowerPlan(guid) => power_set(guid),
        }
    }
    fn can_undo(t: &Tweak, s: &Snapshot) -> Result<(), String> {
        match (&t.operation, s) {
            (Operation::Registry(changes), Snapshot::Registry(entries)) => {
                if entries.len() != changes.len() {
                    return Err("Sauvegarde incohérente avec les réglages reconnus par Ferrite11.".into());
                }
                for c in changes {
                    let current = read_value(c)?;
                    let original = entries.iter().find(|b| b.path == c.path && b.name == c.name)
                        .ok_or_else(|| format!("Valeur {} absente de la sauvegarde", c.name))?;
                    if !matches!((&c.desired, &original.old),
                        (_, OldValue::Missing) | (Desired::Dword(_), OldValue::Dword(_)) | (Desired::Text(_), OldValue::Text(_))) {
                        return Err(format!("Le type sauvegardé pour {} est incohérent.", c.name));
                    }
                    // Tolère également un état jamais modifié (crash avant écriture) ou
                    // partiellement modifié (crash pendant une opération multi-valeurs).
                    if !matches(c, &current) && !same_old(&current, &original.old) {
                        return Err(format!("Le réglage {} a changé depuis l'application : restauration bloquée pour éviter d'écraser ton choix.", c.name));
                    }
                }
                Ok(())
            }
            (Operation::OrphanStartup, Snapshot::Startup(previous)) => previous.check_restore(),
            (Operation::CleanPath, Snapshot::UserPath(previous)) => previous.check_restore(),
            (Operation::BalancedPower, Snapshot::PowerPlan(original)) => {
                if parse_guid(original).as_deref() != Some(original.as_str()) {
                    return Err("Identifiant du plan sauvegardé invalide.".into());
                }
                let current = power_current()?;
                if current.eq_ignore_ascii_case(BALANCED_GUID) || current.eq_ignore_ascii_case(original) { Ok(()) }
                else { Err("Le plan d'alimentation a été modifié depuis l'application.".into()) }
            }
            _ => Err("Format de sauvegarde incompatible".into()),
        }
    }
    #[cfg(test)]
    mod setting_tests {
        use super::*;
        use winreg::types::ToRegValue;

        #[test]
        fn analyze_and_preview_in_simulation_from_the_ui() {
            let mut app = FerriteApp::new();
            app.store = Store::default();
            app.storage_error = None;
            app.supported = true;
            // The general analysis must keep a separate individual choice.
            app.selected.insert("game-mode".to_owned());
            let ctx = egui::Context::default();
            let mut draw = |events| {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 1400.))),
                    events, ..Default::default()
                };
                let frame = ctx.run_ui(input, |ui| { app.home(ui); app.bottom_actions(ui); });
                frame.shapes.into_iter().filter_map(|s| match s.shape {
                    egui::Shape::Text(text) => Some((text.galley.text().to_owned(), text.pos + egui::vec2(3., 4.))),
                    _ => None,
                }).collect::<BTreeMap<_, _>>()
            };
            let mut labels = draw(Vec::new());
            for button in ["Mode simulation", "Analyser et sélectionner les réglages utiles", "Valider"] {
                let pos = labels[button];
                for pressed in [true, false] {
                    labels = draw(vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {
                        pos, pressed, button: egui::PointerButton::Primary, modifiers: Default::default(),
                    }]);
                }
            }
            assert!(labels.contains_key("PRÉVISUALISATION"));
            assert!(labels.contains_key("Activer le Mode Jeu de Windows"));
            assert!(labels.contains_key("Fermer la simulation"));
            assert!(!labels.contains_key("Confirmer et appliquer"));
            assert!(app.simulation && app.confirmation && app.store.backups.is_empty() && app.store.log.is_empty());
            app.run_apply();
            assert!(app.store.backups.is_empty() && app.store.log.is_empty());
        }

        #[test]
        fn settings_have_unique_ids_and_do_not_share_registry_values() {
            let mut ids = BTreeSet::new();
            let mut values = BTreeSet::new();
            for t in tweaks() {
                assert!(ids.insert(t.id));
                if let Operation::Registry(changes) = t.operation {
                    assert!(!changes.is_empty());
                    for c in changes { assert!(values.insert((c.path.to_lowercase(), c.name.to_lowercase()))); }
                }
            }
        }

        #[test]
        fn standard_values_roundtrip_and_unexpected_registry_types_are_preserved() {
            let text = Desired::Text("0");
            assert!(matches!(decode_setting(&text, &"001".to_reg_value()), Ok(OldValue::Text(s)) if s == "001"));
            assert!(matches!(decode_setting(&Desired::Dword(0), &42u32.to_reg_value()), Ok(OldValue::Dword(42))));
            let mut expanded = "001".to_reg_value();
            expanded.vtype = winreg::enums::REG_EXPAND_SZ;
            assert!(decode_setting(&text, &expanded).is_err());
            assert!(decode_setting(&text, &42u32.to_reg_value()).is_err());
        }
    }
    fn same_old(a: &OldValue, b: &OldValue) -> bool {
        match (a,b) {
            (OldValue::Missing, OldValue::Missing) => true,
            (OldValue::Dword(x), OldValue::Dword(y)) => x == y,
            (OldValue::Text(x), OldValue::Text(y)) => x == y,
            _ => false,
        }
    }
    fn open_target(target: &str) -> Result<(), String> {
        Command::new("explorer.exe").arg(target).spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    struct OptionGuide {
        id: &'static str, page: Page, title: &'static str, steps: &'static str,
        undo: &'static str, note: &'static str, target: &'static str, source: &'static str,
    }
    mod windows_options { include!("windows_options.rs"); }
    mod advanced_options { include!("advanced_options.rs"); }
    mod gpu_latency { include!("gpu_latency.rs"); }
    fn options() -> impl Iterator<Item = &'static OptionGuide> {
        windows_options::GUIDES.iter().chain(advanced_options::GUIDES).chain(gpu_latency::GUIDES)
    }

    struct Guide {
        id: &'static str, page: Page, title: &'static str, description: &'static str,
        steps: &'static str, undo: &'static str,
        benefit: &'static str, risk: &'static str, button: &'static str, target: &'static str,
    }
    const GUIDES: &[Guide] = &[
        Guide {
            id: "startup", page: Page::Demarrage, title: "Nettoyer les applications au démarrage",
            description: "Garde les utilitaires nécessaires à ton matériel, notamment ceux qui pilotent la pompe ou les ventilateurs. Pour l’éclairage RGB, vérifie si ton profil dépend de l’application. Identifie ensuite les lanceurs que tu préfères ouvrir toi-même.",
            steps: "Dans Applications au démarrage, repère un lanceur que tu préfères ouvrir à la demande. Note son état, désactive-le et vérifie ta prochaine ouverture de session.",
            undo: "Réactive ce lanceur sur la même page. Tu peux aussi le lancer depuis Démarrer à tout moment.",
            benefit: "RAM et CPU au repos : potentiellement élevé", risk: "Faible",
            button: "Ouvrir Démarrage", target: "ms-settings:startupapps",
        },
        Guide {
            id: "installed", page: Page::Demarrage, title: "Désinstaller les apps inutilisées",
            description: "Retire les applications préinstallées ou abandonnées depuis les Paramètres. Évite les scripts de debloat qui suppriment des composants système.",
            steps: "Identifie une application que tu reconnais. Vérifie que tu peux récupérer son installateur et exporte les données locales à conserver. Ouvre son menu et choisis Désinstaller.",
            undo: "Réinstalle l’application depuis sa source d’origine et importe ta sauvegarde. Une réinstallation seule ne récupère pas forcément tes données.",
            benefit: "Stockage et éventuelles tâches résidentes", risk: "Faible à modéré",
            button: "Applications", target: "ms-settings:appsfeatures",
        },
        Guide {
            id: "visual", page: Page::Windows, title: "Désactiver d'autres effets visuels",
            description: "Réduis les animations et la transparence dans les options d'accessibilité. Garde le lissage des caractères.",
            steps: "Dans Accessibilité > Effets visuels, note tes préférences. Bascule un effet à la fois puis ouvre et réduis une fenêtre pour comparer.",
            undo: "Rétablis les interrupteurs que tu as modifiés.",
            benefit: "Réactivité visuelle ; FPS rarement affectés", risk: "Faible",
            button: "Effets visuels", target: "ms-settings:easeofaccess-visualeffects",
        },
        Guide {
            id: "taskbar", page: Page::Windows, title: "Barre des tâches minimaliste",
            description: "Choisis les icônes visibles, dont Widgets, Conversation ou Teams selon ta version. Le masquage change l’affichage. Pour contrôler le lancement de Teams à la connexion, utilise Applications et démarrage.",
            steps: "Dans Personnalisation > Barre des tâches, note les éléments affichés et masque ceux que tu utilises peu. Si Teams est épinglé, clique dessus avec le bouton droit puis choisis de le détacher.",
            undo: "Réaffiche les éléments sur la même page. Pour épingler de nouveau Teams, retrouve l’application dans Démarrer puis utilise son menu contextuel.",
            benefit: "Interface plus sobre", risk: "Faible",
            button: "Barre des tâches", target: "ms-settings:taskbar",
        },
        Guide {
            id: "search", page: Page::Systeme, title: "Réduire les emplacements indexés",
            description: "Conserve Windows Search si tu t'en sers. Réduis seulement les dossiers indexés si l'indexeur est réellement actif.",
            steps: "Dans les paramètres de recherche Windows, relève les exclusions existantes. Exclus un dossier volumineux dans lequel tu ne recherches jamais de fichiers, puis teste une recherche habituelle.",
            undo: "Retire ce dossier de la liste des exclusions. Windows aura besoin de temps pour l’indexer à nouveau.",
            benefit: "Moins d'activité disque dans certains cas", risk: "Faible",
            button: "Recherche", target: "ms-settings:cortana-windowssearch",
        },
        Guide {
            id: "optional", page: Page::Systeme, title: "Fonctionnalités facultatives",
            description: "Retire un composant facultatif seulement si tu connais son usage et sais comment le réinstaller.",
            steps: "Dans Fonctionnalités facultatives, ouvre la liste des éléments installés. Vérifie l’usage d’un élément que tu reconnais, par exemple une visionneuse, avant de le retirer.",
            undo: "Utilise Afficher les fonctionnalités pour retrouver et réinstaller cet élément. Windows peut demander un téléchargement ou un redémarrage.",
            benefit: "Variable", risk: "Modéré",
            button: "Fonctionnalités", target: "ms-settings:optionalfeatures",
        },
        Guide {
            id: "storage", page: Page::Systeme, title: "Nettoyage du stockage",
            description: "Utilise l'assistant de stockage et vérifie les fichiers avant suppression. Ne supprime pas manuellement des dossiers système.",
            steps: "Ouvre Stockage puis Fichiers temporaires. Lis la description de chaque catégorie et laisse tes dossiers personnels décochés. Confirme seulement les éléments dont tu acceptes la suppression.",
            undo: "Les fichiers supprimés ici peuvent être irrécupérables. Pour les données personnelles, prévois une copie avant le nettoyage.",
            benefit: "Espace disque, parfois temps de chargement", risk: "Faible à modéré",
            button: "Stockage", target: "ms-settings:storagesense",
        },
        Guide {
            id: "gamedvr-ui", page: Page::Jeux, title: "Vérifier les captures Windows",
            description: "Si tu utilises Xbox Game Bar, garde l'enregistrement activé. Sinon, examine les captures en arrière-plan.",
            steps: "Dans Jeux > Captures, relève l’état de l’enregistrement en arrière-plan. Désactive-le si tu n’utilises jamais les clips des dernières secondes de jeu.",
            undo: "Réactive cet enregistrement puis vérifie que tu peux enregistrer un nouveau clip.",
            benefit: "Réduction d'activité en jeu si active", risk: "Faible",
            button: "Captures", target: "ms-settings:gaming-gamedvr",
        },
        Guide {
            id: "gamemode", page: Page::Jeux, title: "Conserver le Mode Jeu Windows",
            description: "Vérifie que le Mode Jeu est activé. Compare uniquement si tu constates un problème dans un jeu précis.",
            steps: "Relève l’état du Mode Jeu. Pour le conserver actif, laisse-le activé ou active-le s’il est désactivé. Tu peux ensuite comparer le comportement dans une même scène de jeu.",
            undo: "Remets l’interrupteur dans sa position initiale.",
            benefit: "Gestion des ressources en jeu", risk: "Faible",
            button: "Mode Jeu", target: "ms-settings:gaming-gamemode",
        },
        Guide {
            id: "graphics", page: Page::Jeux, title: "Optimisations graphiques Windows",
            description: "Teste HAGS et les optimisations de jeux fenêtrés séparément, avec redémarrage et comparaison des 1 % low.",
            steps: "Dans Affichage > Graphiques, ouvre les paramètres par défaut disponibles. Relève l’état de la planification GPU ou des optimisations en mode fenêtré. Change une seule option, puis redémarre si Windows le demande.",
            undo: "Rétablis la valeur notée et redémarre si demandé.",
            benefit: "Dépend du jeu et du pilote", risk: "Faible à modéré",
            button: "Graphiques", target: "ms-settings:display-advancedgraphics",
        },
        Guide {
            id: "refresh", page: Page::Jeux, title: "Vérifier la fréquence de l'écran",
            description: "Sélectionne la fréquence de rafraîchissement réellement prise en charge par ton moniteur.",
            steps: "Dans Affichage avancé, sélectionne le bon écran. Note la fréquence et la résolution actuelles. Pour privilégier la fluidité, choisis la fréquence la plus élevée disponible à cette résolution, par exemple 144, 240 ou 360 Hz si l’écran, le câble et le GPU le permettent. Garde-la si l’image est stable.",
            undo: "Sélectionne l’ancienne fréquence. Si l’image disparaît, attends le retour automatique quand Windows affiche une demande de confirmation.",
            benefit: "Fluidité de l'affichage", risk: "Faible",
            button: "Écran avancé", target: "ms-settings:display-advanced",
        },
        Guide {
            id: "nvidia", page: Page::Jeux, title: "Cartes NVIDIA : pilote et Reflex",
            description: "Si tu as une GeForce : pilote adapté, Reflex dans les jeux compatibles et overlays inutiles désactivés. Ne force pas la performance maximale globalement.",
            steps: "Dans un jeu compatible, ouvre les paramètres graphiques et relève la valeur NVIDIA Reflex. Essaie Activé puis compare les commandes sur la même scène. Pour un pilote, identifie d’abord ton modèle exact.",
            undo: "Rétablis la valeur Reflex dans le jeu. En cas de changement de pilote, utilise la version précédente compatible fournie par le fabricant.",
            benefit: "Latence et stabilité selon les jeux", risk: "Faible",
            button: "Support NVIDIA", target: "https://www.nvidia.com/fr-fr/geforce/drivers/",
        },
        Guide {
            id: "radeon", page: Page::Jeux, title: "Cartes AMD Radeon : pilotes et Anti-Lag",
            description: "Si tu as une Radeon : utilise le pilote officiel compatible ; teste Anti-Lag et les options de latence jeu par jeu, avec mesures avant/après.",
            steps: "Dans AMD Software, ouvre le profil du jeu choisi. Relève l’état de Radeon Anti-Lag, change-le si l’option est proposée et compare la même scène.",
            undo: "Rétablis l’état initial d’Anti-Lag dans le profil de ce jeu.",
            benefit: "Latence et stabilité selon les jeux", risk: "Faible",
            button: "Support AMD", target: "https://www.amd.com/fr/support/download/drivers.html",
        },
        Guide {
            id: "intelgpu", page: Page::Jeux, title: "Graphiques Intel : pilotes et réglages",
            description: "Si tu as une carte Intel Arc ou un iGPU Intel : installe un pilote adapté et mesure les performances avant de modifier les paramètres.",
            steps: "Dans le Gestionnaire de périphériques, relève le modèle et la version du pilote sous Cartes graphiques. Compare avec la version proposée par le fabricant du PC pour ton modèle.",
            undo: "Dans les propriétés du périphérique, utilise Restaurer le pilote si disponible. Sinon, réinstalle le pilote compatible conservé avant le changement.",
            benefit: "Compatibilité et stabilité", risk: "Faible",
            button: "Support Intel", target: "https://www.intel.com/content/www/fr/fr/download-center/home.html",
        },
        Guide {
            id: "rebar", page: Page::Materiel, title: "Resizable BAR / Smart Access Memory",
            description: "Si ta plateforme et ton GPU le prennent en charge, vérifie la disponibilité de Resizable BAR. Un changement UEFI exige de connaître la procédure adaptée à la carte mère.",
            steps: "Consulte les informations système de ton pilote graphique pour relever l’état de Resizable BAR. Vérifie ensuite la compatibilité exacte de ta carte mère, de son firmware et du GPU dans leur documentation.",
            undo: "La vérification est en lecture seule. Si tu interviens ensuite dans l’UEFI, conserve une photo des valeurs d’origine et suis la procédure de restauration du fabricant.",
            benefit: "Variable selon les jeux et le GPU", risk: "Modéré",
            button: "Documentation générale", target: "https://learn.microsoft.com/fr-fr/windows-hardware/drivers/display/resizable-bar-support",
        },
        Guide {
            id: "ram", page: Page::Materiel, title: "Mémoire DDR4 / DDR5 : fréquence et canaux",
            description: "Vérifie les vitesses configurées, la répartition des barrettes et la stabilité. Les profils XMP, EXPO ou DOCP dépendent du processeur, du kit et de la carte mère.",
            steps: "Dans le Gestionnaire des tâches, ouvre Performances > Mémoire et relève la vitesse et les emplacements utilisés. Compare-les au manuel de ta carte mère et à la référence du kit.",
            undo: "La lecture des valeurs ne demande aucune restauration. Si tu testes un profil mémoire dans l’UEFI, conserve le profil d’origine et la procédure de récupération du fabricant.",
            benefit: "Potentiellement fort si la mémoire est mal configurée", risk: "Modéré",
            button: "Informations mémoire", target: "ms-settings:about",
        },
        Guide {
            id: "chipset-amd", page: Page::Materiel, title: "Pilotes chipset AMD",
            description: "Si ton PC a une plateforme AMD, installe seulement le pilote correspondant exactement à ton chipset et à ton Windows.",
            steps: "Identifie le modèle de carte mère ou de PC, consulte sa page de support et conserve l’installateur actuel avant de passer à une version compatible. Suis les étapes du fabricant.",
            undo: "Consulte la procédure de retour à la version précédente du fabricant pour ce modèle. Certains composants du paquet ne permettent pas un retour individuel.",
            benefit: "Stabilité et compatibilité", risk: "Faible à modéré",
            button: "Support AMD", target: "https://www.amd.com/fr/support",
        },
        Guide {
            id: "chipset-intel", page: Page::Materiel, title: "Pilotes chipset Intel",
            description: "Si ton PC a une plateforme Intel, vérifie les pilotes conseillés par le fabricant de la carte mère et Intel.",
            steps: "Identifie le modèle exact de PC ou de carte mère et vérifie les composants concernés dans les notes du paquet proposé. Conserve le paquet précédent avant toute installation.",
            undo: "Utilise la procédure du fabricant du PC pour réinstaller le paquet précédent si elle est disponible.",
            benefit: "Stabilité et compatibilité", risk: "Faible à modéré",
            button: "Support Intel", target: "https://www.intel.com/content/www/fr/fr/download-center/home.html",
        },
        Guide {
            id: "bios", page: Page::Materiel, title: "UEFI/BIOS : prudence",
            description: "Avant toute modification, enregistre un profil UEFI. Ferrite11 ne change ni tensions, ni paramètres d'overclocking, ni firmware.",
            steps: "Ouvre Informations système (msinfo32) et relève le fabricant, le modèle et la version du BIOS. Recherche le manuel de récupération correspondant avant d’envisager une intervention.",
            undo: "Relever la version ne change aucun réglage. Un retour de version du firmware dépend du modèle et doit être explicitement pris en charge par le fabricant.",
            benefit: "Dépend du diagnostic", risk: "Élevé",
            button: "Informations Windows", target: "ms-settings:about",
        },
        Guide {
            id: "measure", page: Page::Jeux, title: "Mesurer FPS et 1 % low",
            description: "Compare trois passages identiques avant/après avec CapFrameX. Ce sont les frametimes qui comptent, pas seulement la RAM libérée.",
            steps: "Choisis une scène et une durée fixes. Enregistre trois passages avant et trois après un seul changement, avec la même résolution et les mêmes réglages. Compare les temps d’image et les FPS.",
            undo: "Une mesure seule ne modifie aucun réglage. Si le résultat se dégrade, annule le changement testé dans sa rubrique.",
            benefit: "Permet de choisir les optimisations utiles", risk: "Aucun",
            button: "CapFrameX", target: "https://github.com/CXWorld/CapFrameX",
        },
        Guide {
            id: "notifications", page: Page::Windows, title: "Choisir les applications qui envoient des notifications",
            description: "Garde les alertes importantes et adapte les bannières à ton usage.",
            steps: "Dans Système > Notifications, repère une application dont les bannières te gênent. Note son état, puis modifie uniquement cette application.",
            undo: "Rétablis ses notifications sur la même page.",
            benefit: "Moins d’interruptions", risk: "Faible",
            button: "Ouvrir les notifications", target: "ms-settings:notifications",
        },
        Guide {
            id: "night", page: Page::Windows, title: "Programmer l’éclairage nocturne",
            description: "Choisis la teinte et les horaires qui te conviennent.",
            steps: "Dans Système > Affichage > Éclairage nocturne, note l’état, la teinte et les horaires actuels. Teste une teinte confortable, puis choisis l’horaire souhaité.",
            undo: "Désactive l’éclairage nocturne ou rétablis les valeurs notées si tu l’utilisais déjà.",
            benefit: "Confort visuel selon l’usage", risk: "Faible",
            button: "Régler l’éclairage nocturne", target: "ms-settings:nightlight",
        },
        Guide {
            id: "sleep", page: Page::Systeme, title: "Choisir les délais d’écran et de veille",
            description: "Un délai court économise l’énergie, un délai long garde le PC disponible pendant une lecture ou un travail prolongé.",
            steps: "Dans Système > Alimentation, ouvre les délais d’écran et de veille. Note les durées actuelles sur secteur et, sur portable, sur batterie. Choisis des durées adaptées à ton usage.",
            undo: "Rétablis les durées notées pour chaque mode d’alimentation.",
            benefit: "Consommation adaptée à tes habitudes", risk: "Faible",
            button: "Ouvrir l’alimentation", target: "ms-settings:powersleep",
        },
        Guide {
            id: "defaults", page: Page::Demarrage, title: "Choisir l’application qui ouvre un type de fichier",
            description: "Après un remplacement, choisis les extensions que tu veux ouvrir avec la nouvelle application dans Windows.",
            steps: "Dans Applications par défaut, recherche une extension comme .txt ou .png. Note l’application affichée, choisis son remplacement et ouvre un fichier de ce type pour vérifier.",
            undo: "Recherche la même extension et resélectionne l’application précédente. Répète pour chaque extension modifiée.",
            benefit: "Ouverture des fichiers dans ton outil préféré", risk: "Faible",
            button: "Ouvrir les applications par défaut", target: "ms-settings:defaultapps",
        },
        Guide {
            id: "activehours", page: Page::Systeme, title: "Planifier les heures d’activité",
            description: "Indique tes heures habituelles d’utilisation à Windows Update.",
            steps: "Dans Windows Update > Options avancées, ouvre Heures d’activité. Note le mode actuel, puis choisis la plage pendant laquelle ton PC doit rester disponible.",
            undo: "Rétablis l’ajustement automatique ou tes anciens horaires.",
            benefit: "Moins de redémarrages inopportuns", risk: "Faible",
            button: "Choisir mes heures d’activité", target: "ms-settings:windowsupdate-activehours",
        },
        Guide {
            id: "autoplay-media", page: Page::Systeme, title: "Choisir ce qui se passe quand tu branches une clé USB",
            description: "Windows peut ouvrir le support, te demander quoi faire ou ne lancer aucune action. Tu peux choisir séparément pour les lecteurs amovibles et les cartes mémoire.",
            steps: "Ouvre Lecture automatique. Note l’état de l’interrupteur et les choix par support. Sélectionne l’action souhaitée pour le type de support concerné, puis rebranche-le pour vérifier.",
            undo: "Remets l’interrupteur et les choix par support dans leur état précédent.",
            benefit: "Comportement adapté à tes supports", risk: "Faible",
            button: "Ouvrir Lecture automatique", target: "ms-settings:autoplay",
        },
        Guide {
            id: "restore-folder-windows", page: Page::Demarrage, title: "Rouvrir tes dossiers à la prochaine connexion",
            description: "Retrouve les fenêtres de dossiers ouvertes à la fermeture de session, ou commence avec un bureau dégagé.",
            steps: "Dans l’Explorateur, ouvre le menu des trois points > Options > Affichage. Repère l’option de restauration des fenêtres de dossiers à l’ouverture de session, note son état, puis coche-la ou décoche-la. Valide avec OK.",
            undo: "Rouvre l’onglet Affichage, rétablis la case telle qu’elle était et clique sur OK. Le choix prend effet à la prochaine connexion.",
            benefit: "Reprise de ton espace de travail selon tes habitudes", risk: "Faible",
            button: "Ouvrir l’Explorateur", target: "shell:MyComputerFolder",
        },
        Guide {
            id: "folder-tip-size", page: Page::Windows, title: "Afficher la taille d’un dossier au survol",
            description: "Choisis si l’Explorateur affiche la taille dans l’info-bulle quand tu laisses la souris sur un dossier.",
            steps: "Dans l’Explorateur, ouvre le menu des trois points > Options > Affichage. Repère l’option qui affiche les informations de taille de fichier dans les info-bulles des dossiers. Note son état, choisis ta préférence, clique sur OK et survole un dossier.",
            undo: "Dans le même onglet, rétablis l’état initial de cette case puis clique sur OK.",
            benefit: "Consultation rapide de la taille ou info-bulles simplifiées", risk: "Faible",
            button: "Ouvrir l’Explorateur", target: "shell:MyComputerFolder",
        },
    ];
    pub struct FerriteApp {
        display_modes: Option<Vec<String>>,
        search: String,
        apps: crate::apps::Apps,
        startup_preview: Result<crate::startup::Saved, String>,
        page: Page,
        selected: BTreeSet<String>,
        simulation: bool,
        confirmation: bool,
        confirm_restore_all: bool,
        store: Store,
        storage_error: Option<String>,
        message: String,
        machine: Option<Machine>,
        machine_error: Option<String>,
        machine_rx: mpsc::Receiver<Result<Machine, String>>,
        supported: bool,
        statuses: BTreeMap<String, Result<bool, String>>,
    }
    impl FerriteApp {
        fn new() -> Self {
            let (store, error) = match load_store() {
                Ok(s) => (s, None),
                Err(e) => (Store::default(), Some(e)),
            };
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || { let _ = tx.send(get_machine()); });
            Self { display_modes: None, search: String::new(), apps: crate::apps::Apps::default(), startup_preview: crate::startup::snapshot(),
                page: Page::Accueil, selected: BTreeSet::new(), simulation: false,
                confirmation: false, confirm_restore_all: false, store, storage_error: error,
                message: "Choisis les réglages à appliquer, puis valide la sélection.".into(),
                machine: None, machine_error: None, machine_rx: rx,
                supported: windows11_supported(), statuses: scan_statuses() }
        }
        fn push_log(&mut self, s: String) {
            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .map(|x| x.as_secs()).unwrap_or(0);
            self.store.log.push(format!("[{ts}] {s}"));
            if self.store.log.len() > 250 { let n = self.store.log.len()-250; self.store.log.drain(..n); }
            self.message = s;
            if let Err(e) = save_store(&self.store) { self.storage_error = Some(e); }
        }
        // False means the current value already satisfies the requested setting.
        fn apply_one(&mut self, t: &Tweak) -> Result<bool, String> {
            if let Some(e) = &self.storage_error { return Err(format!("Sauvegarde indisponible : {e}")); }
            if !self.supported { return Err("Cette application exige Windows 11 pour modifier des réglages.".into()); }
            if self.store.backups.contains_key(t.id) { return Err("Déjà appliqué par Ferrite11 ; restaurer d'abord.".into()); }
            if tweak_satisfied(t)? { return Ok(false); }
            let backup = if matches!(t.operation, Operation::OrphanStartup) {
                Snapshot::Startup(self.startup_preview.clone()?)
            } else { snapshot(t)? };
            self.store.backups.insert(t.id.to_owned(), backup.clone());
            if let Err(e) = save_store(&self.store) {
                self.store.backups.remove(t.id);
                self.storage_error = Some(e.clone());
                return Err(format!("Sauvegarde préalable impossible : {e}"));
            }
            match do_change(t, &backup) {
                Ok(()) => Ok(true),
                Err(e) => {
                    // The Run-key change uses one Windows transaction and cannot partially commit.
                    let rollback = if matches!(t.operation, Operation::OrphanStartup) { Ok(()) }
                        else { can_undo(t, &backup).and_then(|()| undo_snapshot(&backup)) };
                    if let Err(rollback) = rollback {
                        Err(format!("Application échouée : {e}. Restauration partielle impossible : {rollback}. La sauvegarde a été conservée."))
                    } else {
                        self.store.backups.remove(t.id);
                        if let Err(save_error) = save_store(&self.store) {
                            self.store.backups.insert(t.id.to_owned(), backup);
                            self.storage_error = Some(save_error.clone());
                            return Err(format!("Valeurs rétablies après un échec ({e}). Sauvegarde toujours présente : {save_error}"));
                        }
                        Err(format!("Application échouée : {e}. Valeurs initiales rétablies."))
                    }
                }
            }
        }
        fn undo_one(&mut self, t: &Tweak) -> Result<(), String> {
            let b = self.store.backups.get(t.id).cloned().ok_or("Aucune sauvegarde".to_owned())?;
            can_undo(t, &b)?;
            undo_snapshot(&b)?;
            self.store.backups.remove(t.id);
            if let Err(e) = save_store(&self.store) {
                self.store.backups.insert(t.id.to_owned(), b);
                return Err(format!("Paramètres restaurés, mais état de sauvegarde non enregistré : {e}"));
            }
            self.statuses.insert(t.id.to_owned(), tweak_satisfied(t));
            Ok(())
        }
        fn run_apply(&mut self) {
            let found: Vec<_> = tweaks().into_iter()
                .filter(|t| self.selected.contains(t.id)).collect();
            if found.is_empty() { self.message = "Aucune option automatique cochée.".into(); return; }
            if self.simulation {
                self.message = format!("Simulation : {} action(s) sélectionnée(s), aucune écriture.", found.len());
                return;
            }
            let mut errors = Vec::new();
            let mut ok = 0;
            let mut unchanged = 0;
            for t in found {
                if self.storage_error.is_some() { break; }
                match self.apply_one(&t) {
                    Ok(true) => {
                        ok += 1;
                        self.selected.remove(t.id);
                        self.statuses.insert(t.id.to_owned(), tweak_satisfied(&t));
                        self.push_log(format!("APPLIQUÉ : {}", t.title));
                    },
                    Ok(false) => {
                        unchanged += 1;
                        self.selected.remove(t.id);
                        self.statuses.insert(t.id.to_owned(), Ok(true));
                    },
                    Err(e) => {
                        self.statuses.insert(t.id.to_owned(), tweak_satisfied(&t));
                        errors.push(format!("{} : {e}", t.title));
                    }
                }
            }
            self.confirmation = false;
            let summary = format!("{ok} réglage(s) appliqué(s), {unchanged} déjà conforme(s), {} erreur(s). {}", errors.len(), errors.join(" | "));
            if let Some(e) = &self.storage_error {
                self.message = format!("{summary} Traitement interrompu : {e}");
            } else {
                self.push_log(summary);
            }
        }
        fn restore_all(&mut self) {
            let mut errors = Vec::new(); let mut ok = 0;
            for t in tweaks() {
                if self.store.backups.contains_key(t.id) {
                    match self.undo_one(&t) {
                        Ok(()) => { ok += 1; self.push_log(format!("RESTAURÉ : {}", t.title)); },
                        Err(e) => errors.push(format!("{} : {e}", t.title)),
                    }
                }
            }
            self.confirm_restore_all = false;
            self.push_log(format!("Restaurations : {ok} ; erreurs : {}. {}", errors.len(), errors.join(" | ")));
        }
        fn action_card(&mut self, ui: &mut egui::Ui, t: &Tweak) {
            let backed_up = self.store.backups.contains_key(t.id);
            let available = matches!(self.statuses.get(t.id), Some(Ok(false))) && !backed_up;
            if !available && self.selected.remove(t.id) { self.confirmation = false; }
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let mut checked = self.selected.contains(t.id);
                    let label = if backed_up { format!("{}  [appliqué]", t.title) } else { t.title.to_string() };
                    if ui.add_enabled(available && self.supported && self.storage_error.is_none(), egui::Checkbox::new(&mut checked, RichText::new(label).strong().size(17.0))).changed() {
                        if checked { self.selected.insert(t.id.to_owned()); } else { self.selected.remove(t.id); }
                        self.confirmation = false;
                        self.message = format!("{} · {}", t.title, if checked { "ajouté à la sélection" } else { "retiré de la sélection" });
                        ui.ctx().request_repaint();
                    }
                    ui.label(RichText::new(format!("Risque : {}", t.risk)).color(Color32::LIGHT_BLUE));
                });
                if self.selected.contains(t.id) { ui.colored_label(Color32::LIGHT_BLUE, "Sélectionné · en attente de validation"); }
                ui.label(t.description);
                ui.label(RichText::new(format!("Effet attendu : {}", t.effect)).color(Color32::GRAY));
                ui.label(RichText::new(format!("Prise d'effet : {}", t.restart)).small().color(Color32::GRAY));
                match self.statuses.get(t.id) {
                    Some(Ok(true)) => { ui.label(RichText::new("État lu : réglage déjà conforme à l'optimisation proposée.").color(Color32::LIGHT_GREEN).small()); },
                    Some(Err(e)) => { ui.label(RichText::new(format!("État non lisible : {e}")).small().color(Color32::YELLOW)); },
                    None => { ui.small("État inconnu. Actualise les réglages depuis le tableau de bord."); },
                    _ => {},
                }
                if backed_up && ui.add_enabled(self.storage_error.is_none(), egui::Button::new("↩ Restaurer ce réglage")).clicked() {
                    match self.undo_one(t) {
                        Ok(()) => self.push_log(format!("RESTAURÉ : {}", t.title)),
                        Err(e) => self.push_log(format!("ÉCHEC restauration {} : {e}", t.title)),
                    }
                }
            });
            ui.add_space(8.0);
        }
        fn guide_card(&mut self, ui: &mut egui::Ui, g: &Guide) {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let mut done = self.store.manual_done.contains(g.id);
                ui.label(RichText::new(g.title).strong().size(17.0));
                ui.label(g.description);
                ui.label(RichText::new(format!("Bénéfice possible : {}  •  Risque : {}", g.benefit, g.risk)).color(Color32::GRAY));
                egui::CollapsingHeader::new("Voir les étapes et le retour arrière")
                    .id_salt(g.id)
                    .show(ui, |ui| {
                        ui.strong("Marche à suivre");
                        ui.label(g.steps);
                        ui.strong("Retour arrière");
                        ui.label(g.undo);
                    });
                if ui.button(g.button).clicked() {
                    if let Err(e) = open_target(g.target) { self.message = format!("Impossible d'ouvrir la page : {e}"); }
                }
                self.guide_progress(ui, g.id, g.title, &mut done);
            });
            ui.add_space(8.0);
        }
        fn guide_progress(&mut self, ui: &mut egui::Ui, id: &str, title: &str, done: &mut bool) {
                if ui.add_enabled(self.storage_error.is_none(), egui::Checkbox::new(done, "Étape réalisée manuellement")).changed() {
                    if *done { self.store.manual_done.insert(id.to_string()); }
                    else { self.store.manual_done.remove(id); }
                    match save_store(&self.store) {
                        Ok(()) => self.message = format!("{} · {}", title, if *done { "étape marquée comme réalisée" } else { "étape à faire" }),
                        Err(e) => {
                            if *done { self.store.manual_done.remove(id); } else { self.store.manual_done.insert(id.to_owned()); }
                            self.storage_error = Some(e);
                        }
                    }
                }
        }
        fn option_card(&mut self, ui: &mut egui::Ui, g: &OptionGuide) {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.strong(g.title);
                ui.label(g.note);
                egui::CollapsingHeader::new("Détails et retour arrière").id_salt(g.id).show(ui, |ui| {
                    ui.label(g.steps);
                    ui.strong("Retour arrière");
                    ui.label(g.undo);
                    ui.horizontal_wrapped(|ui| {
                        if !g.target.is_empty() && g.target != g.source && ui.button("Ouvrir le réglage ou l’outil").clicked() {
                            if let Err(e) = open_target(g.target) { self.message = e; }
                        }
                        if ui.button("Consulter la source").clicked() {
                            if let Err(e) = open_target(g.source) { self.message = e; }
                        }
                    });
                    let mut done = self.store.manual_done.contains(g.id);
                    self.guide_progress(ui, g.id, g.title, &mut done);
                });
            });
            ui.add_space(8.0);
        }
        fn hardware_summary(&mut self, ui: &mut egui::Ui) {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("Configuration détectée localement");
                if let Some(m) = &self.machine {
                    ui.label(format!("Système : {} (build {})", m.os, m.build));
                    ui.label(format!("Processeur : {}", m.cpu));
                    ui.label(format!("Graphiques : {}", m.gpu));
                    ui.label(format!("Mémoire physique : {:.1} Go", m.ram_gb));
                    ui.label(RichText::new("Détection locale et indicative. Les étapes chipset/GPU non pertinentes sont masquées automatiquement ; aucun réglage matériel n'est forcé.").small().color(Color32::GRAY));
                } else if let Some(error) = &self.machine_error {
                    ui.label(format!("Détection indisponible : {error}"));
                } else {
                    ui.spinner(); ui.label("Détection du matériel en cours…");
                }
            });
        }
        fn guide_relevant(&self, g: &Guide) -> bool {
            let Some(m) = &self.machine else { return true; };
            let cpu = m.cpu.to_lowercase();
            let gpu = m.gpu.to_lowercase();
            match g.id {
                "nvidia" => gpu.contains("nvidia") || gpu.contains("geforce"),
                "radeon" => gpu.contains("radeon") || gpu.contains("amd"),
                "intelgpu" => gpu.contains("intel"),
                "chipset-amd" => cpu.contains("amd") || cpu.contains("ryzen"),
                "chipset-intel" => cpu.contains("intel"),
                _ => true,
            }
        }
        fn home(&mut self, ui: &mut egui::Ui) {
            ui.label("Rechercher dans les réglages et les guides");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Ex. G-SYNC, télémétrie, veille…"));
                if !self.search.is_empty() && ui.button("Effacer").clicked() { self.search.clear(); }
            });
            let query = self.search.trim().to_lowercase();
            if !query.is_empty() {
                let found = |fields: &[&str]| fields.iter().any(|f| f.to_lowercase().contains(&query));
                let mut count = 0;
                for t in tweaks().iter().filter(|t| found(&[t.title, t.description])) {
                    self.action_card(ui, t); count += 1;
                }
                for g in GUIDES.iter().filter(|g| found(&[g.title, g.description, g.steps])) {
                    self.guide_card(ui, g); count += 1;
                }
                for g in options().filter(|g| found(&[g.title, g.note, g.steps])) {
                    self.option_card(ui, g); count += 1;
                }
                if count == 0 { ui.label("Aucun réglage trouvé pour cette recherche."); }
                return;
            }
            ui.heading("Réglages automatiques");
            ui.label("Vérifie les démarrages orphelins, les doublons du PATH et l’affichage des extensions de fichiers. Tu peux retirer chaque proposition avant de valider.");
            if ui.button("Analyser et sélectionner les réglages utiles").clicked() {
                self.statuses = scan_statuses();
                let mut unavailable = 0;
                self.startup_preview = crate::startup::snapshot();
                for id in ["orphan-startup", "path-duplicates", "file-extensions"] {
                    match self.statuses.get(id) {
                        Some(Ok(false)) if !self.store.backups.contains_key(id) => { self.selected.insert(id.to_string()); },
                        Some(Err(_)) | None => { self.selected.remove(id); unavailable += 1; },
                        _ => { self.selected.remove(id); },
                    }
                }
                self.confirm_restore_all = false;
                self.confirmation = false;
                self.message = format!("Analyse terminée · {} réglage(s) dans la sélection · {unavailable} vérification(s) indisponible(s).", self.selected.len());
            }
            ui.add_space(12.0);
            ui.heading("Choisir une catégorie");
            ui.horizontal_wrapped(|ui| {
                for page in Page::ALL {
                    if !matches!(page, Page::Accueil | Page::Journal) && ui.button(page.label()).clicked() { self.page = page; }
                }
            });
            ui.add_space(12.0);
            self.hardware_summary(ui);
            if ui.button("Actualiser les états des réglages").clicked() { self.statuses = scan_statuses(); }
            if !self.supported {
                ui.colored_label(Color32::YELLOW, "Windows 11 non détecté : modifications automatiques bloquées par sécurité.");
            }
            ui.add_space(8.0);
            let total_done = self.store.manual_done.len();
            let changes = self.store.backups.len();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{changes} réglage(s) géré(s) par Ferrite11")).strong());
                ui.separator();
                ui.label(format!("{total_done} étape(s) manuelle(s) cochée(s)"));
                ui.separator();
                ui.label(format!("{} action(s) sélectionnée(s)", self.selected.len()));
            });
            ui.add_space(14.0);
            if ui.button("Ouvrir la protection du système").clicked() {
                if let Err(e) = Command::new("SystemPropertiesProtection.exe").spawn() { self.message = e.to_string(); }
            }
        }
        fn content(&mut self, ui: &mut egui::Ui) {
            match self.page {
                Page::Accueil => self.home(ui),
                Page::Journal => {
                    ui.heading("Journal et sauvegardes");
                    ui.label(format!("Fichier : {}", storage_path().display()));
                    ui.label("Les sauvegardes sont réalisées avant chaque modification. Toute restauration en conflit avec un changement externe est bloquée.");
                    if ui.button("Ouvrir le dossier des sauvegardes").clicked() {
                        if let Some(dir) = storage_path().parent() { let _ = Command::new("explorer.exe").arg(dir).spawn(); }
                    }
                    ui.separator();
                    for item in self.store.log.iter().rev().take(70) { ui.label(item); }
                }
                Page::Outils => {
                    self.apps.ui(ui, self.simulation);
                    ui.collapsing("Autres outils à installer manuellement", |ui| {
                        ui.label("GlazeWM organise les fenêtres avec ses propres raccourcis. Consulte sa configuration avant de l’activer.");
                        if ui.button("Projet GlazeWM").clicked() { if let Err(e) = open_target("https://github.com/glzr-io/glazewm") { self.message = e; } }
                        ui.label("Autoruns permet d’examiner les démarrages avancés. Note l’état de chaque entrée pour pouvoir la réactiver.");
                        if ui.button("Autoruns de Microsoft").clicked() { if let Err(e) = open_target("https://learn.microsoft.com/fr-fr/sysinternals/downloads/autoruns") { self.message = e; } }
                    });
                }
                page => {
                    ui.heading(page.label());
                    if matches!(page, Page::Jeux | Page::Nvidia) {
                        if ui.button("Vérifier la fréquence des écrans").clicked() {
                            self.display_modes = Some(crate::display_modes::summary());
                        }
                        if let Some(outputs) = &self.display_modes {
                            for output in outputs { ui.label(output); }
                            if ui.button("Ouvrir Affichage avancé").clicked() {
                                if let Err(e) = open_target("ms-settings:display-advanced") { self.message = e; }
                            }
                        }
                    }
                    if page == Page::Jeux {
                        ui.label("Pour jouer sans capture Xbox, prépare ensemble le Mode Jeu, l’arrêt des captures et la désactivation du raccourci manette de la Game Bar.");
                        if ui.button("Préparer ces trois réglages de jeu").clicked() {
                            self.statuses = scan_statuses();
                            let mut added = 0;
                            let mut unknown = 0;
                            for id in ["game-mode", "capture", "gamebar-controller"] {
                                match self.statuses.get(id) {
                                    Some(Ok(false)) if !self.store.backups.contains_key(id) => {
                                        self.selected.insert(id.to_owned());
                                        added += 1;
                                    }
                                    status => {
                                        self.selected.remove(id);
                                        if !matches!(status, Some(Ok(_))) { unknown += 1; }
                                    }
                                }
                            }
                            self.confirmation = false;
                            self.confirm_restore_all = false;
                            self.message = format!("Profil jeu · {added} réglage(s) à valider, {unknown} état(s) illisible(s).");
                        }
                    }
                    if page == Page::Nvidia && self.machine.as_ref().is_some_and(|m| {
                        let gpu = m.gpu.to_lowercase();
                        !gpu.contains("nvidia") && !gpu.contains("geforce")
                    }) { ui.label("Aucun GPU NVIDIA détecté sur ce PC. Ces réglages concernent les machines équipées d’un pilote NVIDIA."); }
                    if page == Page::Materiel { self.hardware_summary(ui); ui.add_space(9.0); }
                    let actions = tweaks();
                    if actions.iter().any(|t| t.page == page) {
                        ui.heading("Réglages à valider");
                        for t in actions.iter().filter(|t| t.page == page) { self.action_card(ui, t); }
                    }
                    let guides: Vec<&Guide> = GUIDES.iter().filter(|g| g.page == page && self.guide_relevant(g)).collect();
                    if !guides.is_empty() {
                        ui.heading("Étapes manuelles");
                        ui.label("Ouvre le guide, réalise l’étape puis coche-la pour enregistrer ton avancement.");
                    }
                    for g in guides { self.guide_card(ui, g); }
                    for g in options().filter(|g| g.page == page) { self.option_card(ui, g); }
                }
            }
        }
        fn bottom_actions(&mut self, ui: &mut egui::Ui) {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Sélection : {}", self.selected.len()));
                if ui.checkbox(&mut self.simulation, "Mode simulation").changed() {
                    self.confirmation = false;
                    self.message = if self.simulation { "Valider affichera une simulation." } else { "Valider affichera le récapitulatif avant application." }.into();
                }
                if ui.add_enabled(!self.selected.is_empty() && self.storage_error.is_none() && self.supported,
                    egui::Button::new(RichText::new("Valider").strong())).clicked() {
                    self.confirm_restore_all = false;
                    if self.selected.contains("orphan-startup") { self.startup_preview = crate::startup::snapshot(); }
                    self.confirmation = true;
                    self.message = if self.simulation { "Simulation terminée. Aucun réglage Windows modifié." } else { "Vérifie la sélection puis confirme son application." }.into();
                }
                if ui.add_enabled(self.storage_error.is_none() && !self.store.backups.is_empty(), egui::Button::new("Restaurer les réglages Windows")).clicked() {
                    self.confirmation = false;
                    self.confirm_restore_all = true;
                }
            });
            if self.confirmation {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(if self.simulation { "PRÉVISUALISATION" } else { "CONFIRMATION AVANT MODIFICATION" }).strong());
                    let available: Vec<_> = tweaks().into_iter().filter(|t| self.selected.contains(t.id)).collect();
                    if available.is_empty() { ui.label("Aucun réglage automatique sélectionné."); }
                    for t in &available {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(t.title);
                            ui.label(format!("{} · risque {}", t.page.label(), t.risk));
                            if ui.small_button("Retirer").clicked() {
                                self.selected.remove(t.id);
                                self.confirmation = !self.selected.is_empty();
                            }
                        });
                        match &t.operation {
                            Operation::Registry(changes) => for c in changes {
                                let target = match &c.desired { Desired::Dword(v) => v.to_string(), Desired::Text(v) => (*v).to_string() };
                                ui.label(RichText::new(format!("  HKCU\\{} : {} → {}", c.path, c.name, target)).small().color(Color32::GRAY));
                            },
                            Operation::OrphanStartup => match &self.startup_preview {
                                Ok(entries) if entries.is_empty() => { ui.small("Aucune entrée orpheline détectée."); },
                                Ok(entries) => for name in entries.names() { ui.label(format!("• {name}")); },
                                Err(e) => { ui.colored_label(Color32::YELLOW, e); },
                            },
                            Operation::CleanPath => { ui.small("Sauvegarde du PATH actuel, puis retrait des entrées identiques."); },
                            Operation::BalancedPower => { ui.label(RichText::new("  powercfg /setactive SCHEME_BALANCED").small().color(Color32::GRAY)); }
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.button(if self.simulation { "Fermer la simulation" } else { "Annuler" }).clicked() { self.confirmation = false; }
                        if !self.simulation && !available.is_empty() && ui.button("Confirmer et appliquer").clicked() { self.run_apply(); }
                    });
                });
            }
            if self.confirm_restore_all {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label("Restaurer les valeurs d'origine enregistrées par Ferrite11 ? Les modifications effectuées ailleurs depuis seront respectées.");
                    ui.horizontal(|ui| {
                        if ui.button("Annuler").clicked() { self.confirm_restore_all = false; }
                        if ui.button("Confirmer la restauration").clicked() { self.restore_all(); }
                    });
                });
            }
            ui.label(RichText::new(&self.message).small().color(Color32::LIGHT_BLUE));
        }
    }
    impl eframe::App for FerriteApp {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            self.apps.poll(ui.ctx());
            if self.machine.is_none() && self.machine_error.is_none() {
                match self.machine_rx.try_recv() {
                    Ok(Ok(machine)) => self.machine = Some(machine),
                    Ok(Err(e)) => self.machine_error = Some(e),
                    Err(mpsc::TryRecvError::Disconnected) => self.machine_error = Some("Processus de détection interrompu".into()),
                    Err(mpsc::TryRecvError::Empty) => ui.ctx().request_repaint_after(Duration::from_millis(350)),
                }
            }
            ui.ctx().set_visuals(egui::Visuals::dark());
            egui::Panel::top("header").resizable(false).show(ui, |ui| {
                ui.heading(RichText::new("FERRITE11   /   RUST WINDOWS OPTIMIZER").size(22.0).strong());
                ui.label(RichText::new("Optimiseur universel • configuration locale • réversible • aucun profil matériel imposé").color(Color32::GRAY));
                if let Some(e) = &self.storage_error {
                    ui.colored_label(Color32::RED, format!("Mode lecture seule : {e}. Corrige le fichier de sauvegarde avant toute modification."));
                }
            });
            let actions_height = ui.available_height() * 0.45;
            egui::Panel::bottom("actions").resizable(false).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("action-scroll")
                    .max_height(actions_height).show(ui, |ui| self.bottom_actions(ui));
            });
            egui::Panel::left("navigation").exact_size(204.0).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("navigation-scroll").show(ui, |ui| {
                        for p in Page::ALL {
                            if ui.selectable_label(self.page == p, p.label()).clicked() { self.page = p; self.confirmation = false; }
                            ui.add_space(6.0);
                        }
                        ui.separator();
                        ui.label(RichText::new("Aucun service critique modifié").small().color(Color32::LIGHT_GREEN));
                        ui.label(RichText::new("Sauvegarde : %LOCALAPPDATA%\\Ferrite11").small().color(Color32::GRAY));
                });
            });
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt(("content", self.page.label()))
                    .auto_shrink([false, false]).show(ui, |ui| self.content(ui));
            });
        }
    }

    const WINDOW_TITLE: &str = "Ferrite11 — Optimiseur Windows 11";

    fn lock_interface() -> io::Result<Option<std::os::windows::io::OwnedHandle>> {
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use std::ptr::null;
        use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
        use windows_sys::Win32::System::Threading::CreateMutexW;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, IsIconic, MB_ICONINFORMATION, MB_OK, MessageBoxW, SW_RESTORE,
            SetForegroundWindow, ShowWindow,
        };

        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let name = wide(r"Local\Ferrite11-Interface");
        // Named kernel object; the last handle disappearing also releases the guard on a crash.
        let raw = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
        if raw.is_null() { return Err(io::Error::last_os_error()); }
        let already_open = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        if !already_open { return Ok(Some(handle)); }

        let title = wide(WINDOW_TITLE);
        let window = unsafe { FindWindowW(null(), title.as_ptr()) };
        if !window.is_null() {
            if unsafe { IsIconic(window) } != 0 { unsafe { ShowWindow(window, SW_RESTORE); } }
            if unsafe { SetForegroundWindow(window) } != 0 { return Ok(None); }
        }
        let message = wide("Ferrite11 est déjà ouvert. Retrouve la fenêtre dans la barre des tâches.");
        unsafe { MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION); }
        Ok(None)
    }

    pub fn run() -> eframe::Result {
        let Some(_instance) = lock_interface().map_err(|e| eframe::Error::AppCreation(e.into()))? else {
            return Ok(());
        };
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_title(WINDOW_TITLE)
                .with_inner_size([1130.0, 820.0]).with_min_inner_size([850.0, 640.0]),
            ..Default::default()
        };
        eframe::run_native("Ferrite11", options, Box::new(|_| Ok(Box::new(FerriteApp::new()))))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn guide_ids_are_stable_and_routes_available() {
            let mut ids = BTreeSet::new();
            for g in GUIDES { assert!(ids.insert(g.id), "{}", g.id); }
            for g in options() {
                assert!(ids.insert(g.id), "{}", g.id);
                assert!(Page::ALL.contains(&g.page));
                assert!(g.source.starts_with("https://"));
                assert!(!g.steps.is_empty() && !g.undo.is_empty());
            }
            for id in ["updates-pause-test", "classic-context", "winutil-profile", "nv-hdcp", "lat-timers"] {
                assert!(ids.contains(id), "{id}");
            }
        }
        #[test]
        fn parses_power_plan_guid() {
            assert_eq!(parse_guid("GUID du mode de gestion de l'alimentation : 381b4222-f694-41f0-9685-ff5bb260df2e (Utilisation normale)").as_deref(), Some(BALANCED_GUID));
        }
        #[test]
        fn rejects_random_text() { assert_eq!(parse_guid("Aucun GUID disponible"), None); }
    }
}

#[cfg(windows)]
fn main() -> eframe::Result {
    if let Some(result) = apps::launch_from_args() {
        if let Err(message) = result {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR};
            let caption: Vec<_> = "Ferrite11".encode_utf16().chain(Some(0)).collect();
            let message: Vec<_> = message.encode_utf16().chain(Some(0)).collect();
            unsafe { MessageBoxW(std::ptr::null_mut(), message.as_ptr(), caption.as_ptr(), MB_ICONERROR); }
            std::process::exit(1);
        }
        return Ok(());
    }
    app::run()
}
