using GreyHackTestSupport.Setup;

namespace GreyHackTestSupport.Tests.Support;

internal static class PluginBuildTableSetup
{
    public static TheoryData<string> GetProjects() =>
        new(PluginBuildTable.Plugins.Select(plugin => plugin.Project));
}
