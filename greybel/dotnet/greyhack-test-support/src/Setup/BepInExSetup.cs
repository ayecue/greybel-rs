using GreyHackTestSupport.Game;

namespace GreyHackTestSupport.Setup;

/// <summary>
/// Installs greyhack-setup's BepInEx release for the selected major into the clone, replacing whichever version the
/// installed game contains. Downloads are cached in the work folder.
/// </summary>
internal static class BepInExSetup
{
    public static void Ensure(WorkFolder folder, GameLayout game, BepInExMajor major) =>
        SetupTool.EnsureBepInEx(game, major, folder.Downloads);
}
