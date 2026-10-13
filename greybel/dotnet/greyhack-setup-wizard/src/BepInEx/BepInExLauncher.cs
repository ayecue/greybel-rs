using System.Text.RegularExpressions;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.BepInEx;

public static partial class BepInExLauncher
{
    public const string FileName = "run_bepinex.sh";

    public static string? GetRequiredArchitectures(BepInExMajor major) =>
        major == BepInExMajor.Beta ? "x86_64" : null;

    public static void PreferArchitectures(GameLayout game, string architectures)
    {
        string path = Path.Combine(game.Folder, FileName);
        string script = File.ReadAllText(path);
        if (!GetPreferenceRegex().IsMatch(script))
        {
            throw new SetupException(
                Localization.Format(Messages.LauncherPreferenceMissing, architectures, path)
            );
        }
        File.WriteAllText(
            path,
            GetPreferenceRegex().Replace(script, $"archpreference=\"{architectures}\"", 1)
        );
    }

    public static string? GetArchitectures(GameLayout game)
    {
        string path = Path.Combine(game.Folder, FileName);
        Match match = File.Exists(path)
            ? GetPreferenceRegex().Match(File.ReadAllText(path))
            : Match.Empty;
        return match.Success ? match.Groups[1].Value : null;
    }

    [GeneratedRegex("^archpreference=\"([^\"]*)\"", RegexOptions.Multiline)]
    private static partial Regex GetPreferenceRegex();
}
