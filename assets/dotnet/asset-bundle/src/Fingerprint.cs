using System.Security.Cryptography;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;

namespace AssetBundle;

public static class Fingerprint
{
    public const string FileName = "fingerprint.json";

    private static readonly JsonSerializerOptions Options = new()
    {
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    public static string Compute(string game, IEnumerable<(string Name, byte[] Contents)> files)
    {
        var json = new StringBuilder();
        json.Append("{\n  \"game\": ")
            .Append(JsonSerializer.Serialize(game, Options))
            .Append(",\n  \"files\": {");
        string separator = "\n";
        foreach (var (name, contents) in files.OrderBy(file => file.Name, StringComparer.Ordinal))
        {
            json.Append(separator)
                .Append("    ")
                .Append(JsonSerializer.Serialize(name, Options))
                .Append(": \"")
                .Append(Convert.ToHexStringLower(SHA256.HashData(contents)))
                .Append('"');
            separator = ",\n";
        }
        json.Append("\n  }\n}\n");
        return json.ToString();
    }

    public static void WriteBundle(
        string folder,
        string name,
        string game,
        IReadOnlyList<(string Name, byte[] Contents)> files
    )
    {
        Directory.CreateDirectory(folder);
        File.WriteAllBytes(Path.Combine(folder, name), BundleFile.Write(files));
        File.WriteAllText(Path.Combine(folder, FileName), Compute(game, files));
    }
}
