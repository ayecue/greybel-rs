using System.Reflection;
using System.Text;

namespace AssetBundle.Tests.Support;

public static class Fixtures
{
    public static readonly IReadOnlyList<(string Name, byte[] Contents)> Files =
    [
        ("texts/en.json", Encoding.UTF8.GetBytes("{\"a\": 1}")),
        ("names/empty.txt", []),
        ("atlas.raw", Enumerable.Repeat((byte)7, 4096).ToArray()),
    ];

    public static string Folder =>
        Path.Combine(
            typeof(Fixtures)
                .Assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
                .Single(attribute => attribute.Key == "AssetsRust")
                .Value!,
            "asset-bundle",
            "tests",
            "fixtures"
        );

    public static byte[] Read(string name) => File.ReadAllBytes(Path.Combine(Folder, name));

    public static IReadOnlyList<(string Name, string Contents)> AsHex(
        IEnumerable<(string Name, byte[] Contents)> files
    ) => files.Select(file => (file.Name, Convert.ToHexString(file.Contents))).ToList();
}
