using System.Diagnostics;
using GreyHackSetupWizard.Game;

namespace GreyHackSetupWizard.BepInEx;

public sealed record BepInExInstallation(BepInExMajor Major, string Version)
{
    public static BepInExInstallation? Detect(GameLayout game)
    {
        string core = Path.Combine(game.BepInExFolder, "core");
        string betaCore = Path.Combine(core, "BepInEx.Unity.Mono.dll");
        if (File.Exists(betaCore))
        {
            return new BepInExInstallation(
                BepInExMajor.Beta,
                GetVersion(Path.Combine(core, "BepInEx.Core.dll"))
            );
        }
        string stableCore = Path.Combine(core, "BepInEx.dll");
        if (File.Exists(stableCore))
        {
            return new BepInExInstallation(BepInExMajor.Stable, GetVersion(stableCore));
        }
        return null;
    }

    public string GetDescription() =>
        Version.Length == 0 ? $"BepInEx {(int)Major}" : $"BepInEx {(int)Major} ({Version})";

    private static string GetVersion(string assembly)
    {
        if (!File.Exists(assembly))
        {
            return "";
        }
        string version = FileVersionInfo.GetVersionInfo(assembly).ProductVersion ?? "";
        return version.Split('+')[0];
    }
}
