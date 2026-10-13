using System.IO.Compression;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.BepInEx;

public static class BepInExInstaller
{
    public static BepInExInstallation Install(GameLayout game, string archive)
    {
        using (ZipArchive zip = ZipFile.OpenRead(archive))
        {
            if (
                !zip.Entries.Any(entry =>
                    entry.FullName.StartsWith("BepInEx/core/", StringComparison.Ordinal)
                )
            )
            {
                throw new SetupException(Localization.Format(Messages.ArchiveNotBepInEx, archive));
            }
        }
        BepInExFileLayout.Remove(game);
        ZipFile.ExtractToDirectory(archive, game.Folder, overwriteFiles: true);
        string launcher = Path.Combine(game.Folder, BepInExLauncher.FileName);
        if (!OperatingSystem.IsWindows() && File.Exists(launcher))
        {
            File.SetUnixFileMode(
                launcher,
                File.GetUnixFileMode(launcher)
                    | UnixFileMode.UserExecute
                    | UnixFileMode.GroupExecute
                    | UnixFileMode.OtherExecute
            );
        }
        BepInExInstallation installation =
            BepInExInstallation.Detect(game)
            ?? throw new SetupException(
                Localization.Format(Messages.BepInExAbsentAfterInstall, archive)
            );
        if (
            BepInExLauncher.GetRequiredArchitectures(installation.Major) is { } architectures
            && BepInExLauncher.GetArchitectures(game) != null
        )
        {
            BepInExLauncher.PreferArchitectures(game, architectures);
        }
        return installation;
    }

    public static bool HasRelease(GameLayout game, BepInExRelease release) =>
        BepInExInstallation.Detect(game) is { } installation
        && installation.Major == release.Major
        && installation.Version == release.Version;
}
