using System.CommandLine;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Cli.Resources;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Cli.Commands;

internal static class BepInExCommand
{
    public static Command Create() =>
        new("bepinex", CommandText.BepInExDescription) { Install(), Remove() };

    private static Command Install()
    {
        Option<DirectoryInfo?> game = Options.CreateGameOption();
        Option<int?> major = Options.CreateMajorOption(CommandText.BepInExVersionOption);
        major.Required = true;
        Option<FileInfo?> archive = new("--zip") { Description = CommandText.ZipOption };
        Option<bool> yes = Options.CreateYesOption();
        Option<DirectoryInfo> cache = Options.CreateCacheOption();
        var command = new Command("install", CommandText.BepInExInstallDescription)
        {
            game,
            major,
            archive,
            yes,
            cache,
        };
        command.SetAction(result =>
            CommandRunner.Run(async () =>
            {
                GameLayout layout = GameLocator.Require(result.GetValue(game)?.FullName);
                var bepInEx = (BepInExMajor)result.GetValue(major)!.Value;
                string? path = result.GetValue(archive)?.FullName;
                if (path == null)
                {
                    BepInExRelease release = BepInExSetup.GetRelease(bepInEx);
                    if (
                        !Prompt.Confirm(
                            Localization.Format(
                                CommandText.BepInExDownloadPrompt,
                                release.Version,
                                release.Url
                            ),
                            result.GetValue(yes)
                        )
                    )
                    {
                        Console.Error.WriteLine(
                            Localization.Format(CommandText.BepInExDownloadDeclined, release.Url)
                        );
                        return 1;
                    }
                }
                using var http = new HttpClient();
                var download = new DownloadSettings(http, result.GetRequiredValue(cache).FullName);
                await BepInExSetup.InstallAsync(
                    layout,
                    bepInEx,
                    path,
                    download,
                    ConsoleProgress.Instance
                );
                return 0;
            })
        );
        return command;
    }

    private static Command Remove()
    {
        Option<DirectoryInfo?> game = Options.CreateGameOption();
        var command = new Command("remove", CommandText.BepInExRemoveDescription) { game };
        command.SetAction(result =>
            CommandRunner.Run(() =>
                BepInExSetup.Remove(
                    GameLocator.Require(result.GetValue(game)?.FullName),
                    ConsoleProgress.Instance
                )
            )
        );
        return command;
    }
}
