using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.Game;

public sealed class GameLayout
{
    private GameLayout(string folder, string executable, string dataFolder)
    {
        Folder = folder;
        Executable = executable;
        DataFolder = dataFolder;
    }

    public string Folder { get; }

    public string Executable { get; }

    public string DataFolder { get; }

    public string ManagedFolder => Path.Combine(DataFolder, "Managed");

    public string BepInExFolder => Path.Combine(Folder, "BepInEx");

    public string PluginsFolder => Path.Combine(BepInExFolder, "plugins");

    public string ConfigFolder => Path.Combine(BepInExFolder, "config");

    public static GameLayout FromFolder(string folder)
    {
        string app = Path.Combine(folder, "Grey Hack.app");
        if (Directory.Exists(app))
        {
            return new GameLayout(folder, app, Path.Combine(app, "Contents", "Resources", "Data"));
        }
        string exe = Path.Combine(folder, "Grey Hack.exe");
        if (File.Exists(exe))
        {
            return new GameLayout(folder, exe, Path.Combine(folder, "Grey Hack_Data"));
        }
        string? binary = Directory.Exists(folder)
            ? Directory.EnumerateFiles(folder, "*.x86_64").FirstOrDefault()
            : null;
        if (binary != null)
        {
            return new GameLayout(
                folder,
                binary,
                Path.Combine(folder, Path.GetFileNameWithoutExtension(binary) + "_Data")
            );
        }
        throw new SetupException(Localization.Format(Messages.GameMissingInFolder, folder));
    }
}
