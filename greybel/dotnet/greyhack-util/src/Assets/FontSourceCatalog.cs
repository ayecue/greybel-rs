using System.Formats.Tar;
using System.IO.Compression;
using System.Security.Cryptography;

namespace GreyHackUtil.Assets;

internal static class FontSourceCatalog
{
    private sealed record Source(
        string Family,
        string File,
        string Url,
        string? Member,
        string Sha256
    );

    private static readonly Source[] Sources =
    [
        new(
            "Source Code Pro",
            "SourceCodePro-Regular.ttf",
            "https://raw.githubusercontent.com/adobe-fonts/source-code-pro/803b7e23ec97ae58b6232ea76519a76d428ba268/TTF/SourceCodePro-Regular.ttf",
            null,
            "74bd80d3e42a08517cd7e1108ba3d86f2da29ac0f3065be95e0357956ab9db37"
        ),
        new(
            "Liberation Sans",
            "LiberationSans-Regular.ttf",
            "https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz",
            "liberation-fonts-ttf-2.1.5/LiberationSans-Regular.ttf",
            "76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8"
        ),
        new(
            "JetBrains Mono",
            "JetBrainsMono-Regular.ttf",
            "https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip",
            "fonts/ttf/JetBrainsMono-Regular.ttf",
            "a0bf60ef0f83c5ed4d7a75d45838548b1f6873372dfac88f71804491898d138f"
        ),
        new(
            "M PLUS 1 Code",
            "MPLUS1Code-Regular.ttf",
            "https://fonts.gstatic.com/s/mplus1code/v18/ypvMbXOOx2xFpzmYJS3N2_J2hBN6RZ5oIp8m_7iN0HHp.ttf",
            null,
            "fb1b53f18056938e6bc178ae5c147aba26e65dcae27ab737a6bff6a04fb03a06"
        ),
        new(
            "Taviraj",
            "Taviraj-Medium.ttf",
            "https://raw.githubusercontent.com/google/fonts/097f5a9a35829ecfb159c266260422aa456ea27f/ofl/taviraj/Taviraj-Medium.ttf",
            null,
            "126a0ed061ff96c499b314bdcdacfa1c2ccd153fcd78e4fbb3575c88e98dae3d"
        ),
    ];

    public static string? GetFile(string family) =>
        Sources.FirstOrDefault(source => source.Family == family)?.File;

    public static IEnumerable<(string File, byte[] Contents)> Load(string cache)
    {
        using var client = new HttpClient();
        foreach (Source source in Sources)
        {
            string path = Path.Combine(cache, "fonts", source.File);
            byte[] contents = File.Exists(path)
                ? File.ReadAllBytes(path)
                : Download(client, source);
            if (Convert.ToHexStringLower(SHA256.HashData(contents)) != source.Sha256)
            {
                throw new InvalidDataException(
                    $"{source.File} from {source.Url} is not the one expected (SHA-256 {source.Sha256})"
                );
            }
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            File.WriteAllBytes(path, contents);
            yield return (source.File, contents);
        }
    }

    private static byte[] Download(HttpClient client, Source source)
    {
        byte[] download = client.GetByteArrayAsync(source.Url).GetAwaiter().GetResult();
        if (source.Member == null)
        {
            return download;
        }
        if (source.Url.EndsWith(".zip", StringComparison.Ordinal))
        {
            using var archive = new ZipArchive(new MemoryStream(download));
            ZipArchiveEntry entry =
                archive.GetEntry(source.Member)
                ?? throw new InvalidDataException($"{source.Url} has no {source.Member}");
            return Read(entry.Open());
        }
        using var tar = new TarReader(
            new GZipStream(new MemoryStream(download), CompressionMode.Decompress)
        );
        while (tar.GetNextEntry() is { } entry)
        {
            if (entry.Name == source.Member && entry.DataStream != null)
            {
                return Read(entry.DataStream);
            }
        }
        throw new InvalidDataException($"{source.Url} has no {source.Member}");
    }

    private static byte[] Read(Stream stream)
    {
        using var contents = new MemoryStream();
        stream.CopyTo(contents);
        return contents.ToArray();
    }
}
