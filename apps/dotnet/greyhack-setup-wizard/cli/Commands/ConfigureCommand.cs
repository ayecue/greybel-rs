using System.CommandLine;
using GreyHackSetupWizard.Cli.Resources;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Cli.Commands;

internal static class ConfigureCommand
{
    public static Command Create()
    {
        Option<DirectoryInfo?> game = Options.CreateGameOption();
        Option<int> port = new("--port")
        {
            Description = Localization.Format(CommandText.PortOption, PluginConfig.DefaultPort),
            Required = true,
        };
        var command = new Command("configure", CommandText.ConfigureDescription) { game, port };
        command.SetAction(result =>
            CommandRunner.Run(() =>
                PluginSetup.ConfigurePort(
                    GameLocator.Require(result.GetValue(game)?.FullName),
                    result.GetValue(port),
                    ConsoleProgress.Instance
                )
            )
        );
        return command;
    }
}
