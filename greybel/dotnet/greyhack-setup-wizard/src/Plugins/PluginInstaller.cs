using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.Plugins;

public static class PluginInstaller
{
    public static void Install(
        GameLayout game,
        PluginBuild plugin,
        BepInExMajor major,
        string assemblyPath
    )
    {
        string name = plugin.GetAssemblyName(major);
        if (
            !string.Equals(
                Path.GetFileNameWithoutExtension(assemblyPath),
                name,
                StringComparison.Ordinal
            )
        )
        {
            throw new SetupException(
                Localization.Format(Messages.PluginFileWrong, assemblyPath, (int)major, name)
            );
        }
        Remove(game, plugin);
        Directory.CreateDirectory(game.PluginsFolder);
        File.Copy(assemblyPath, Path.Combine(game.PluginsFolder, name + ".dll"), overwrite: true);
        string symbols = Path.ChangeExtension(assemblyPath, ".pdb");
        if (File.Exists(symbols))
        {
            File.Copy(symbols, Path.Combine(game.PluginsFolder, name + ".pdb"), overwrite: true);
        }
    }

    public static void Remove(GameLayout game, PluginBuild plugin)
    {
        if (!Directory.Exists(game.PluginsFolder))
        {
            return;
        }
        foreach (string name in plugin.Assemblies)
        {
            foreach (string extension in new[] { ".dll", ".pdb" })
            {
                File.Delete(Path.Combine(game.PluginsFolder, name + extension));
            }
        }
    }

    public static IReadOnlyList<string> GetAssemblies(GameLayout game) =>
        Directory.Exists(game.PluginsFolder)
            ? Directory
                .EnumerateFiles(game.PluginsFolder, "*.dll", SearchOption.AllDirectories)
                .Select(Path.GetFileNameWithoutExtension)
                .OfType<string>()
                .Order()
                .ToList()
            : [];
}
