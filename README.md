# Ferrite11 — Optimisation et confidentialité pour Windows 11

Ferrite11 aide à alléger le démarrage de Windows 11 x64 et à choisir quelles données les applications peuvent utiliser. Ses 346 fiches de confidentialité décrivent l’effet de chaque option en français et en anglais. Tu peux examiner ta sélection en mode simulation, appliquer les changements dans Ferrite11 et restaurer les valeurs sauvegardées.

L’installateur permet de sélectionner la langue et de saisir ou parcourir le dossier de destination. Dans l’application, **Français** et **English** changent la langue immédiatement. Le choix est mémorisé pour le compte Windows.

Le bouton **Mettre à jour / Update** vérifie la dernière version stable publiée dans les Releases GitHub. Si elle est plus récente, Ferrite11 télécharge son installateur x64, vérifie son empreinte SHA-256, puis redémarre depuis le même emplacement. Les préférences, le journal et les sauvegardes sont conservés.

La page **Contrôles de confidentialité** regroupe les réglages par thème. Chaque fiche décrit en français et en anglais ce qui change pour l’utilisateur. Ferrite11 consulte les valeurs Windows et sauvegarde leur état initial avant d’appliquer une sélection. Le mode simulation affiche le récapitulatif. Pour revenir en arrière, coche une fiche marquée comme sauvegardée et valide, ou utilise la restauration générale. Les contrôles guidés indiquent la page Windows appropriée. Pour les stratégies du PC, ouvre Ferrite11 en tant qu’administrateur si Windows refuse l’écriture. La configuration enregistrée peut être supplantée par une stratégie d’entreprise ou ignorée par un composant absent ou obsolète.

## Utilisation

1. Lance l’installateur Windows x64 ou ouvre la version portable. Ferme Ferrite11 avant une mise à niveau.
2. Depuis le tableau de bord, lance l’analyse des réglages. Le récapitulatif présente les actions proposées, dont l’affichage des extensions, les entrées de démarrage orphelines et les doublons du PATH utilisateur.
3. Retire les actions que tu souhaites conserver en l’état. Utilise le mode simulation pour examiner la sélection. Pour l’appliquer, désactive la simulation et confirme le récapitulatif.
4. Consulte le journal et les sauvegardes pour restaurer les réglages gérés. La restauration vérifie les conflits avec les modifications effectuées depuis.

Les rubriques sur l’interface, les mises à jour, la confidentialité, les pilotes NVIDIA, la latence et les services comprennent des étapes manuelles et des indications de retour arrière. La possibilité de revenir en arrière dépend de l’opération et des sauvegardes prises avant de la réaliser.

## Applications de remplacement

Kwil (bêta) ouvre les fichiers texte (.txt, .log, .md), PdfCraft les documents PDF et Oculante les images (.png, .jpg, .jpeg, .bmp, .webp) via les associations natives de Windows. Choisis « Installer et choisir les extensions », valide chaque association dans les paramètres Windows, puis actualise la liste. Le double-clic lance directement le programme choisi, même lorsque Ferrite11 est fermé. Pour revenir en arrière, rétablis l’application précédente dans Windows avant de retirer la copie portable. Les anciens outils à raccourcis peuvent être retirés depuis cette page.

ArchiveRclick peut ouvrir les archives ZIP et 7z au double-clic. LeanRows affiche en lecture seule les tableaux CSV et TSV et les données JSONL et NDJSON. Kitab peut être choisi pour les livres EPUB, FB2 et MOBI ainsi que les bandes dessinées CBZ. Chaque format se choisit et se rétablit individuellement.

FastTask 1.1.0 peut prendre en charge Ctrl + Maj + Échap pour tous les utilisateurs. Depuis Applications de remplacement, choisis « Remplacer par FastTask », valide et accepte la demande de Windows. Utilise « Rétablir le Gestionnaire des tâches Windows » avant de désinstaller FastTask.

## Compiler sous Windows

Prévois un Windows x64 avec la chaîne stable Rust MSVC x64, les outils de compilation C++ de Visual Studio, un SDK Windows et le compilateur C# du .NET Framework 4.x x64. Le premier téléchargement des dépendances Rust demande une connexion.

À la racine du dépôt, exécute dans PowerShell la commande suivante.

~~~powershell
.\build-windows.ps1
~~~

Le script utilise les versions verrouillées dans Cargo.lock, exécute les tests, construit en mode release et compile le programme d’installation C#. Dans dist, tu trouveras Ferrite11.exe, Ferrite11-Setup-0.12.1-x64.exe, la licence et les empreintes SHA-256. Les exécutables construits par cette procédure sont non signés.

Pour travailler uniquement sur le programme Rust, utilise cargo run ou cargo test --release --locked depuis la racine du dépôt. Le code de l’interface et des réglages se trouve dans src. Le fichier installer/Setup.cs contient le programme d’installation pour le compte utilisateur.

## Compilation GitHub Actions

Le workflow « Compiler Ferrite11 pour Windows x64 » s’exécute pour les changements sur main, les demandes d’intégration et les lancements manuels. Il exécute le même script sur un agent Windows. L’archive Ferrite11-Windows-x64 est disponible dans les artefacts du workflow terminé.

Pour distribuer une nouvelle version, augmente le numéro de version du projet et de l’installateur sur main. Après les tests, GitHub publie l’installateur x64 dans une version stable et vérifie son empreinte avant de la rendre visible au bouton **Mettre à jour**.
