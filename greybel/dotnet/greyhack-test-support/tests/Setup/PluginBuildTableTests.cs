using GreyHackTestSupport.Tests.Support;
using static GreyHackTestSupport.Tests.Support.PluginBuildTableSetup;

namespace GreyHackTestSupport.Tests.Setup;

public class PluginBuildTableTests
{
    [Theory]
    [MemberData(nameof(GetProjects), MemberType = typeof(PluginBuildTableSetup))]
    public void ShouldPointEachPluginAtExactlyOneProject(string project)
    {
        string[] files = Directory.GetFiles(Path.Combine(Repository.Dotnet, project), "*.csproj");

        Assert.Single(files);
    }
}
