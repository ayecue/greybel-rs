using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Plugins;
using static GreyHackSetupWizard.Tests.Support.PluginReleaseSetup;

namespace GreyHackSetupWizard.Tests;

public class PluginReleaseTests
{
    [Fact]
    public void ShouldPickTheBuildOfTheMajorWithItsSha256()
    {
        ReleaseAsset asset = Release().GetAsset(PluginBuild.MessageHook, BepInExMajor.Stable);

        Assert.Equal(("GreyHackMessageHook5.dll", new string('a', 64)), (asset.Name, asset.Sha256));
    }

    [Fact]
    public void ShouldRejectABuildGitHubRecordsNoSha256For()
    {
        var error = Assert.Throws<SetupException>(() =>
            Release().GetAsset(PluginBuild.MessageHook, BepInExMajor.Beta)
        );

        Assert.Equal(
            "Release v0.1.0 has no checksum for GreyHackMessageHook.dll, so it can't be verified.",
            error.Message
        );
    }
}
