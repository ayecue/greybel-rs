using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Tests.Support;

namespace GreyHackSetupWizard.Tests;

public class PluginConfigTests
{
    [Fact]
    public void ShouldWriteANewConfigWithThePort()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);

        PluginConfig.SetPort(game, 7797);

        Assert.Equal("[General]\n\nPort = 7797\n", File.ReadAllText(PluginConfig.GetPath(game)));
        Assert.Equal(7797, PluginConfig.GetPort(game));
    }

    [Fact]
    public void ShouldChangeOnlyThePortOfTheConfigThePluginWrote()
    {
        string config =
            "## Settings file\n\n[General]\n\n## Port\n# Setting type: Int32\nPort = 7777\n\nFile Extensions = src,gs,ms\n";

        string changed = PluginConfig.WithPort(config, 7797);

        Assert.Equal(config.Replace("Port = 7777", "Port = 7797"), changed);
    }

    [Fact]
    public void ShouldAddThePortToTheGeneralSectionWithoutIt()
    {
        string config = "[General]\nFile Extensions = src\n";

        string changed = PluginConfig.WithPort(config, 7797);

        Assert.Equal("[General]\n\nPort = 7797\nFile Extensions = src\n", changed);
    }
}
