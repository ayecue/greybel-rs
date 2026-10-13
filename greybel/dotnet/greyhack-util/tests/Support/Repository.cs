using System.Reflection;

namespace GreyHackUtil.Tests.Support;

public static class Repository
{
    private static readonly string Dotnet = GetFolder("GreybelDotnet");

    private static readonly string Rust = GetFolder("GreybelRust");

    public static readonly string ContractDirectory = GetFolder("GreybelContracts");

    public static readonly string ProtocolFile = Path.Combine(
        Rust,
        "..",
        "..",
        "protocols",
        "greyhack",
        "1.0.protocol"
    );

    private static string GetFolder(string name) =>
        typeof(Repository)
            .Assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .Single(attribute => attribute.Key == name)
            .Value!;

    public static string? FindInstalledAssembly()
    {
        string? managed = Environment.GetEnvironmentVariable("GREY_HACK_MANAGED_PATH");
        if (
            string.IsNullOrEmpty(managed)
            && Environment.GetEnvironmentVariable("GREY_HACK_PATH") is { Length: > 0 } game
        )
        {
            string app = Path.Combine(game, "Grey Hack.app");
            managed = Directory.Exists(app)
                ? Path.Combine(app, "Contents", "Resources", "Data", "Managed")
                : Path.Combine(game, "Grey Hack_Data", "Managed");
        }
        string? assembly = managed == null ? null : Path.Combine(managed, "Assembly-CSharp.dll");
        return File.Exists(assembly) ? assembly : null;
    }

    public static string GetGameContract()
    {
        var (_, contents) = AssetBundle
            .BundleFile.Read(
                File.ReadAllBytes(Path.Combine(ContractDirectory, Contracts.ContractSet.GameBundle))
            )
            .Single(file => file.Name == Contracts.ContractSet.GameFile);
        return System.Text.Encoding.UTF8.GetString(contents);
    }
}
