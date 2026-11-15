using System;
using System.Drawing;
using System.IO;
using System.Reflection;
using System.Windows.Forms;

[assembly: AssemblyTitle("Ferrite11 Setup")]
[assembly: AssemblyProduct("Ferrite11 Universal")]
[assembly: AssemblyVersion("0.9.0.0")]

internal static class Setup
{
    static string Preferences {
        get { return Path.Combine(Environment.GetEnvironmentVariable("LOCALAPPDATA") ?? Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Ferrite11"); }
    }

    static string ReadPreference(string name)
    {
        string file = Path.Combine(Preferences, name);
        return File.Exists(file) ? File.ReadAllText(file).Trim() : null;
    }

    static void SavePreference(string name, string value)
    {
        Directory.CreateDirectory(Preferences);
        string file = Path.Combine(Preferences, name);
        string temp = Path.Combine(Preferences, Guid.NewGuid().ToString("N") + ".tmp");
        try {
            File.WriteAllText(temp, value);
            if (File.Exists(file)) File.Replace(temp, file, null);
            else File.Move(temp, file);
        } finally { if (File.Exists(temp)) File.Delete(temp); }
    }

    static bool Choose(ref string destination, ref bool english)
    {
        using (Form form = new Form())
        using (FolderBrowserDialog picker = new FolderBrowserDialog())
        {
            form.ClientSize = new Size(550, 165);
            form.AutoScaleMode = AutoScaleMode.Font;
            form.StartPosition = FormStartPosition.CenterScreen;
            form.FormBorderStyle = FormBorderStyle.FixedDialog;
            form.MaximizeBox = false;
            form.MinimizeBox = false;
            Label language = new Label { Left = 16, Top = 20, AutoSize = true, Text = "Langue / Language" };
            ComboBox languages = new ComboBox { Left = 155, Top = 16, Width = 155, DropDownStyle = ComboBoxStyle.DropDownList };
            languages.Items.AddRange(new object[] { "Français", "English" });
            Label location = new Label { Left = 16, Top = 57, AutoSize = true };
            TextBox folder = new TextBox { Left = 16, Top = 80, Width = 408, Text = destination };
            Button browse = new Button { Left = 430, Top = 78, Width = 104, Height = 26 };
            Button install = new Button { Left = 325, Top = 123, Width = 100, Height = 27 };
            Button cancel = new Button { Left = 434, Top = 123, Width = 100, Height = 27, DialogResult = DialogResult.Cancel };
            languages.SelectedIndexChanged += delegate {
                bool en = languages.SelectedIndex == 1;
                form.Text = en ? "Install Ferrite11 0.9.0" : "Installer Ferrite11 0.9.0";
                location.Text = en ? "Installation folder" : "Dossier d’installation";
                browse.Text = en ? "Browse…" : "Parcourir…";
                install.Text = en ? "Install" : "Installer";
                cancel.Text = en ? "Cancel" : "Annuler";
                picker.Description = en ? "Choose the Ferrite11 installation folder" : "Choisis le dossier d’installation de Ferrite11";
            };
            browse.Click += delegate {
                if (Directory.Exists(folder.Text)) picker.SelectedPath = folder.Text;
                if (picker.ShowDialog(form) == DialogResult.OK) folder.Text = picker.SelectedPath;
            };
            install.Click += delegate {
                try {
                    string path = Environment.ExpandEnvironmentVariables(folder.Text.Trim());
                    if (!(path.StartsWith(@"\\") || (path.Length >= 3 && char.IsLetter(path[0]) && path[1] == ':' && (path[2] == '\\' || path[2] == '/'))))
                        throw new IOException(languages.SelectedIndex == 1 ? "Enter a full folder path." : "Saisis le chemin complet du dossier.");
                    folder.Text = Path.GetFullPath(path);
                    form.DialogResult = DialogResult.OK;
                } catch (Exception e) { MessageBox.Show(form, e.Message, "Ferrite11"); }
            };
            form.Controls.AddRange(new Control[] { language, languages, location, folder, browse, install, cancel });
            form.AcceptButton = install;
            form.CancelButton = cancel;
            languages.SelectedIndex = english ? 1 : 0;
            if (form.ShowDialog() != DialogResult.OK) return false;
            destination = folder.Text;
            english = languages.SelectedIndex == 1;
            return true;
        }
    }

    [STAThread]
    static int Main(string[] args)
    {
        bool quiet = args.Length == 2 && args[0] == "/install";
        bool english = false;
        try {
            string folder = quiet ? Path.GetFullPath(args[1]) : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "Ferrite11");
            if (!quiet) {
                Application.EnableVisualStyles();
                english = ReadPreference("language.txt") == "en";
                string previous = ReadPreference("install-dir.txt");
                if (!String.IsNullOrWhiteSpace(previous)) folder = previous;
                if (!Choose(ref folder, ref english)) return 1;
            }
            Directory.CreateDirectory(folder);
            foreach (string name in new[] { "LICENSE.txt", "Ferrite11.exe" }) {
                string destination = Path.Combine(folder, name);
                if (name == "LICENSE.txt" && File.Exists(destination) && !File.Exists(Path.Combine(folder, "Ferrite11.exe")))
                    destination = Path.Combine(folder, "Ferrite11-LICENSE.txt");
                string temp = Path.Combine(folder, Guid.NewGuid().ToString() + ".tmp");
                try {
                    using (Stream input = Assembly.GetExecutingAssembly().GetManifestResourceStream(name))
                    using (Stream output = File.Create(temp)) input.CopyTo(output);
                    if (File.Exists(destination)) File.Replace(temp, destination, null);
                    else File.Move(temp, destination);
                } finally { if (File.Exists(temp)) File.Delete(temp); }
            }
            if (!quiet) {
                try {
                    SavePreference("language.txt", english ? "en" : "fr");
                    SavePreference("install-dir.txt", folder);
                } catch (IOException e) {
                    MessageBox.Show((english ? "Installed. Could not save preferences: " : "Installation terminée. Préférences non enregistrées : ") + e.Message, "Ferrite11");
                } catch (UnauthorizedAccessException e) {
                    MessageBox.Show((english ? "Installed. Could not save preferences: " : "Installation terminée. Préférences non enregistrées : ") + e.Message, "Ferrite11");
                }
                if (MessageBox.Show(english ? "Installation complete. Open Ferrite11?" : "Installation terminée. Ouvrir Ferrite11 ?", "Ferrite11", MessageBoxButtons.YesNo) == DialogResult.Yes)
                    System.Diagnostics.Process.Start(Path.Combine(folder, "Ferrite11.exe"));
            }
            return 0;
        } catch (Exception error) {
            if (!quiet) MessageBox.Show(error.Message, "Ferrite11");
            return 2;
        }
    }
}
