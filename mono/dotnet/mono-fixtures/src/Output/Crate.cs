using System.Reflection;

namespace MonoFixtures.Output;

internal sealed class Crate(string name)
{
    private static readonly string Workspace = typeof(Crate)
        .Assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
        .Single(attribute => attribute.Key == "MonoRust")
        .Value!;

    private readonly string _directory = Path.Combine(Workspace, name);

    public string GetPath(params string[] parts) => Path.Combine(_directory, Path.Combine(parts));

    public string GetFixturePath(string fileName) => GetPath("tests", "fixtures", fileName);
}
