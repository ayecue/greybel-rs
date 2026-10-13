using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Downloads;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.Setup;

public static class BepInExSetup
{
    public static BepInExRelease GetRelease(BepInExMajor major) =>
        BepInExRelease.Get(major, PlatformDetector.Current);

    public static async Task<BepInExInstallation> InstallAsync(
        GameLayout game,
        BepInExMajor major,
        string? archive,
        DownloadSettings download,
        IProgress<string> progress
    )
    {
        if (archive == null)
        {
            BepInExRelease release = GetRelease(major);
            progress.Report(
                Localization.Format(Messages.ProgressBepInExDownload, release.Version, release.Url)
            );
            archive = await Download.FetchAsync(
                download.Http,
                release.Url,
                release.FileName,
                release.Sha256,
                download.CacheFolder
            );
        }
        progress.Report(Localization.Format(Messages.ProgressBepInExInstall, archive));
        BepInExInstallation installation = BepInExInstaller.Install(game, archive);
        progress.Report(
            Localization.Format(
                Messages.ProgressBepInExInstallComplete,
                installation.GetDescription(),
                game.Folder
            )
        );
        return installation;
    }

    public static void Remove(GameLayout game, IProgress<string> progress)
    {
        BepInExFileLayout.Remove(game);
        progress.Report(Localization.Format(Messages.ProgressBepInExRemoval, game.Folder));
    }
}
