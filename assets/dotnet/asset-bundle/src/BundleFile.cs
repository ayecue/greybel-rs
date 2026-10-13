using System.IO.Compression;
using System.Text;

namespace AssetBundle;

public static class BundleFile
{
    private static readonly byte[] Magic = "BNDL"u8.ToArray();
    private const byte Version = 1;

    public static byte[] Write(IReadOnlyList<(string Name, byte[] Contents)> files)
    {
        byte[][] compressedFiles = files.Select(file => Deflate(file.Contents)).ToArray();
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);
        writer.Write(Magic);
        writer.Write(Version);
        writer.Write((uint)files.Count);
        for (int index = 0; index < files.Count; index++)
        {
            byte[] name = Encoding.UTF8.GetBytes(files[index].Name);
            writer.Write(checked((ushort)name.Length));
            writer.Write(name);
            writer.Write(checked((uint)files[index].Contents.Length));
            writer.Write(checked((uint)compressedFiles[index].Length));
        }
        foreach (byte[] contents in compressedFiles)
        {
            writer.Write(contents);
        }
        writer.Flush();
        return stream.ToArray();
    }

    public static IReadOnlyList<(string Name, byte[] Contents)> Read(byte[] bytes)
    {
        using var reader = new BinaryReader(new MemoryStream(bytes));
        if (!reader.ReadBytes(4).SequenceEqual(Magic))
        {
            throw new InvalidDataException("not a bundle: no BNDL magic");
        }
        byte version = reader.ReadByte();
        if (version != Version)
        {
            throw new InvalidDataException($"bundle version {version}, only {Version} is read");
        }
        uint count = reader.ReadUInt32();
        var table = new List<(string Name, int Unpacked, int Packed)>();
        for (uint index = 0; index < count; index++)
        {
            string name = Encoding.UTF8.GetString(reader.ReadBytes(reader.ReadUInt16()));
            table.Add((name, checked((int)reader.ReadUInt32()), checked((int)reader.ReadUInt32())));
        }
        return table
            .Select(entry =>
            {
                byte[] contents = Inflate(reader.ReadBytes(entry.Packed));
                if (contents.Length != entry.Unpacked)
                {
                    throw new InvalidDataException(
                        $"{entry.Name}: unpacked to {contents.Length} bytes, not {entry.Unpacked}"
                    );
                }
                return (entry.Name, contents);
            })
            .ToList();
    }

    private static readonly byte[] EmptyStream = [0x03, 0x00];

    private static byte[] Deflate(byte[] contents)
    {
        using var compressedStream = new MemoryStream();
        using (
            var deflate = new DeflateStream(
                compressedStream,
                CompressionLevel.SmallestSize,
                leaveOpen: true
            )
        )
        {
            deflate.Write(contents);
        }

        return compressedStream.Length == 0 ? EmptyStream : compressedStream.ToArray();
    }

    private static byte[] Inflate(byte[] compressedBytes)
    {
        using var inflatedStream = new MemoryStream();
        using (
            var inflate = new DeflateStream(
                new MemoryStream(compressedBytes),
                CompressionMode.Decompress
            )
        )
        {
            inflate.CopyTo(inflatedStream);
        }
        return inflatedStream.ToArray();
    }
}
