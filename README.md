# Ferrite11 Universal

Ferrite11 est une application de bureau en français pour Windows 11 x64. Elle analyse les réglages du compte local, explique leur effet et propose des changements réversibles. La version 0.4.0 est publiée sous licence MIT.

## Utilisation

1. Lance l’installateur Windows x64 ou ouvre la version portable. Ferme Ferrite11 avant une mise à niveau.
2. Depuis le tableau de bord, lance l’analyse des réglages. Le récapitulatif présente les actions proposées, dont l’affichage des extensions, les entrées de démarrage orphelines et les doublons du PATH utilisateur.
3. Retire les actions que tu souhaites conserver en l’état. Utilise le mode simulation pour examiner la sélection. Pour l’appliquer, désactive la simulation et confirme le récapitulatif.
4. Consulte le journal et les sauvegardes pour restaurer les réglages gérés. La restauration vérifie les conflits avec les modifications effectuées depuis.

Les rubriques sur l’interface, les mises à jour, la confidentialité, les pilotes NVIDIA, la latence et les services comprennent des étapes manuelles et des indications de retour arrière. La possibilité de revenir en arrière dépend de l’opération et des sauvegardes prises avant de la réaliser.

## Applications de remplacement

Ferrite11 peut télécharger des versions portables de Yazi, Micro, Oculante, bottom, ripgrep et fd. Les téléchargements nécessitent une connexion et leur empreinte SHA-256 est vérifiée. Les lanceurs de Ferrite11 ouvrent l’alternative choisie et permettent de revenir à l’outil précédent. Les associations globales de fichiers et les applications déjà installées restent gérées par Windows.

Les sauvegardes locales se trouvent sous %LOCALAPPDATA%\Ferrite11 et les copies d’outils sous son sous-dossier outils. Utilise le bouton de retour arrière dans la rubrique des applications avant de supprimer Ferrite11 si tu souhaites rétablir ses lanceurs.

## Compiler sous Windows

Prévois un Windows x64 avec la chaîne stable Rust MSVC x64, les outils de compilation C++ de Visual Studio, un SDK Windows et le compilateur C# du .NET Framework 4.x x64. Le premier téléchargement des dépendances Rust demande une connexion.

À la racine du dépôt, exécute dans PowerShell la commande suivante.

~~~powershell
.\build-windows.ps1
~~~

Le script utilise les versions verrouillées dans Cargo.lock, exécute les tests, construit en mode release et compile le programme d’installation C#. Dans dist, tu trouveras Ferrite11.exe, Ferrite11-Setup-0.4.0-x64.exe, la licence et les empreintes SHA-256. Les exécutables construits par cette procédure sont non signés.

Pour travailler uniquement sur le programme Rust, utilise cargo run ou cargo test --release --locked depuis la racine du dépôt. Le code de l’interface et des réglages se trouve dans src. Le fichier installer/Setup.cs contient le programme d’installation pour le compte utilisateur.

## Compilation GitHub Actions

Le workflow « Compiler Ferrite11 pour Windows x64 » s’exécute pour les changements sur main, les demandes d’intégration et les lancements manuels. Il exécute le même script sur un agent Windows. L’archive Ferrite11-Windows-x64 est disponible dans les artefacts du workflow terminé.
