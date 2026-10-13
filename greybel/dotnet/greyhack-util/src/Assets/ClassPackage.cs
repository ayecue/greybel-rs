using System.Security.Cryptography;
using AssetRipper.Primitives;
using AssetRipper.Tpk;
using AssetRipper.Tpk.TypeTrees;
using AssetRipper.Tpk.TypeTrees.Json;

namespace GreyHackUtil.Assets;

internal static class ClassPackage
{
    private const string Dumps =
        "https://raw.githubusercontent.com/AssetRipper/TypeTreeDumps/20ba06e00e0625de0e49a502c6868d6cc86d0f0b/InfoJson";

    private static readonly Dictionary<string, string> Sha256 = new()
    {
        ["2022.3.62f3"] = "a9bf758ccd47fc01821713d301c3aab875a5525f6463b364b2df71b2996322dd",
    };

    public static Stream Build(string cache, string version)
    {
        if (!Sha256.TryGetValue(version, out string? sha256))
        {
            throw new InvalidDataException(
                $"no type tree dump is pinned for Unity {version}: add its SHA-256 (from {Dumps}/{version}.json) to {nameof(ClassPackage)}"
            );
        }
        string url = $"{Dumps}/{version}.json";
        string path = Path.Combine(cache, "type-trees", $"{version}.json");
        byte[] dump = File.Exists(path) ? File.ReadAllBytes(path) : Download(url);
        if (Convert.ToHexStringLower(SHA256.HashData(dump)) != sha256)
        {
            throw new InvalidDataException($"{url} is not the one expected (SHA-256 {sha256})");
        }
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllBytes(path, dump);
        UnityInfo info =
            UnityInfo.FromStream(new MemoryStream(dump))
            ?? throw new InvalidDataException($"{url} holds no type trees");
        return new MemoryStream(
            TpkFile
                .FromBlob(LoadBlob(UnityVersion.Parse(version), info), TpkCompressionType.None)
                .WriteToMemory()
        );
    }

    private static TpkTypeTreeBlob LoadBlob(UnityVersion version, UnityInfo info)
    {
        var blob = new TpkTypeTreeBlob { CreationTime = DateTime.UnixEpoch };
        blob.Versions.Add(version);
        blob.CommonString.Add(
            version,
            info.Strings.Select(entry => new TpkCommonString.Entry(
                    (ushort)entry.Index,
                    entry.String,
                    blob.StringBuffer
                ))
                .ToArray()
        );
        foreach (UnityClass unityClass in info.Classes)
        {
            var information = new TpkClassInformation(unityClass.TypeID);
            information.Classes.Add(
                new(
                    version,
                    ClassConversion.Convert(unityClass, blob.StringBuffer, blob.NodeBuffer)
                )
            );
            blob.ClassInformation.Add(information);
        }
        return blob;
    }

    private static byte[] Download(string url)
    {
        using var client = new HttpClient();
        return client.GetByteArrayAsync(url).GetAwaiter().GetResult();
    }
}
