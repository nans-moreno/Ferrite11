use super::{OptionGuide, Page};

const NVIDIA_3D: &str = "https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-us/mergedProjects/nv3d/Manage_3D_Settings_%28Reference%29.htm";
const NVIDIA_LATENCY: &str = "https://www.nvidia.com/en-us/geforce/guides/system-latency-optimization-guide/";
const INSPECTOR: &str = "https://github.com/Orbmu2k/nvidiaProfileInspector";
const TIMERS: &str = "https://learn.microsoft.com/en-us/windows/win32/api/timeapi/nf-timeapi-timebeginperiod";
const PARKCONTROL: &str = "https://bitsum.com/parkcontrol/";
const PARK_SOURCE: &str = "https://dl.bitsum.com/parkcontrol/";
const NVAPP: &str = "https://www.nvidia.com/en-us/software/nvidia-app/";

pub(super) const GUIDES: &[OptionGuide] = &[
    OptionGuide {
        id: "nv-output", page: Page::Nvidia, title: "Résolution native, fréquence et plage RVB 0–255",
        steps: "Dans le panneau NVIDIA, ouvre Affichage › Changer la résolution. Sélectionne l’écran, sa résolution native (rubrique PC si présente) et la fréquence la plus élevée qu’il prend en charge. Choisis RVB et plage complète 0–255 seulement si l’entrée de l’écran accepte aussi la plage complète. Vérifie les noirs et les blancs sur une mire.",
        undo: "Note les valeurs avant le test. Rétablis la fréquence et la plage précédentes sur le PC et sur l’écran si les noirs sont bouchés ou délavés.",
        note: "Un téléviseur peut attendre la plage limitée 16–235. Le câble, le port, le HDR et la profondeur de couleur peuvent limiter les combinaisons disponibles.",
        target: "ms-settings:display-advanced", source: "https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-us/mergedProjects/nvdsp/To_change_the_RGB_range.htm",
    },
    OptionGuide {
        id: "nv-vrr", page: Page::Nvidia, title: "G-SYNC, VRR et V-Sync pour l’eSport",
        steps: "Dans un jeu reproductible, compare deux essais à fréquence et charge identiques. Pour privilégier la latence, essaie V-Sync désactivée et relève le déchirement. Pour l’image sans déchirement sur écran compatible, teste G-SYNC avec la V-Sync et Reflex si le jeu le propose. Mesure la latence de bout en bout si ton équipement le permet.",
        undo: "Rétablis le mode G-SYNC de l’écran et les options V-Sync et Reflex du profil du jeu préalablement notées.",
        note: "Désactiver G-SYNC ou le VRR n’apporte pas systématiquement 1 à 1,5 ms. L’effet dépend du plafond de fréquence, de la file de rendu et de la charge.",
        target: NVIDIA_LATENCY, source: NVIDIA_LATENCY,
    },
    OptionGuide {
        id: "nv-scale", page: Page::Nvidia, title: "Mise à l’échelle de l’écran ou du GPU",
        steps: "Ouvre Régler la taille et la position du bureau. En résolution inférieure à la résolution native, compare « Aucune mise à l’échelle » et « Rapport largeur/hauteur ». Vérifie le cadrage et les bandes noires avant de retenir le mode adapté.",
        undo: "Rétablis le mode, le périphérique de mise à l’échelle et la case de remplacement des préférences du jeu à leurs valeurs initiales.",
        note: "À résolution native, l’image correspond déjà aux pixels de l’écran. Un gain de latence en supprimant le scaling reste à mesurer.",
        target: "ms-settings:display", source: "https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-us/mergedProjects/Display/Adjust_Desktop_Size_and_Position_-_Windows_Vista_and_Later.htm",
    },
    OptionGuide {
        id: "nv-counters", page: Page::Nvidia, title: "Accès développeur aux compteurs GPU",
        steps: "Si un profileur signale ERR_NVGPUCTRPERM, consulte Système › Avancé › Développeur dans NVIDIA App. Sur certaines versions, ouvre le panneau NVIDIA › Bureau › Paramètres développeur. Garde les compteurs réservés aux administrateurs sur un poste partagé.",
        undo: "Si un administrateur a temporairement accordé l’accès à tous les utilisateurs pour un profilage, il peut rétablir « Administrateurs uniquement » au même endroit.",
        note: "Ce droit sert à collecter des mesures matérielles. L’accorder à tous les comptes n’accélère pas le rendu du jeu.",
        target: NVAPP, source: "https://developer.nvidia.com/nvidia-development-tools-solutions-ERR_NVGPUCTRPERM-permission-issue-performance-counters",
    },
    OptionGuide {
        id: "nv-physx", page: Page::Nvidia, title: "Choix du processeur PhysX",
        steps: "Dans Configurer PhysX, relève le choix actuel. Pars de « Sélection automatique ». Si un ancien jeu utilise effectivement GPU PhysX et qu’un GPU compatible apparaît, teste le GPU dédié sur la même scène.",
        undo: "Rétablis le choix noté dans le sélecteur PhysX et relance le jeu.",
        note: "Le pilote recommande la sélection automatique. Ce choix s’applique aux calculs PhysX compatibles, pas à l’ensemble des opérations graphiques.",
        target: NVAPP, source: "https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-us/mergedProjects/3D%20Settings/Set_PhysX_configuration_%28reference%29.htm",
    },
    OptionGuide {
        id: "nv-quality", page: Page::Nvidia, title: "Optimisation threadée, filtrage des textures et alimentation",
        steps: "Ouvre Gérer les paramètres 3D › Paramètres de programme et choisis le jeu. Note ses trois valeurs initiales. Compare un seul changement à la fois sur trois passages : optimisation threadée Auto, filtrage des textures Haute performance, puis mode d’alimentation Privilégier les performances maximales. Observe qualité, frametimes, bruit et température.",
        undo: "Remets individuellement les trois valeurs initiales dans ce même profil, puis clique sur Appliquer.",
        note: "La qualité du filtrage peut baisser et la consommation augmenter. L’intérêt dépend du moteur, du pilote et de la limite CPU ou GPU.",
        target: NVAPP, source: NVIDIA_3D,
    },
    OptionGuide {
        id: "nv-inspector", page: Page::Nvidia, title: "Profile Inspector · sauvegarde du profil et retour arrière",
        steps: "Sur la page officielle, ouvre Releases et télécharge le ZIP stable. Extrais tout son contenu dans un dossier dédié. Avec un pilote NVIDIA compatible et .NET Framework 4.8, lance nvidiaProfileInspector.exe ; l’écriture des profils nécessite une élévation. Ferme le jeu, choisis son profil et exporte-le au format .nip dans Documents. Note ses réglages avant de tester une option.",
        undo: "Réinitialise les options ajoutées pour cet essai, puis réimporte le .nip sauvegardé et vérifie les valeurs. Conserve la sauvegarde après avoir retiré l’utilitaire.",
        note: "Pour retirer Inspector, ferme-le puis retire les fichiers extraits. Les profils résident dans le pilote : restaure-les avant de retirer l’utilitaire.",
        target: "https://github.com/Orbmu2k/nvidiaProfileInspector/releases/latest", source: INSPECTOR,
    },
    OptionGuide {
        id: "nv-inspector-sync", page: Page::Nvidia, title: "Inspector · G-SYNC, triple buffering, V-Sync et FXAA",
        steps: "Dans le profil du jeu, relève G-SYNC/VRR, Vertical Sync, Triple buffering et FXAA. Compare séparément V-Sync, l’anticrénelage FXAA et le réglage VRR aux valeurs du jeu. Pour Triple buffering du pilote, réalise le test sur une application OpenGL compatible.",
        undo: "Rétablis pour chaque option sa valeur initiale, y compris « contrôlé par l’application » si elle était héritée. Vérifie le profil exporté avant le test.",
        note: "Forcer FXAA peut lisser les contours et le texte. Désactiver ensemble quatre options masque celle qui influe sur l’image ou la latence.",
        target: INSPECTOR, source: NVIDIA_3D,
    },
    OptionGuide {
        id: "nv-inspector-queue", page: Page::Nvidia, title: "Maximum frames allowed = 1 et NVIDIA Reflex",
        steps: "Commence par activer Reflex dans le jeu s’il est pris en charge. Sinon, compare le mode Faible latence du pilote pour ce jeu. Mesure frametimes et latence d’entrée avant de choisir. Laisse « Maximum frames allowed » hérité lorsque le pilote n’en décrit pas l’usage.",
        undo: "Remets Reflex dans son état initial dans le jeu. Rétablis les paramètres de faible latence du profil et les options avancées modifiées.",
        note: "Reflex prend la priorité sur le mode Ultra du pilote. Une limite historique de 1 image dans Inspector ne garantit pas une file d’une image dans chaque API.",
        target: NVIDIA_LATENCY, source: NVIDIA_LATENCY,
    },
    OptionGuide {
        id: "nv-p2", page: Page::Nvidia, title: "Force P2 State et encodage NVENC",
        steps: "Pour diagnostiquer un encodage, compare le débit d’images encodées, les images perdues et la charge GPU dans deux captures identiques. Conserve la valeur héritée de « CUDA – Force P2 State » tant que le comportement sur ta carte et ta version de pilote n’est pas établi.",
        undo: "Si une valeur a été forcée pendant un essai, restaure la valeur héritée ou sauvegardée dans le profil concerné.",
        note: "Force P2 State est un réglage avancé. Un gain d’encodage NVENC par désactivation systématique n’est pas établi.",
        target: INSPECTOR, source: INSPECTOR,
    },
    OptionGuide {
        id: "nv-shell-power", page: Page::Nvidia, title: "Profils de puissance d’Explorer, DWM, Steam et ShellExperienceHost",
        steps: "Si les fréquences GPU restent élevées sur le bureau, ouvre la liste de profils et vérifie explorer.exe, dwm.exe, Steam et ShellExperienceHost.exe. Note leurs éventuelles dérogations au mode d’alimentation hérité. Teste les performances maximales sur le jeu concerné uniquement.",
        undo: "Remets chaque profil modifié dans son état sauvegardé. Vérifie consommation et fréquences au repos après fermeture du jeu.",
        note: "Forcer la puissance maximale pour des applications résidentes peut maintenir des fréquences élevées. Aucune hausse générale des FPS n’est établie pour ces profils.",
        target: INSPECTOR, source: NVIDIA_3D,
    },
    OptionGuide {
        id: "nv-hdcp", page: Page::Nvidia, title: "HDCP et clé RMHdcpKeyglobZero",
        steps: "Pour un problème de vidéo protégée, ouvre Affichage › Afficher l’état HDCP dans le panneau NVIDIA. Contrôle l’écran sélectionné, son port HDMI/DisplayPort et le câble, puis rouvre la page après un changement de branchement.",
        undo: "Le diagnostic en lecture seule se ferme directement. Si un ancien script a changé le registre, utilise sa sauvegarde d’origine pour revenir à son état antérieur.",
        note: "L’effet de RMHdcpKeyglobZero=1 sur la latence n’est pas documenté par la référence NVIDIA. Modifier la protection peut perturber les contenus protégés.",
        target: NVAPP, source: "https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-us/mergedProjects/nvdsp/View_HDCP_Status_Reference.htm",
    },
    OptionGuide {
        id: "lat-timers", page: Page::Latence, title: "GlobalTimerResolutionRequests et SetTimerResolution.exe",
        steps: "Avant de toucher aux minuteries, fais trois mesures du même parcours dans le jeu. Si SetTimerResolution.exe se lance déjà avec Windows, note sa commande et désactive temporairement son démarrage. Redémarre, répète le parcours et compare les temps d’image ainsi que la consommation.",
        undo: "Réactive l’entrée de démarrage d’origine si elle apportait un gain mesuré. Pour une clé déjà créée par un ancien script, réutilise la sauvegarde du script.",
        note: "Les valeurs 0,52 ms et 0,50 ms décrivent une résolution demandée. Aucune supériorité générale de 0,52 ms n’est établie. Windows fait évoluer le comportement par processus ; GlobalTimerResolutionRequests=1 n’est pas un réglage utilisateur documenté par cette API.",
        target: "ms-settings:startupapps", source: TIMERS,
    },
    OptionGuide {
        id: "lat-islc", page: Page::Latence, title: "ISLC · comparer avant de retirer l’utilitaire",
        steps: "Si Intelligent Standby List Cleaner est installé, note sa version, ses seuils et son démarrage automatique. Arrête-le depuis sa fenêtre et désactive provisoirement son lancement automatique. Mesure les saccades pendant une session comparable avant de décider de le conserver ou de le retirer.",
        undo: "Rouvre ISLC et rétablis les seuils et le mode de lancement notés. Si tu veux le retirer, conserve d’abord sa configuration et le lien de l’éditeur.",
        note: "ISLC agit sur la liste d’attente mémoire et propose des options de minuterie. Son utilité dépend de la machine et du problème observé. Sa prétendue obsolescence n’est pas un motif de désinstallation automatique.",
        target: "ms-settings:startupapps", source: "https://www.wagnardsoft.com/forums/viewtopic.php?t=1256",
    },
    OptionGuide {
        id: "lat-park", page: Page::Latence, title: "ParkControl et Bitsum Highest Performance",
        steps: "Note le mode d’alimentation actif et ses valeurs secteur/batterie dans ParkControl. Si le plan Bitsum Highest Performance est disponible, teste-le sur secteur dans une charge répétable. Compare les temps d’image, la température et la consommation au mode habituel.",
        undo: "Reviens au plan précédemment actif ; si ses paramètres ont été changés, rétablis les valeurs notées pour secteur et batterie.",
        note: "Le plan Bitsum favorise la disponibilité du processeur. Le compromis d’énergie varie entre portable et PC fixe. Un jeu peu sensible au processeur peut donner le même résultat.",
        target: PARKCONTROL, source: PARK_SOURCE,
    },
    OptionGuide {
        id: "lat-park-start", page: Page::Latence, title: "ParkControl au démarrage et Dynamic Boost",
        steps: "Vérifie si tu utilises Dynamic Boost, les notifications ou la surveillance en direct. Pour conserver uniquement les réglages statiques du plan, désactive le lancement de ParkControl dans ses préférences ou dans Applications de démarrage. Redémarre et vérifie le plan actif.",
        undo: "Réactive le démarrage de ParkControl puis ouvre-le pour retrouver Dynamic Boost et les fonctions résidentes.",
        note: "Les paramètres statiques du plan persistent après fermeture. Les fonctions dynamiques nécessitent que l’utilitaire reste en cours d’exécution.",
        target: "ms-settings:startupapps", source: PARK_SOURCE,
    },
    OptionGuide {
        id: "lat-perfcheck", page: Page::Latence, title: "PerformanceTimeCheckInterval · intervalle PERFCHECK",
        steps: "Dans Terminal, consulte le plan actif avec powercfg /getactivescheme. Lis ensuite powercfg /qh SCHEME_CURRENT SUB_PROCESSOR et cherche PERFCHECK. Note les intervalles secteur et batterie en millisecondes. Garde les valeurs du plan pendant la mesure de référence.",
        undo: "Si tu compares un autre plan, réactive le plan initial. Si un réglage a déjà été modifié, restaure dans le plan concerné les deux valeurs secteur et batterie sauvegardées.",
        note: "Cet intervalle concerne l’évaluation des états de performance CPU. L’augmenter espace les contrôles et peut retarder certaines adaptations. Son effet dépend du processeur, de son pilotage et du plan.",
        target: "ms-settings:powersleep", source: "https://learn.microsoft.com/en-us/windows-hardware/customize/power-settings/options-for-perf-state-engine-perfincreasetime",
    },
];
