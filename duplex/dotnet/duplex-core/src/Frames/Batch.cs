#nullable enable
using System;

namespace Duplex;

internal readonly struct SliceRoute : IEquatable<SliceRoute>
{
    public SliceRoute(Role from, uint channel, uint connection)
    {
        From = from;
        Channel = channel;
        Connection = connection;
    }

    public Role From { get; }

    public uint Channel { get; }

    public uint Connection { get; }

    public ulong PeerKey => (ulong)Channel * 2 + (ulong)From;

    public static SliceRoute Read(DuplexReader reader)
    {
        var from = RoleCodec.Read(reader);
        var channel = reader.ReadFixed32();
        return new SliceRoute(from, channel, reader.ReadFixed32());
    }

    public void Write(DuplexWriter writer)
    {
        RoleCodec.Write(writer, From);
        writer.WriteFixed32(Channel);
        writer.WriteFixed32(Connection);
    }

    public bool Equals(SliceRoute other) =>
        From == other.From && Channel == other.Channel && Connection == other.Connection;

    public override bool Equals(object? other) => other is SliceRoute route && Equals(route);

    public override int GetHashCode() =>
        unchecked((int)(Channel * 31 + Connection * 7 + (uint)From));
}

internal enum SliceKeyKind
{
    NoAlias,
    Announcement,
    Alias,
}

internal readonly struct SliceKey
{
    private SliceKey(SliceKeyKind kind, uint alias, SliceRoute route)
    {
        Kind = kind;
        Alias = alias;
        Route = route;
    }

    public SliceKeyKind Kind { get; }

    public uint Alias { get; }

    public SliceRoute Route { get; }

    public static SliceKey FromRoute(SliceRoute route) =>
        new SliceKey(SliceKeyKind.NoAlias, 0, route);

    public static SliceKey CreateAnnouncement(uint alias, SliceRoute route) =>
        new SliceKey(SliceKeyKind.Announcement, alias, route);

    public static SliceKey FromAlias(uint alias) =>
        new SliceKey(SliceKeyKind.Alias, alias, default);

    public void Write(DuplexWriter writer)
    {
        switch (Kind)
        {
            case SliceKeyKind.NoAlias:
                writer.WriteU32(0);
                Route.Write(writer);
                break;
            case SliceKeyKind.Announcement:
                writer.WriteU32(Alias * 2 - 1);
                Route.Write(writer);
                break;
            case SliceKeyKind.Alias:
                writer.WriteU32(Alias * 2);
                break;
        }
    }

    public static SliceKey Read(DuplexReader reader)
    {
        var key = reader.ReadU32();
        if (key == 0)
        {
            return FromRoute(SliceRoute.Read(reader));
        }
        if (key % 2 == 1)
        {
            return CreateAnnouncement((key + 1) / 2, SliceRoute.Read(reader));
        }
        return FromAlias(key / 2);
    }
}

internal readonly struct Slice
{
    public Slice(SliceKey key, int start, int end)
    {
        Key = key;
        Start = start;
        End = end;
    }

    public SliceKey Key { get; }

    public int Start { get; }

    public int End { get; }
}

internal sealed class BatchReader
{
    private readonly DuplexReader reader;
    private uint slicesLeft;

    private BatchReader(byte[] bytes)
    {
        reader = new DuplexReader(bytes);
        Core = reader.ReadU8();
        slicesLeft = reader.ReadU32();
        if (slicesLeft == 0)
        {
            throw new MalformedException("a batch without slices");
        }
    }

    public byte Core { get; }

    public static BatchReader? FromBytes(byte[] bytes)
    {
        try
        {
            return new BatchReader(bytes);
        }
        catch (MalformedException)
        {
            return null;
        }
    }

    public Slice? Next()
    {
        if (slicesLeft == 0)
        {
            return null;
        }
        slicesLeft--;
        var key = SliceKey.Read(reader);
        var length = slicesLeft == 0 ? reader.UnreadLength : reader.ReadLength();
        var start = reader.Offset;
        reader.Skip(length);
        return new Slice(key, start, start + length);
    }
}

internal sealed class BatchWriter
{
    internal const byte CoreMajor = 1;

    private readonly DuplexWriter writer;
    private int slicesLeft;

    public BatchWriter(DuplexWriter writer, byte core, int slices)
    {
        writer.Reset();
        writer.WriteU8(core);
        writer.WriteU32((uint)slices);
        this.writer = writer;
        slicesLeft = slices;
    }

    public void WriteSlice(SliceKey key, byte[] bytes, int start, int end, int lastFrameStart)
    {
        slicesLeft--;
        key.Write(writer);
        var lastBodyStart = Varuint.FindEnd(bytes, lastFrameStart + 1);
        if (lastBodyStart < 0 || lastBodyStart > end)
        {
            throw new MalformedException("a frame cut short");
        }
        if (slicesLeft > 0)
        {
            writer.WriteLength(end - start - (lastBodyStart - lastFrameStart - 1));
        }
        writer.WriteRaw(bytes, start, lastFrameStart);
        writer.WriteByte((byte)(bytes[lastFrameStart] | FrameLayout.LastFrame));
        writer.WriteRaw(bytes, lastBodyStart, end);
    }

    public void WriteFragment(SliceKey key, byte[] bytes, int start, int end, bool isLastPiece)
    {
        slicesLeft--;
        key.Write(writer);
        if (slicesLeft > 0)
        {
            writer.WriteLength(2 + end - start);
        }
        writer.WriteByte((byte)((byte)FrameKind.Fragment | FrameLayout.LastFrame));
        writer.WriteByte(isLastPiece ? FrameLayout.LastPiece : (byte)0);
        writer.WriteRaw(bytes, start, end);
    }
}

internal static class Varuint
{
    public static int FindEnd(byte[] bytes, int offset)
    {
        for (var index = offset; index < bytes.Length && index - offset < 10; index++)
        {
            if ((bytes[index] & 0x80) == 0)
            {
                return index + 1;
            }
        }
        return -1;
    }

    public static int FindFrameEnd(byte[] bytes, int offset, int limit)
    {
        var lengthEnd = FindEnd(bytes, offset + 1);
        if (lengthEnd < 0 || lengthEnd > limit)
        {
            return -1;
        }
        var reader = new DuplexReader(bytes, offset + 1, lengthEnd);
        return lengthEnd + reader.ReadLength();
    }
}
