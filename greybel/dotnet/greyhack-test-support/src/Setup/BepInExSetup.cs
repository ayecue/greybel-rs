using GreyHackSetupWizard;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Downloads;
using GreyHackSetupWizard.Game;
using GreyHackTestSupport.Game;

namespace GreyHackTestSupport.Setup;

/// <summary>
/// Installs the setup wizard's BepInEx release for the selected major into the clone, replacing whichever version
/// the installed game contains. Downloads are cached in the work folder.
/// </summary>
internal static class BepInExSetup
{
    public static void Ensure(WorkFolder folder, GameLayout game, BepInExMajor major)
    {
        BepInExRelease release = BepInExRelease.Get(major, PlatformDetector.Current);
        if (BepInExInstaller.HasRelease(game, release))
        {
            return;
        }
        using var http = new HttpClient();
        string archive = Download
            .FetchAsync(http, release.Url, release.FileName, release.Sha256, folder.Downloads)
            .GetAwaiter()
            .GetResult();
        BepInExInstaller.Install(game, archive);
    }
}
