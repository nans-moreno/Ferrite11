pub(super) struct Topic {
    pub fr: &'static str,
    pub en: &'static str,
    pub ids: &'static [&'static str],
}
pub(super) const TOPICS: &[Topic] = &[
    Topic {
        fr: "Effets visuels et écran",
        en: "Visual effects and display",
        ids: &[
            "transparency",
            "animations",
            "visual",
            "custom-visual-effects",
            "night",
        ],
    },
    Topic {
        fr: "Barre des tâches et notifications",
        en: "Taskbar and notifications",
        ids: &["taskbar", "widgets", "notifications", "dragdrop-taskbar"],
    },
    Topic {
        fr: "Explorateur et souris",
        en: "File Explorer and mouse",
        ids: &[
            "file-extensions",
            "classic-context",
            "pointer-precision",
            "folder-tip-size",
        ],
    },
    Topic {
        fr: "Présentation et zoom",
        en: "Presentation and zoom",
        ids: &["zoomit-demo"],
    },
    Topic {
        fr: "Calendrier et report",
        en: "Scheduling and deferral",
        ids: &[
            "updates-pause-test",
            "updates-security",
            "update-blocker-review",
            "activehours",
        ],
    },
    Topic {
        fr: "Téléchargements et partage",
        en: "Downloads and sharing",
        ids: &["delivery-peer-sharing", "offline-map-updates"],
    },
    Topic {
        fr: "Suivi et autorisations",
        en: "Tracking and permissions",
        ids: &[
            "app-launch-tracking",
            "website-language-list",
            "privacy-general",
            "privacy-search-permissions",
        ],
    },
    Topic {
        fr: "Diagnostics et historique",
        en: "Diagnostics and history",
        ids: &[
            "privacy-diagnostics",
            "privacy-activity-history",
            "diagnostic-options",
        ],
    },
    Topic {
        fr: "Connexion et synchronisation",
        en: "Sign-in and sync",
        ids: &["accounts-after-update", "accounts-sync-preferences"],
    },
    Topic {
        fr: "Mode Jeu et captures Xbox",
        en: "Game Mode and Xbox captures",
        ids: &[
            "capture",
            "gamebar-controller",
            "game-mode",
            "gamemode",
            "gamedvr-ui",
            "game-bar-shortcuts",
        ],
    },
    Topic {
        fr: "Écrans et rendu graphique",
        en: "Displays and graphics",
        ids: &["refresh", "graphics", "nv-output", "nv-vrr", "nv-scale"],
    },
    Topic {
        fr: "Pilotes et mesures en jeu",
        en: "Drivers and game benchmarks",
        ids: &["nvidia", "radeon", "intelgpu", "measure"],
    },
    Topic {
        fr: "Profils 3D et synchronisation",
        en: "3D profiles and sync",
        ids: &[
            "nv-physx",
            "nv-quality",
            "nv-inspector",
            "nv-inspector-sync",
            "nv-inspector-queue",
            "nv-p2",
            "nv-shell-power",
        ],
    },
    Topic {
        fr: "Compteurs et protection vidéo",
        en: "Counters and video protection",
        ids: &["nv-counters", "nv-hdcp"],
    },
    Topic {
        fr: "Ouverture de session",
        en: "Sign-in and startup",
        ids: &[
            "orphan-startup",
            "startup",
            "autoruns-review",
            "restore-folder-windows",
        ],
    },
    Topic {
        fr: "Applications installées et par défaut",
        en: "Installed and default apps",
        ids: &["installed", "edge-review", "store-recovery", "defaults"],
    },
    Topic {
        fr: "Stockage et recherche",
        en: "Storage and search",
        ids: &["temp-review", "search", "storage"],
    },
    Topic {
        fr: "Composants et environnement Windows",
        en: "Windows components and environment",
        ids: &[
            "path-duplicates",
            "optional",
            "windows-activation-fullscreen",
            "autoplay-media",
        ],
    },
    Topic {
        fr: "Alimentation, USB et veille",
        en: "Power, USB and sleep",
        ids: &[
            "balanced",
            "power-performance",
            "usb-suspend",
            "hibernate-review",
            "sleep",
        ],
    },
    Topic {
        fr: "Mémoire et bus PCIe",
        en: "Memory and PCIe",
        ids: &["rebar", "ram"],
    },
    Topic {
        fr: "Microprogramme et chipset",
        en: "Firmware and chipset",
        ids: &["bios", "chipset-amd", "chipset-intel"],
    },
    Topic {
        fr: "Services Windows",
        en: "Windows services",
        ids: &["svchost-grouping", "manual-services"],
    },
    Topic {
        fr: "Profils et diagnostics",
        en: "Profiles and diagnostics",
        ids: &["winutil-profile"],
    },
    Topic {
        fr: "Minuteries et mémoire",
        en: "Timers and memory",
        ids: &["lat-timers", "lat-islc"],
    },
    Topic {
        fr: "Profils d’alimentation",
        en: "Power profiles",
        ids: &["lat-park", "lat-park-start", "lat-perfcheck"],
    },
];
