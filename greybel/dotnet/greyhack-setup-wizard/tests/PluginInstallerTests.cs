using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Tests.Support;

namespace GreyHackSetupWizard.Tests;

public class PluginInstallerTests
{
    [Fact]
    public void ShouldReplaceTheBuildOfTheOtherMajor()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        FakeGame.Write(game, "BepInEx/plugins/GreyHackMessageHook5.dll");
        FakeGame.Write(game, "BepInEx/plugins/GreyHackMessageHook5.pdb");
        string build = folder.GetPath("GreyHackMessageHook.dll");
        File.WriteAllText(build, "beta build");
        File.WriteAllText(folder.GetPath("GreyHackMessageHook.pdb"), "symbols");

        PluginInstaller.Install(game, PluginBuild.MessageHook, BepInExMajor.Beta, build);

        Assert.Equal(["GreyHackMessageHook"], PluginInstaller.GetAssemblies(game));
        Assert.True(File.Exists(Path.Combine(game.PluginsFolder, "GreyHackMessageHook.pdb")));
        Assert.False(File.Exists(Path.Combine(game.PluginsFolder, "GreyHackMessageHook5.pdb")));
    }

    [Fact]
    public void ShouldInstallIntoAGameWithoutAPluginsFolder()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        string build = folder.GetPath("GreyHackMessageHook5.dll");
        File.WriteAllText(build, "stable build");

        PluginInstaller.Install(game, PluginBuild.MessageHook, BepInExMajor.Stable, build);

        Assert.Equal(["GreyHackMessageHook5"], PluginInstaller.GetAssemblies(game));
    }

    [Fact]
    public void ShouldRejectTheBuildOfAnotherMajor()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        string build = folder.GetPath("GreyHackMessageHook5.dll");
        File.WriteAllText(build, "stable build");

        var error = Assert.Throws<SetupException>(() =>
            PluginInstaller.Install(game, PluginBuild.MessageHook, BepInExMajor.Beta, build)
        );

        Assert.EndsWith(
            "is the wrong plugin file. BepInEx 6 needs GreyHackMessageHook.dll.",
            error.Message
        );
    }
}
