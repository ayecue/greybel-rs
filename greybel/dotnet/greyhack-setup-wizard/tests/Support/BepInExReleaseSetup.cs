using GreyHackSetupWizard.BepInEx;

namespace GreyHackSetupWizard.Tests.Support;

internal static class BepInExReleaseSetup
{
    public static TheoryData<BepInExMajor, Platform> GetTargets() =>
        new(
            from major in Enum.GetValues<BepInExMajor>()
            from platform in Enum.GetValues<Platform>()
            select (major, platform)
        );
}
