using System.CommandLine;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Cli.Resources;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Cli.Commands;

internal static class PluginCommand
{
    private const string LatestTag = "latest";

    public static Command Create() =>
        new("plugin", CommandText.PluginDescription) { Install(), Remove() };

    private static Command Install()
    {
        Option<DirectoryInfo?> game = Options.CreateGameOption();
        Option<int?> major = Options.CreateMajorOption(CommandText.PluginVersionOption);
        Option<FileInfo?> assembly = new("--assembly") { Description = CommandText.AssemblyOption };
        Option<string?> release = new("--release") { Description = CommandText.ReleaseOption };
        Option<string> repository = new("--repository")
        {
            Description = CommandText.RepositoryOption,
            DefaultValueFactory = _ => PluginRelease.DefaultRepository,
        };
        Option<bool> yes = Options.CreateYesOption();
        Option<DirectoryInfo> cache = Options.CreateCacheOption();
        var command = new Command("install", CommandText.PluginInstallDescription)
        {
            game,
            major,
            assembly,
            release,
            repository,
            yes,
            cache,
        };
        command.Validators.Add(result =>
        {
            if ((result.GetValue(assembly) == null) == (result.GetValue(release) == null))
            {
                result.AddError(CommandText.PluginSourceInvalid);
            }
        });
        command.SetAction(result =>
            CommandRunner.Run(async () =>
            {
                GameLayout layout = GameLocator.Require(result.GetValue(game)?.FullName);
                BepInExMajor bepInEx = result.GetValue(major) is int majorOption
                    ? (BepInExMajor)majorOption
                    : PluginSetup.GetGameMajor(layout);
                using var http = new HttpClient();
                var download = new DownloadSettings(http, result.GetRequiredValue(cache).FullName);
                PluginSource source;
                if (result.GetValue(assembly) is { } file)
                {
                    source = PluginSource.FromAssembly(file.FullName);
                }
                else
                {
                    string tag = result.GetValue(release)!;
                    PluginRelease pluginRelease = await PluginSetup.FindReleaseAsync(
                        download,
                        result.GetRequiredValue(repository),
                        tag == LatestTag ? null : tag
                    );
                    ReleaseAsset asset = pluginRelease.GetAsset(PluginBuild.MessageHook, bepInEx);
                    if (
                        !Prompt.Confirm(
                            Localization.Format(
                                CommandText.PluginDownloadPrompt,
                                asset.Name,
                                pluginRelease.Tag,
                                asset.Url
                            ),
                            result.GetValue(yes)
                        )
                    )
                    {
                        Console.Error.WriteLine(
                            Localization.Format(CommandText.PluginDownloadDeclined, asset.Url)
                        );
                        return 1;
                    }
                    source = PluginSource.FromRelease(pluginRelease);
                }
                await PluginSetup.InstallAsync(
                    layout,
                    bepInEx,
                    source,
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
        var command = new Command("remove", CommandText.PluginRemoveDescription) { game };
        command.SetAction(result =>
            CommandRunner.Run(() =>
                PluginSetup.Remove(
                    GameLocator.Require(result.GetValue(game)?.FullName),
                    ConsoleProgress.Instance
                )
            )
        );
        return command;
    }
}
