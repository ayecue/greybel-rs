using System.CommandLine;
using GreyHackSetupWizard.Cli.Resources;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Cli.Commands;

internal static class LocateCommand
{
    public static Command Create()
    {
        Option<DirectoryInfo?> game = Options.CreateGameOption();
        var command = new Command("locate", CommandText.LocateDescription) { game };
        command.SetAction(result =>
            CommandRunner.Run(() =>
            {
                GameStatus status = GameStatus.Read(
                    GameLocator.Require(result.GetValue(game)?.FullName)
                );
                Console.WriteLine(Localization.Format(CommandText.LocateGame, status.Game.Folder));
                Console.WriteLine(
                    Localization.Format(
                        CommandText.LocateBepInEx,
                        status.BepInEx?.GetDescription() ?? CommandText.ValueNotInstalled
                    )
                );
                Console.WriteLine(
                    Localization.Format(
                        CommandText.LocatePlugins,
                        status.PluginAssemblies.Count == 0
                            ? CommandText.ValueNone
                            : string.Join(", ", status.PluginAssemblies)
                    )
                );
                Console.WriteLine(
                    Localization.Format(
                        CommandText.LocatePort,
                        status.Port?.ToString()
                            ?? Localization.Format(
                                CommandText.ValuePortDefault,
                                PluginConfig.DefaultPort
                            )
                    )
                );
            })
        );
        return command;
    }
}
