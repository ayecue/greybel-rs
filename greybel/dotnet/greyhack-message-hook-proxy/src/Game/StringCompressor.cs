#nullable enable
using System;
using System.IO;
using System.IO.Compression;
using System.Text;

namespace GreyHackMessageHook.Game;

internal static class StringCompressor
{
    public static byte[] Zip(string text)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(text);
        if (bytes.Length < 24)
        {
            return ToRaw(bytes);
        }
        using var archive = new MemoryStream();
        archive.WriteByte(1);
        using (var gzip = new GZipStream(archive, CompressionMode.Compress, leaveOpen: true))
        {
            gzip.Write(bytes, 0, bytes.Length);
        }
        byte[] archiveBytes = archive.ToArray();
        return GetBase64Length(archiveBytes.Length) >= GetBase64Length(bytes.Length + 1)
            ? ToRaw(bytes)
            : archiveBytes;
    }

    public static string Unzip(byte[]? bytes)
    {
        if (bytes == null || bytes.Length == 0)
        {
            return "";
        }
        if (bytes[0] == 0)
        {
            return Encoding.UTF8.GetString(bytes, 1, bytes.Length - 1);
        }
        using var source = new MemoryStream(bytes, 1, bytes.Length - 1);
        using var text = new MemoryStream();
        using (var gzip = new GZipStream(source, CompressionMode.Decompress))
        {
            gzip.CopyTo(text);
        }
        return Encoding.UTF8.GetString(text.ToArray());
    }

    private static byte[] ToRaw(byte[] bytes)
    {
        var raw = new byte[bytes.Length + 1];
        Buffer.BlockCopy(bytes, 0, raw, 1, bytes.Length);
        return raw;
    }

    private static int GetBase64Length(int bytes) => (bytes + 2) / 3 * 4;
}
