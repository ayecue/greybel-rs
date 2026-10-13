using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.Game;

public static class GameLocator
{
    public const string Variable = "GREY_HACK_PATH";

    public static GameLayout? Locate()
    {
        string? folder = Environment.GetEnvironmentVariable(Variable);
        if (string.IsNullOrEmpty(folder))
        {
            folder = SteamLibraryCatalog.FindGame(PlatformDetector.Current);
        }
        return folder == null ? null : GameLayout.FromFolder(folder);
    }

    public static GameLayout Require(string? folder) =>
        folder != null
            ? GameLayout.FromFolder(folder)
            : Locate()
                ?? throw new SetupException(Localization.Format(Messages.GameNotFound, Variable));
}
