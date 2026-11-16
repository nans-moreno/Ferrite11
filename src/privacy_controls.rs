use super::{Store, save_store};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, sync::LazyLock};
use winreg::{
    HKCU, HKLM, RegKey, RegValue,
    enums::{KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD, REG_SZ},
    transaction::Transaction,
};

fn tr(fr: &'static str, en: &'static str) -> &'static str {
    if crate::language::english() { en } else { fr }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum Value {
    Number(u32),
    Text(String),
}

#[derive(Deserialize)]
struct Rule {
    machine: bool,
    path: String,
    name: String,
    on: Value,
    off: Option<Value>,
}
#[derive(Deserialize)]
struct Feature {
    id: String,
    key: String,
    group: String,
    scope: String,
    rating: String,
    fr: String,
    en: String,
    description_fr: String,
    description_en: String,
    min: u32,
    max: u32,
    edition: String,
    guide: Option<String>,
    rules: Vec<Rule>,
}
static CATALOG: LazyLock<Vec<Feature>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("privacy_catalog.json"))
        .expect("valid built-in Windows settings")
});
impl Feature {
    fn title(&self) -> &str {
        if crate::language::english() {
            &self.en
        } else {
            &self.fr
        }
    }
    fn description(&self) -> &str {
        if crate::language::english() {
            &self.description_en
        } else {
            &self.description_fr
        }
    }
    fn matches(&self, q: &str) -> bool {
        [
            &self.id,
            &self.fr,
            &self.en,
            self.description(),
            group_name(&self.group),
        ]
        .iter()
        .any(|s| s.to_lowercase().contains(q))
    }
    fn desired(&self, on: bool) -> Vec<Option<Value>> {
        self.rules
            .iter()
            .map(|r| {
                if on {
                    Some(r.on.clone())
                } else {
                    r.off.clone()
                }
            })
            .collect()
    }
    fn read(&self) -> Result<Vec<Option<Value>>, String> {
        self.rules.iter().map(|r| r.read(None)).collect()
    }
    fn available(&self, build: u32, edition: &str) -> bool {
        build >= self.min
            && (self.max == 0 || build <= self.max)
            && (self.edition == "All"
                || self.edition.split(", ").any(|e| match e {
                    "Pro" => edition.starts_with("Professional"),
                    "Enterprise" => edition.contains("Enterprise"),
                    "Education" => edition.contains("Education"),
                    "IoT" => edition.contains("IoT"),
                    "Server" => edition.contains("Server"),
                    _ => false,
                }))
    }
}

fn failure(e: io::Error) -> String {
    if e.kind() == io::ErrorKind::PermissionDenied {
        tr("Accès refusé. Pour un réglage du PC, ferme Ferrite11 et utilise « Exécuter en tant qu’administrateur ». Une stratégie de l’organisation peut aussi protéger cette valeur.",
           "Access denied. For PC settings, close Ferrite11 and use Run as administrator. An organization policy may also protect this value.").into()
    } else {
        e.to_string()
    }
}
impl Rule {
    fn hive(&self) -> &'static RegKey {
        if self.machine { HKLM } else { HKCU }
    }
    fn read(&self, tx: Option<&Transaction>) -> Result<Option<Value>, String> {
        let hive = self.hive();
        let key = match tx {
            Some(t) => hive.open_subkey_transacted_with_flags(&self.path, t, KEY_QUERY_VALUE),
            None => hive.open_subkey_with_flags(&self.path, KEY_QUERY_VALUE),
        };
        let raw = match key.and_then(|k| k.get_raw_value(&self.name)) {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if self
                    .path
                    .to_ascii_lowercase()
                    .starts_with(r"system\currentcontrolset\services\")
                {
                    return Err(tr(
                        "Ce composant Windows est absent.",
                        "This Windows component is absent.",
                    )
                    .into());
                }
                return Ok(None);
            }
            Err(e) => return Err(failure(e)),
        };
        self.decode(&raw).map(Some)
    }
    fn decode(&self, raw: &RegValue<'_>) -> Result<Value, String> {
        match (&self.on, &raw.vtype) {
            (Value::Number(_), &REG_DWORD) if raw.bytes.len() == 4 => Ok(Value::Number(
                u32::from_le_bytes(raw.bytes[..].try_into().unwrap()),
            )),
            (Value::Text(_), &REG_SZ) if raw.bytes.len() % 2 == 0 => {
                let mut w = raw
                    .bytes
                    .chunks_exact(2)
                    .map(|v| u16::from_le_bytes([v[0], v[1]]))
                    .collect::<Vec<_>>();
                if w.pop() == Some(0) && !w.contains(&0) {
                    return String::from_utf16(&w)
                        .map(Value::Text)
                        .map_err(|e| e.to_string());
                }
                Err(tr("Texte Windows illisible.", "Invalid Windows string.").into())
            }
            _ => Err(tr(
                "Type de valeur Windows inattendu.",
                "Unexpected Windows value type.",
            )
            .into()),
        }
    }
    fn put(&self, tx: &Transaction, value: &Option<Value>) -> Result<(), String> {
        let hive = self.hive();
        match value {
            None => hive
                .open_subkey_transacted_with_flags(&self.path, tx, KEY_SET_VALUE)
                .and_then(|k| k.delete_value(&self.name)),
            Some(v) => hive
                .create_subkey_transacted_with_flags(&self.path, tx, KEY_SET_VALUE)
                .and_then(|(k, _)| match v {
                    Value::Number(n) => k.set_value(&self.name, n),
                    Value::Text(s) => k.set_value(&self.name, s),
                }),
        }
        .map_err(failure)
    }
    fn address(&self) -> String {
        format!(
            "{}\\{}\\{}",
            if self.machine { "HKLM" } else { "HKCU" },
            self.path,
            self.name
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Backup {
    addresses: Vec<String>,
    before: Vec<Option<Value>>,
    applied: bool,
}
type Values = Vec<Option<Value>>;
#[derive(Clone)]
enum Choice {
    Set(bool, Values),
    Restore,
}

fn original_values(f: &Feature, b: &Backup, current: &Values) -> Result<Values, String> {
    if b.addresses != f.rules.iter().map(Rule::address).collect::<Vec<_>>()
        || b.before.len() != f.rules.len()
        || f.rules.iter().zip(&b.before).any(|(r, v)| {
            v.as_ref()
                .is_some_and(|v| std::mem::discriminant(v) != std::mem::discriminant(&r.on))
        })
    {
        return Err(tr(
            "Sauvegarde incompatible avec ce réglage.",
            "Backup does not match this setting.",
        )
        .into());
    }
    let applied = f.desired(b.applied);
    if current != &applied && current != &b.before {
        return Err(tr(
            "Valeur modifiée ailleurs depuis la sauvegarde.",
            "Value changed elsewhere since backup.",
        )
        .into());
    }
    Ok(b.before.clone())
}

fn shared_native_id(f: &Feature) -> Option<&'static str> {
    match f.id.as_str() {
        "P015" => Some("website-language-list"),
        "P025" => Some("app-launch-tracking"),
        "G001" => Some("capture"),
        _ => None,
    }
}
pub(super) fn owns_native(id: &str, store: &Store) -> bool {
    CATALOG
        .iter()
        .any(|f| shared_native_id(f) == Some(id) && store.privacy.contains_key(&f.key))
}

fn execute(f: &Feature, choice: &Choice, store: &mut Store) -> Result<(), String> {
    if f.rules.is_empty() {
        return Err(tr(
            "Ce réglage est guidé ou indisponible.",
            "This setting is guided or unavailable.",
        )
        .into());
    }
    let tx = Transaction::new().map_err(failure)?;
    let current = f
        .rules
        .iter()
        .map(|r| r.read(Some(&tx)))
        .collect::<Result<Values, _>>()?;
    let target = match choice {
        Choice::Set(on, seen) => {
            if shared_native_id(f).is_some_and(|id| store.backups.contains_key(id)) {
                return Err(tr(
                    "Restaure d’abord le réglage correspondant dans son autre rubrique.",
                    "Restore the corresponding setting in its other section first.",
                )
                .into());
            }
            if store.privacy.contains_key(&f.key) || seen != &current {
                return Err(tr(
                    "L’état a changé. Actualise puis vérifie la sélection.",
                    "State changed. Refresh and review the selection.",
                )
                .into());
            }
            f.desired(*on)
        }
        Choice::Restore => original_values(
            f,
            store
                .privacy
                .get(&f.key)
                .ok_or_else(|| tr("Sauvegarde absente.", "Missing backup.").to_string())?,
            &current,
        )?,
    };
    for ((r, old), new) in f.rules.iter().zip(&current).zip(&target) {
        if old != new {
            r.put(&tx, new)?;
            if r.read(Some(&tx))? != *new {
                return Err(tr("Écriture non confirmée.", "Write could not be verified.").into());
            }
        }
    }
    if let Choice::Set(on, _) = choice {
        store.privacy.insert(
            f.key.clone(),
            Backup {
                addresses: f.rules.iter().map(Rule::address).collect(),
                before: current,
                applied: *on,
            },
        );
        if let Err(e) = save_store(store) {
            store.privacy.remove(&f.key);
            return Err(e);
        }
    }
    // A failed or interrupted commit leaves the original snapshot available for recovery.
    tx.commit().map_err(failure)?;
    if f.read()? != target {
        return Err(tr(
            "La valeur relue diffère. La sauvegarde est conservée.",
            "Read-back differs. The backup has been kept.",
        )
        .into());
    }
    if matches!(choice, Choice::Restore) {
        let backup = store.privacy.remove(&f.key).unwrap();
        if let Err(e) = save_store(store) {
            store.privacy.insert(f.key.clone(), backup);
            return Err(e);
        }
    }
    Ok(())
}
pub(super) fn restore_all(store: &mut Store) -> (usize, Vec<String>) {
    let keys = store.privacy.keys().cloned().collect::<Vec<_>>();
    let mut restored = 0;
    let mut errors = Vec::new();
    for key in keys {
        let result = CATALOG
            .iter()
            .find(|f| f.id == key)
            .ok_or_else(|| format!("{} · {key}", tr("Réglage inconnu", "Unknown setting")))
            .and_then(|f| execute(f, &Choice::Restore, store));
        match result {
            Ok(()) => restored += 1,
            Err(e) => errors.push(format!("{key} · {e}")),
        }
    }
    (restored, errors)
}

#[derive(Default)]
pub(super) struct Panel {
    initialized: bool,
    build: u32,
    edition: String,
    query: String,
    scope: Option<&'static str>,
    show_all: bool,
    states: BTreeMap<String, Result<Values, String>>,
    choices: BTreeMap<String, Choice>,
    review: bool,
    pub(super) message: String,
}
impl Panel {
    pub(super) fn refresh(&mut self) {
        let os = HKLM
            .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
            .ok();
        self.build = os
            .as_ref()
            .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber").ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        self.edition = os
            .and_then(|k| k.get_value("EditionID").ok())
            .unwrap_or_default();
        self.states = CATALOG
            .iter()
            .filter(|f| {
                f.available(self.build, &self.edition) && !f.rules.is_empty() && f.id == f.key
            })
            .map(|f| (f.key.clone(), f.read()))
            .collect();
        self.choices.clear();
        self.review = false;
        self.initialized = true;
    }
    pub(super) fn search(&mut self, q: &str) {
        self.query = q.to_owned();
        self.scope = None;
        self.show_all = true;
    }
    pub(super) fn match_count(&self, q: &str) -> usize {
        CATALOG.iter().filter(|f| f.matches(q)).count()
    }
    pub(super) fn content(&mut self, ui: &mut egui::Ui, store: &Store, enabled: bool) {
        if !self.initialized {
            self.refresh();
        }
        ui.heading(title());
        ui.label(tr("Choisis un réglage, lis son effet, puis valide la sélection. Les valeurs modifiées sont sauvegardées avant application.",
                    "Choose a setting, read its effect, then review the selection. Changed values are backed up before applying."));
        if ui
            .button(tr(
                "Actualiser les valeurs Windows",
                "Refresh Windows values",
            ))
            .clicked()
        {
            self.refresh();
        }
        ui.add(egui::TextEdit::singleline(&mut self.query).hint_text(tr(
            "Rechercher par nom, effet ou identifiant…",
            "Search by name, effect or ID…",
        )));
        ui.horizontal_wrapped(|ui| {
            for (scope, label) in [
                (None, tr("Tous", "All")),
                (Some("user"), tr("Mon compte", "My account")),
                (Some("machine"), tr("Tout le PC", "Whole PC")),
            ] {
                ui.selectable_value(&mut self.scope, scope, label);
            }
            ui.checkbox(
                &mut self.show_all,
                tr(
                    "Afficher les réglages incompatibles",
                    "Show incompatible settings",
                ),
            );
        });
        let q = self.query.trim().to_lowercase();
        let mut groups = BTreeMap::<&str, Vec<&Feature>>::new();
        for f in CATALOG.iter() {
            if (self.show_all
                || f.available(self.build, &self.edition)
                || store.privacy.contains_key(&f.key))
                && self.scope.is_none_or(|s| s == f.scope)
                && f.matches(&q)
            {
                groups.entry(group_name(&f.group)).or_default().push(f);
            }
        }
        if groups.is_empty() {
            ui.label(tr("Aucun réglage trouvé.", "No settings found."));
        }
        for (group, rows) in groups {
            let header = egui::CollapsingHeader::new(format!("{group} ({})", rows.len()))
                .id_salt(("privacy", rows[0].group.as_str()));
            let header = if q.is_empty() {
                header
            } else {
                header.open(Some(true))
            };
            header.show(ui, |ui| {
                if rows[0].group == "NewEdge" {
                    ui.small(tr(
                        "Edge indique les stratégies reconnues pour sa version dans edge://policy.",
                        "Edge lists policies recognized by the installed version at edge://policy.",
                    ));
                }
                for f in rows {
                    ui.push_id(&f.id, |ui| {
                        let backup = store.privacy.contains_key(&f.key);
                        let compatible = f.available(self.build, &self.edition);
                        let state = self.states.get(&f.key).and_then(|r| r.as_ref().ok());
                        let on = state.is_some_and(|s| s == &f.desired(true));
                        let other_backup =
                            shared_native_id(f).is_some_and(|id| store.backups.contains_key(id));
                        let mut chosen = self.choices.contains_key(&f.key);
                        if ui
                            .add_enabled(
                                enabled
                                    && (backup || (!other_backup && compatible && state.is_some())),
                                egui::Checkbox::new(&mut chosen, f.title()),
                            )
                            .changed()
                        {
                            if chosen {
                                self.choices.insert(
                                    f.key.clone(),
                                    if backup {
                                        Choice::Restore
                                    } else {
                                        Choice::Set(!on, state.unwrap().clone())
                                    },
                                );
                            } else {
                                self.choices.remove(&f.key);
                            }
                            self.review = false;
                        }
                        ui.label(f.description());
                        if f.scope == "machine" && f.guide.is_none() && compatible {
                            ui.small(tr(
                                "Une autorisation administrateur peut être nécessaire.",
                                "Administrator rights may be required.",
                            ));
                        }
                        let scope = if f.scope == "machine" {
                            tr("Tout le PC", "Whole PC")
                        } else {
                            tr("Mon compte", "My account")
                        };
                        let rating = match f.rating.as_str() {
                            "Critical" => tr("Déconseillé", "Not recommended"),
                            "Limited" => tr("Selon ton usage", "Depends on your needs"),
                            _ => tr("Recommandé", "Recommended"),
                        };
                        let status = if let Some(choice) = self.choices.get(&f.key) {
                            match choice {
                                Choice::Set(true, _) => {
                                    tr("Sélectionné · appliquer", "Selected · apply")
                                }
                                Choice::Set(false, _) => {
                                    tr("Sélectionné · retirer la règle", "Selected · turn rule off")
                                }
                                Choice::Restore => tr(
                                    "Sélectionné · restaurer les valeurs initiales",
                                    "Selected · restore original values",
                                ),
                            }
                        } else if other_backup {
                            tr(
                                "Déjà géré dans une autre rubrique de Ferrite11",
                                "Already managed in another Ferrite11 section",
                            )
                        } else if backup {
                            tr(
                                "Valeurs initiales sauvegardées · cocher pour restaurer",
                                "Original values saved · select to restore",
                            )
                        } else if !compatible {
                            tr(
                                "Indisponible pour cette version ou édition de Windows",
                                "Unavailable on this Windows build or edition",
                            )
                        } else if f.guide.is_some() {
                            tr("Réglage manuel", "Manual setting")
                        } else if on {
                            tr(
                                "Règle configurée · cocher pour la retirer",
                                "Rule configured · select to turn off",
                            )
                        } else {
                            tr(
                                "Règle non appliquée ou partielle · cocher pour appliquer",
                                "Rule unset or partly applied · select to apply",
                            )
                        };
                        ui.small(format!("{} · {scope} · {rating} · {status}", f.id));
                        if let Some(Err(e)) = self.states.get(&f.key) {
                            ui.label(e);
                        }
                        if compatible
                            && let Some(uri) = &f.guide
                            && ui.button(tr("Ouvrir le réglage", "Open setting")).clicked()
                        {
                            if let Err(e) = crate::shell::open_target(uri) {
                                self.message = e;
                            }
                        }
                        ui.add_space(6.);
                    });
                }
            });
        }
    }
    pub(super) fn actions(
        &mut self,
        ui: &mut egui::Ui,
        simulation: &mut bool,
        store: &mut Store,
        enabled: bool,
    ) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.strong(crate::local_format!(
                "Sélection : {}",
                "Selected: {}",
                self.choices.len()
            ));
            if ui
                .checkbox(simulation, crate::language::text("Mode simulation"))
                .changed()
            {
                self.review = false;
            }
            if ui
                .add_enabled(
                    enabled && !self.choices.is_empty(),
                    egui::Button::new(crate::language::text("Valider")),
                )
                .clicked()
            {
                self.review = true;
            }
        });
        if self.review {
            let title = if *simulation {
                tr("Simulation", "Preview")
            } else {
                tr("Changements à confirmer", "Changes to confirm")
            };
            let cancel = crate::language::text(if *simulation {
                "Fermer la simulation"
            } else {
                "Annuler"
            });
            let confirm = (!*simulation).then(|| crate::language::text("Confirmer et appliquer"));
            let action = super::review_dialog(ui, "privacy-review", title, cancel, confirm, |ui| {
                for (id, choice) in &self.choices {
                    if let Some(f) = CATALOG.iter().find(|f| &f.id == id) {
                        let verb = match choice {
                            Choice::Restore => tr("Restaurer", "Restore"),
                            Choice::Set(true, _) => tr("Appliquer", "Apply"),
                            Choice::Set(false, _) => tr("Retirer la règle", "Turn rule off"),
                        };
                        ui.label(format!("{verb} · {id} · {}", f.title()));
                    }
                }
                enabled && !self.choices.is_empty()
            });
            if matches!(action, super::ReviewAction::Confirm) {
                let mut errors = Vec::new();
                let mut count = 0;
                for (id, choice) in &self.choices {
                    let Some(f) = CATALOG.iter().find(|f| &f.id == id) else {
                        continue;
                    };
                    match execute(f, choice, store) {
                        Ok(()) => count += 1,
                        Err(e) => errors.push(format!("{id} · {e}")),
                    }
                }
                self.refresh();
                self.message = crate::local_format!(
                    "{} changement(s) confirmé(s). {}",
                    "{} change(s) confirmed. {}",
                    count,
                    errors.join(" | ")
                );
                changed = true;
            }
            if matches!(action, super::ReviewAction::Close) {
                self.review = false;
            }
        }
        super::footer_message(ui, &self.message);
        changed
    }
}
pub(super) fn title() -> &'static str {
    tr("Contrôles de confidentialité", "Privacy controls")
}
fn group_name(group: &str) -> &str {
    match group {
        "ActivityHistory" => tr("Historique et presse-papiers", "History and clipboard"),
        "AppPrivacy" => tr("Autorisations des applications", "App permissions"),
        "Cortana" => "Cortana",
        "Edge" => tr("Edge classique", "Legacy Edge"),
        "NewEdge" => "Microsoft Edge (Chromium)",
        "Gaming" => tr("Jeux et captures", "Gaming and captures"),
        "Location" => tr("Localisation", "Location"),
        "LockScreen" => tr("Écran de verrouillage", "Lock screen"),
        "Miscellaneous" => tr("Autres réglages Windows", "Other Windows settings"),
        "MobileDevices" => tr("Appareils mobiles", "Mobile devices"),
        "Privacy" => tr("Confidentialité générale", "General privacy"),
        "Search" => tr("Recherche", "Search"),
        "Security" => tr("Sécurité Windows", "Windows security"),
        "Synchronization" => tr("Synchronisation", "Synchronization"),
        "Taskbar" => tr("Barre des tâches", "Taskbar"),
        "UserBehavior" => tr("Télémétrie et utilisation", "Telemetry and usage"),
        "WindowsAI" => tr("Intelligence artificielle Windows", "Windows AI"),
        "WindowsDefender" => "Microsoft Defender",
        "WindowsExplorer" => tr("Explorateur Windows", "File Explorer"),
        "WindowsUpdate" => "Windows Update",
        _ => group,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_feature_has_bilingual_help_and_an_implemented_route() {
        let mut ids = std::collections::BTreeSet::new();
        for f in CATALOG.iter() {
            assert!(ids.insert(f.id.clone()));
            assert!(
                f.description_en.len() > 30
                    && f.description_fr.len() > 30
                    && f.description_en != f.en
                    && f.description_fr != f.fr,
                "{}",
                f.id
            );
            assert!(
                !f.rules.is_empty() || f.guide.is_some() || (f.max > 0 && f.max < 22000),
                "{}",
                f.id
            );
            let canonical = CATALOG.iter().find(|c| c.id == f.key).unwrap();
            assert_eq!(canonical.desired(true), f.desired(true));
            assert_eq!(
                canonical
                    .rules
                    .iter()
                    .map(Rule::address)
                    .collect::<Vec<_>>(),
                f.rules.iter().map(Rule::address).collect::<Vec<_>>()
            );
        }
        assert_eq!(ids.len(), 346);
        assert!(CATALOG.iter().any(|f| f.matches("p015")));
        let old: Store =
            serde_json::from_str(r#"{"backups":{},"manual_done":[],"log":[]}"#).unwrap();
        assert!(old.privacy.is_empty());
    }

    #[test]
    fn preview_from_checkbox_keeps_settings_unchanged() {
        let mut panel = Panel {
            initialized: true,
            build: 26100,
            edition: "Professional".into(),
            query: "P015".into(),
            ..Default::default()
        };
        panel.states.insert("P015".into(), Ok(vec![None]));
        let f = CATALOG.iter().find(|f| f.id == "P015").unwrap();
        let mut store = Store::default();
        let mut simulation = true;
        let ctx = egui::Context::default();
        let mut draw = |events| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200., 1400.),
                )),
                events,
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                panel.content(ui, &store, true);
                panel.actions(ui, &mut simulation, &mut store, true);
            })
            .shapes
            .into_iter()
            .filter_map(|s| match s.shape {
                egui::Shape::Text(t) => {
                    Some((t.galley.text().to_owned(), t.pos + egui::vec2(2., 4.)))
                }
                _ => None,
            })
            .collect::<BTreeMap<_, _>>()
        };
        let mut labels = draw(vec![]);
        assert!(labels.contains_key(f.description()));
        for label in [f.title(), crate::language::text("Valider")] {
            let pos = labels[label];
            for pressed in [true, false] {
                labels = draw(vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        pressed,
                        button: egui::PointerButton::Primary,
                        modifiers: Default::default(),
                    },
                ]);
            }
        }
        labels = draw(vec![]);
        assert!(panel.review && simulation && store.privacy.is_empty());
        assert!(
            labels
                .keys()
                .any(|label| label.contains("P015") && label.contains(f.title()))
        );
        assert!(!labels.contains_key(crate::language::text("Confirmer et appliquer")));
    }

    #[test]
    fn backup_round_trip_preserves_missing_values_and_blocks_external_edits() {
        let f = CATALOG.iter().find(|f| f.id == "G001").unwrap();
        let before = vec![Some(Value::Number(1)), None, None];
        let backup = Backup {
            addresses: f.rules.iter().map(Rule::address).collect(),
            before: before.clone(),
            applied: true,
        };
        let mut store = Store::default();
        store.privacy.insert(f.key.clone(), backup);
        let reopened: Store = serde_json::from_slice(&serde_json::to_vec(&store).unwrap()).unwrap();
        let b = &reopened.privacy[&f.key];
        assert_eq!(original_values(f, b, &f.desired(true)).unwrap(), before);
        assert_eq!(original_values(f, b, &before).unwrap(), before);
        assert!(original_values(f, b, &vec![Some(Value::Number(99)); 3]).is_err());
        let mut bad = b.clone();
        bad.addresses.reverse();
        assert!(original_values(f, &bad, &before).is_err());
    }
    #[test]
    fn native_read_works_without_any_external_app() {
        let f = CATALOG.iter().find(|f| f.id == "P015").unwrap();
        let values = f.read().unwrap();
        assert_eq!(values.len(), 1);
        let tx = Transaction::new().unwrap();
        assert_eq!(f.rules[0].read(Some(&tx)).unwrap(), values[0]);
    }

    #[test]
    #[ignore = "requires permission to create a temporary HKCU test key"]
    fn native_transaction_restores_absent_and_existing_values_and_detects_conflicts() {
        const CHILD: &str = "FERRITE_NATIVE_REG_TEST";
        if std::env::var_os(CHILD).is_none() {
            let temp = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("privacy-test-{}", std::process::id()));
            std::fs::create_dir_all(&temp).unwrap();
            let result=std::process::Command::new(std::env::current_exe().unwrap()).args(["--ignored","--exact","app::privacy_controls::tests::native_transaction_restores_absent_and_existing_values_and_detects_conflicts"]).env(CHILD,"1").env("LOCALAPPDATA",&temp).output().unwrap();
            std::fs::remove_dir_all(temp).unwrap();
            assert!(
                result.status.success(),
                "{} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            return;
        }
        let path = format!(
            r"Software\Ferrite11Test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let (key, _) = HKCU.create_subkey(&path).unwrap();
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = HKCU.delete_subkey_all(&self.0);
            }
        }
        let _cleanup = Cleanup(path.clone());
        key.set_value("Existing", &7u32).unwrap();
        let mut f:Feature=serde_json::from_value(serde_json::json!({
            "id":"test","key":"test","group":"Privacy","scope":"user","rating":"Limited","fr":"","en":"","description_fr":"","description_en":"",
            "min":22000,"max":0,"edition":"All","rules":[]
        })).unwrap();
        for name in ["Existing", "Absent"] {
            f.rules.push(Rule {
                machine: false,
                path: path.clone(),
                name: name.into(),
                on: Value::Number(1),
                off: None,
            });
        }
        let initial = f.read().unwrap();
        let mut store = Store::default();
        execute(&f, &Choice::Set(true, initial.clone()), &mut store).unwrap();
        assert_eq!(f.read().unwrap(), f.desired(true));
        let mut reopened = super::super::load_store().unwrap();
        key.set_value("Existing", &9u32).unwrap();
        assert!(execute(&f, &Choice::Restore, &mut reopened).is_err());
        assert_eq!(key.get_value::<u32, _>("Existing").unwrap(), 9);
        key.set_value("Existing", &1u32).unwrap();
        execute(&f, &Choice::Restore, &mut reopened).unwrap();
        assert_eq!(f.read().unwrap(), initial);
        assert!(reopened.privacy.is_empty());
        assert!(
            !super::super::load_store()
                .unwrap()
                .privacy
                .contains_key("test")
        );
    }
}
