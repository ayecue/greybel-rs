using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Plugins;

namespace GreyHackSetupWizard.Setup;

public sealed record GameStatus(
    GameLayout Game,
    BepInExInstallation? BepInEx,
    IReadOnlyList<string> PluginAssemblies,
    int? Port
)
{
    public static GameStatus Read(GameLayout game) =>
        new(
            game,
            BepInExInstallation.Detect(game),
            PluginInstaller.GetAssemblies(game),
            PluginConfig.GetPort(game)
        );

    public string? FindMessageHook() =>
        PluginAssemblies.FirstOrDefault(PluginBuild.MessageHook.Assemblies.Contains);
}
