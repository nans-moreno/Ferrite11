# Ferrite11 Universal

Ferrite11 est une application de bureau en français pour Windows 11 x64. Elle analyse les réglages du compte local, explique leur effet et propose des changements réversibles. La version 0.5.0 est publiée sous licence MIT.

## Utilisation

1. Lance l’installateur Windows x64 ou ouvre la version portable. Ferme Ferrite11 avant une mise à niveau.
2. Depuis le tableau de bord, lance l’analyse des réglages. Le récapitulatif présente les actions proposées, dont l’affichage des extensions, les entrées de démarrage orphelines et les doublons du PATH utilisateur.
3. Retire les actions que tu souhaites conserver en l’état. Utilise le mode simulation pour examiner la sélection. Pour l’appliquer, désactive la simulation et confirme le récapitulatif.
4. Consulte le journal et les sauvegardes pour restaurer les réglages gérés. La restauration vérifie les conflits avec les modifications effectuées depuis.

Les rubriques sur l’interface, les mises à jour, la confidentialité, les pilotes NVIDIA, la latence et les services comprennent des étapes manuelles et des indications de retour arrière. La possibilité de revenir en arrière dépend de l’opération et des sauvegardes prises avant de la réaliser.

## Applications de remplacement

Micro ouvre les fichiers texte (.txt, .log, .md) et Oculante les images (.png, .jpg, .jpeg, .bmp, .webp) via les associations natives de Windows. Choisis « Installer et choisir les extensions », valide chaque association dans les paramètres Windows, puis actualise la liste. Le double-clic lance directement le programme choisi, même lorsque Ferrite11 est fermé. Pour revenir en arrière, rétablis l’application précédente dans Windows avant de retirer la copie portable. Les anciens outils à raccourcis peuvent être retirés depuis cette page.

## Compiler sous Windows

Prévois un Windows x64 avec la chaîne stable Rust MSVC x64, les outils de compilation C++ de Visual Studio, un SDK Windows et le compilateur C# du .NET Framework 4.x x64. Le premier téléchargement des dépendances Rust demande une connexion.

À la racine du dépôt, exécute dans PowerShell la commande suivante.

~~~powershell
.\build-windows.ps1
~~~

Le script utilise les versions verrouillées dans Cargo.lock, exécute les tests, construit en mode release et compile le programme d’installation C#. Dans dist, tu trouveras Ferrite11.exe, Ferrite11-Setup-0.5.0-x64.exe, la licence et les empreintes SHA-256. Les exécutables construits par cette procédure sont non signés.

Pour travailler uniquement sur le programme Rust, utilise cargo run ou cargo test --release --locked depuis la racine du dépôt. Le code de l’interface et des réglages se trouve dans src. Le fichier installer/Setup.cs contient le programme d’installation pour le compte utilisateur.

## Compilation GitHub Actions

Le workflow « Compiler Ferrite11 pour Windows x64 » s’exécute pour les changements sur main, les demandes d’intégration et les lancements manuels. Il exécute le même script sur un agent Windows. L’archive Ferrite11-Windows-x64 est disponible dans les artefacts du workflow terminé.
