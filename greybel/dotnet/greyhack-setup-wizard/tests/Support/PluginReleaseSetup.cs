using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Plugins;

namespace GreyHackSetupWizard.Tests.Support;

internal static class PluginReleaseSetup
{
    internal static PluginRelease Release() =>
        PluginRelease.Parse(
            File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "Fixtures", "release.json"))
        );
}
