using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.Setup;

public static class PluginSetup
{
    public static BepInExMajor GetGameMajor(GameLayout game) =>
        BepInExInstallation.Detect(game)?.Major
        ?? throw new SetupException(Messages.BepInExMissing);

    public static Task<PluginRelease> FindReleaseAsync(
        DownloadSettings download,
        string repository,
        string? tag
    ) => PluginRelease.FindAsync(download.Http, repository, tag);

    public static async Task<string> InstallAsync(
        GameLayout game,
        BepInExMajor major,
        PluginSource source,
        DownloadSettings download,
        IProgress<string> progress
    )
    {
        string assemblyPath;
        if (source.Release is { } release)
        {
            ReleaseAsset asset = release.GetAsset(PluginBuild.MessageHook, major);
            progress.Report(
                Localization.Format(
                    Messages.ProgressPluginDownload,
                    asset.Name,
                    release.Tag,
                    asset.Url
                )
            );
            assemblyPath = await release.FetchAsync(
                download.Http,
                PluginBuild.MessageHook,
                major,
                download.CacheFolder
            );
        }
        else
        {
            assemblyPath = source.AssemblyPath!;
        }
        PluginInstaller.Install(game, PluginBuild.MessageHook, major, assemblyPath);
        progress.Report(
            Localization.Format(
                Messages.ProgressPluginInstallComplete,
                Path.GetFileName(assemblyPath),
                game.PluginsFolder
            )
        );
        return assemblyPath;
    }

    public static void Remove(GameLayout game, IProgress<string> progress)
    {
        PluginInstaller.Remove(game, PluginBuild.MessageHook);
        progress.Report(Localization.Format(Messages.ProgressPluginRemoval, game.PluginsFolder));
    }

    public static void ConfigurePort(GameLayout game, int port, IProgress<string> progress)
    {
        if (port is < 1 or > 65535)
        {
            throw new SetupException(Localization.Format(Messages.PortInvalid, port));
        }
        PluginConfig.SetPort(game, port);
        progress.Report(
            Localization.Format(Messages.ProgressPortChange, port, PluginConfig.GetPath(game))
        );
    }
}
