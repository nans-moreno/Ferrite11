using System;
using System.IO;
using System.Reflection;
using System.Windows.Forms;

[assembly: AssemblyTitle("Installation de Ferrite11 Universal")]
[assembly: AssemblyProduct("Ferrite11 Universal")]
[assembly: AssemblyVersion("0.5.0.0")]

internal static class Setup
{
    [STAThread]
    static int Main(string[] args)
    {
        bool quiet = args.Length == 2 && args[0] == "/install";
        try
        {
            string folder = quiet ? Path.GetFullPath(args[1]) : Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Programs", "Ferrite11");
            if (!quiet && MessageBox.Show("Installer Ferrite11 Universal 0.5.0 dans " + folder + " ?",
                "Ferrite11", MessageBoxButtons.OKCancel) != DialogResult.OK) return 1;
            Directory.CreateDirectory(folder);
            foreach (string name in new[] { "LICENSE.txt", "Ferrite11.exe" })
            {
                string destination = Path.Combine(folder, name);
                string temp = Path.Combine(folder, Guid.NewGuid().ToString() + ".tmp");
                try
                {
                    using (Stream input = Assembly.GetExecutingAssembly().GetManifestResourceStream(name))
                    using (Stream output = File.Create(temp)) input.CopyTo(output);
                    if (File.Exists(destination)) File.Replace(temp, destination, null);
                    else File.Move(temp, destination);
                }
                finally { if (File.Exists(temp)) File.Delete(temp); }
            }
            if (!quiet && MessageBox.Show("Installation terminée. Ouvrir Ferrite11 ?",
                "Ferrite11", MessageBoxButtons.YesNo) == DialogResult.Yes)
                System.Diagnostics.Process.Start(Path.Combine(folder, "Ferrite11.exe"));
            return 0;
        }
        catch (Exception error)
        {
            if (!quiet) MessageBox.Show(error.Message, "Installation de Ferrite11");
            return 2;
        }
    }
}
