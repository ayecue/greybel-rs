using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Tests.Support;

namespace GreyHackSetupWizard.Tests;

public class GameLayoutTests
{
    [Fact]
    public void ShouldReadTheMacOSGamesDataFolderInItsApp()
    {
        using var folder = new TemporaryFolder();

        GameLayout game = FakeGame.Create(folder);

        Assert.Equal(
            Path.Combine(folder.Path, "Grey Hack.app", "Contents", "Resources", "Data", "Managed"),
            game.ManagedFolder
        );
    }

    [Fact]
    public void ShouldReadTheWindowsGamesDataFolderNextToIt()
    {
        using var folder = new TemporaryFolder();
        File.WriteAllText(folder.GetPath("Grey Hack.exe"), "");

        GameLayout game = GameLayout.FromFolder(folder.Path);

        Assert.Equal(
            (folder.GetPath("Grey Hack.exe"), folder.GetPath("Grey Hack_Data")),
            (game.Executable, game.DataFolder)
        );
    }

    [Fact]
    public void ShouldRejectAFolderWithoutTheGame()
    {
        using var folder = new TemporaryFolder();

        var error = Assert.Throws<SetupException>(() => GameLayout.FromFolder(folder.Path));

        Assert.StartsWith("Grey Hack wasn't found in ", error.Message);
    }
}
