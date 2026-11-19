using GreyHackTestSupport.Game;
using GreyHackTestSupport.Platform;

namespace GreyHackTestSupport.Setup;

/// <summary>
/// Defines test-game plugins and their projects relative to <c>greybel/dotnet</c>. Each plugin is built with the
/// BepInEx major's logging configuration, <c>Debug-BepInEx5</c> or <c>Debug-BepInEx6</c>, and installed as the
/// clone's complete plugin set.
/// </summary>
internal static class PluginBuildTable
{
    public static readonly PluginBuild TestSupport = new(
        "GreyHackTestSupport",
        "GreyHackTestSupport",
        "GreyHackTestSupport"
    );

    public static readonly (PluginBuild Plugin, string Project)[] Plugins =
    [
        (PluginBuild.MessageHook, "greyhack-message-hook/src"),
        (TestSupport, "greyhack-test-support/plugin"),
    ];

    public static void Install(WorkFolder folder, GameLayout game, BepInExMajor major)
    {
        if (Directory.Exists(game.PluginsFolder))
        {
            Directory.Delete(game.PluginsFolder, recursive: true);
        }
        foreach (var (plugin, project) in Plugins)
        {
            string assembly = plugin.GetAssemblyName(major);
            string output = Path.Combine(folder.PluginBuilds, assembly);
            Build(Path.Combine(Repository.Dotnet, project), $"Debug-BepInEx{(int)major}", output);
            Install(game, plugin, major, Path.Combine(output, assembly + ".dll"));
        }
    }

    /// <summary>
    /// Installs the message hook with greyhack-setup, and other plugins by copying the assembly and its symbols.
    /// </summary>
    private static void Install(
        GameLayout game,
        PluginBuild plugin,
        BepInExMajor major,
        string assembly
    )
    {
        if (plugin == PluginBuild.MessageHook)
        {
            SetupTool.InstallHook(game, major, assembly);
            return;
        }
        Directory.CreateDirectory(game.PluginsFolder);
        foreach (string file in new[] { assembly, Path.ChangeExtension(assembly, ".pdb") })
        {
            if (File.Exists(file))
            {
                File.Copy(
                    file,
                    Path.Combine(game.PluginsFolder, Path.GetFileName(file)),
                    overwrite: true
                );
            }
        }
    }

    private static void Build(string project, string configuration, string output)
    {
        ExternalProcess build = ExternalProcess.Run(
            "dotnet",
            "build",
            "--nologo",
            "-v",
            "q",
            "--configuration",
            configuration,
            "-o",
            output,
            project
        );
        if (!build.Succeeded)
        {
            throw new TestSupportException($"dotnet build of {project} failed:\n{build.Output}");
        }
    }
}
