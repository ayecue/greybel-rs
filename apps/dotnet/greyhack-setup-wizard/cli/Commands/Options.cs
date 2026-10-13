using System.CommandLine;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Cli.Resources;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Cli.Commands;

internal static class Options
{
    public static Option<DirectoryInfo?> CreateGameOption() =>
        new("--game")
        {
            Description = Localization.Format(CommandText.GameOption, GameLocator.Variable),
        };

    public static Option<bool> CreateYesOption() =>
        new("--yes", "-y") { Description = CommandText.YesOption };

    public static Option<DirectoryInfo> CreateCacheOption() =>
        new("--cache")
        {
            Description = CommandText.CacheOption,
            DefaultValueFactory = _ => new DirectoryInfo(DownloadSettings.GetDefaultCacheFolder()),
        };

    public static Option<string?> CreateLanguageOption() =>
        new(Localization.Option) { Description = CommandText.LanguageOption, Recursive = true };

    public static Option<int?> CreateMajorOption(string description) =>
        new("--bepinex")
        {
            Description = description,
            Validators =
            {
                result =>
                {
                    if (
                        result.GetValueOrDefault<int?>() is int major
                        && !Enum.IsDefined((BepInExMajor)major)
                    )
                    {
                        result.AddError(CommandText.BepInExOptionInvalid);
                    }
                },
            },
        };
}
