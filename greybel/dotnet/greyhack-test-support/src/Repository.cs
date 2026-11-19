using System.Reflection;

namespace GreyHackTestSupport;

internal static class Repository
{
    public static readonly string Dotnet = GetFolder("GreybelDotnet");

    public static readonly string Target = GetFolder("Target");

    /// <summary>The Cargo manifest of the applications' workspace, which has greyhack-setup.</summary>
    public static readonly string AppsManifest = GetFolder("AppsManifest");

    private static string GetFolder(string name) =>
        typeof(Repository)
            .Assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .Single(attribute => attribute.Key == name)
            .Value!;
}
