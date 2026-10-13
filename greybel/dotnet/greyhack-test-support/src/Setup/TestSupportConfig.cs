using GreyHackSetupWizard.Game;

namespace GreyHackTestSupport.Setup;

internal static class TestSupportConfig
{
    /// <summary>
    /// Configures the test support protocol port and the game's single-player server port. The plugin replaces the
    /// fixed server port 21200 so multiple game copies can run concurrently.
    /// </summary>
    public static void SetPorts(GameLayout game, int testPort, int serverPort)
    {
        Directory.CreateDirectory(game.ConfigFolder);
        File.WriteAllText(
            Path.Combine(game.ConfigFolder, "GreyHackTestSupport.cfg"),
            "[General]\nTestPort = " + testPort + "\nServerPort = " + serverPort + "\n"
        );
    }
}
