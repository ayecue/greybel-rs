using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Tests.Support;
using static GreyHackSetupWizard.Tests.Support.BepInExReleaseSetup;

namespace GreyHackSetupWizard.Tests;

public class BepInExReleaseTests
{
    [Theory]
    [MemberData(nameof(GetTargets), MemberType = typeof(BepInExReleaseSetup))]
    public void ShouldHaveAReleaseOfEveryMajorForEveryPlatform(
        BepInExMajor major,
        Platform platform
    )
    {
        BepInExRelease release = BepInExRelease.Get(major, platform);

        Assert.Matches("^[0-9a-f]{64}$", release.Sha256);
        Assert.EndsWith(".zip", release.FileName);
    }
}
