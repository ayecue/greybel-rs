#nullable enable
using System;
using System.Collections.Generic;

namespace Duplex;

internal sealed class Control
{
    public Control(FrameKind kind)
    {
        Kind = kind;
    }

    public FrameKind Kind { get; }

    public uint Id { get; set; }

    public uint Connection { get; set; }

    public VersionNumber Core { get; set; }

    public VersionNumber Protocol { get; set; }

    public uint Fingerprint { get; set; }

    public string? Deprecation { get; set; }

    public ErrorCode Reason { get; set; }

    public IReadOnlyList<ushort> Majors { get; set; } = Array.Empty<ushort>();

    public byte[] Payload { get; set; } = Array.Empty<byte>();
}

internal sealed class Frame
{
    public Frame(FrameKind kind)
    {
        Kind = kind;
    }

    public FrameKind Kind { get; }

    public uint TypeId { get; set; }

    public uint? Id { get; set; }

    public DuplexReader? Body { get; set; }

    public ErrorCode? Code { get; set; }

    public Violation? Violation { get; set; }

    public uint Count { get; set; }

    public bool Ends { get; set; }

    public byte[]? Bytes { get; set; }

    public Control? Control { get; set; }
}

internal readonly struct FrameHead
{
    public FrameHead(FrameKind kind, int length, uint? id)
    {
        Kind = kind;
        Length = length;
        Id = id;
    }

    public FrameKind Kind { get; }

    public int Length { get; }

    public uint? Id { get; }
}

internal static class FrameLayout
{
    public const byte LastFrame = 0x80;
    public const byte KindMask = 0x7f;
    public const uint DeclaredFailure = 0;
    public const byte LastPiece = 1;

    public static uint GetMessageKey(uint typeId, bool hasId) => typeId * 2 + (hasId ? 1u : 0u);

    public static bool IsControl(FrameKind kind) =>
        kind >= FrameKind.Listen && kind <= FrameKind.Close;

    public static bool IsFrozen(FrameKind kind) =>
        kind == FrameKind.Listen || kind == FrameKind.Open || kind == FrameKind.Refuse;

    public static bool IsKnown(uint kind) => kind <= (uint)FrameKind.Fragment;
}

internal sealed class FrameReader
{
    private readonly DuplexReader reader;
    private readonly int end;

    public FrameReader(byte[] batch, int start, int end)
    {
        reader = new DuplexReader(batch, start, end);
        this.end = end;
    }

    public ushort CoreMinor { get; set; } = DuplexWriter.Newest;

    public ushort ProtocolMinor { get; set; } = DuplexWriter.Newest;

    public int LastFrameStart { get; private set; }

    public int LastFrameLength { get; private set; }

    public Frame? Next()
    {
        for (; ; )
        {
            reader.End = end;
            if (reader.UnreadLength == 0)
            {
                return null;
            }
            var start = reader.Offset;
            var head = reader.ReadByte();
            var frameEnd =
                ((head & FrameLayout.LastFrame) == 0 ? reader.ReadLength() : reader.UnreadLength)
                + reader.Offset;
            if (frameEnd > end)
            {
                throw new MalformedException("a frame past the end of its slice");
            }
            LastFrameStart = start;
            LastFrameLength = frameEnd - start;
            var kind = (uint)(head & FrameLayout.KindMask);
            reader.End = frameEnd;
            var frame = FrameLayout.IsKnown(kind) ? Read((FrameKind)kind, frameEnd) : null;
            if (frame != null)
            {
                return frame;
            }
            reader.Offset = frameEnd;
        }
    }

    public void SkipFrame() => reader.Offset = reader.End;

    private Frame? Read(FrameKind kind, int frameEnd)
    {
        reader.Minor = FrameLayout.IsControl(kind) ? CoreMinor : ProtocolMinor;
        switch (kind)
        {
            case FrameKind.Message:
            {
                var key = reader.ReadU32();
                uint? id = key % 2 == 1 ? reader.ReadU32() : null;
                return new Frame(kind)
                {
                    TypeId = key / 2,
                    Id = id,
                    Body = ReadBody(frameEnd),
                };
            }
            case FrameKind.ResponseEnd:
            case FrameKind.ResponseStart:
            case FrameKind.ResponseItem:
            case FrameKind.MessageItem:
            case FrameKind.MessageEnd:
                return new Frame(kind) { Id = reader.ReadU32(), Body = ReadBody(frameEnd) };
            case FrameKind.Failure:
            {
                var id = reader.ReadU32();
                var code = reader.ReadU32();
                if (code == FrameLayout.DeclaredFailure)
                {
                    return new Frame(kind) { Id = id, Body = ReadBody(frameEnd) };
                }
                if (!Enum.IsDefined(typeof(ErrorCode), code))
                {
                    throw new MalformedException($"no error code {code}");
                }
                var error = (ErrorCode)code;
                var violation = error == ErrorCode.Invalid ? Violation.Read(reader) : null;
                return new Frame(kind)
                {
                    Id = id,
                    Code = error,
                    Violation = violation,
                    Body = ReadBody(frameEnd),
                };
            }
            case FrameKind.ResponseCredit:
            case FrameKind.MessageCredit:
            {
                var id = reader.ReadU32();
                return new Frame(kind) { Id = id, Count = reader.ReadU32() };
            }
            case FrameKind.Cancel:
                return new Frame(kind) { Id = reader.ReadU32() };
            case FrameKind.Fragment:
            {
                var ends = reader.ReadBool();
                return new Frame(kind) { Ends = ends, Bytes = reader.TakeAll() };
            }
            case FrameKind.Listen:
            case FrameKind.Ping:
            case FrameKind.Pong:
                return new Frame(kind) { Control = new Control(kind) };
            case FrameKind.Open:
            {
                var open = Open.Read(reader);
                return new Frame(kind)
                {
                    Control = new Control(kind)
                    {
                        Id = open.Id,
                        Core = VersionNumber.FromVersion(open.Core),
                        Protocol = VersionNumber.FromVersion(open.Protocol),
                        Fingerprint = open.Fingerprint,
                        Payload = reader.TakeAll(),
                    },
                };
            }
            case FrameKind.Ack:
            {
                var ack = Ack.Read(reader);
                if (ack.Connection == 0)
                {
                    throw new MalformedException("an ack of connection 0");
                }
                return new Frame(kind)
                {
                    Control = new Control(kind)
                    {
                        Id = ack.Id,
                        Connection = ack.Connection,
                        Core = VersionNumber.FromVersion(ack.Core),
                        Protocol = VersionNumber.FromVersion(ack.Protocol),
                        Fingerprint = ack.Fingerprint,
                        Deprecation = ack.Deprecation,
                        Payload = reader.TakeAll(),
                    },
                };
            }
            case FrameKind.Refuse:
            {
                var refuse = Refuse.Read(reader);
                var reason = Enum.IsDefined(typeof(ErrorCode), refuse.Reason)
                    ? (ErrorCode)refuse.Reason
                    : ErrorCode.VersionMismatch;
                return new Frame(kind)
                {
                    Control = new Control(kind)
                    {
                        Id = refuse.Id,
                        Reason = reason,
                        Core = VersionNumber.FromVersion(refuse.Core),
                        Protocol = VersionNumber.FromVersion(refuse.Protocol),
                        Majors = refuse.Majors,
                        Payload = reader.TakeAll(),
                    },
                };
            }
            case FrameKind.Close:
                return new Frame(kind)
                {
                    Control = new Control(kind) { Reason = Close.Read(reader).Reason },
                };
            default:
                return null;
        }
    }

    private DuplexReader ReadBody(int frameEnd) =>
        new DuplexReader(reader.Buffer, reader.Offset, frameEnd) { Minor = reader.Minor };

    public static FrameHead? ReadHead(byte[] bytes)
    {
        var reader = new DuplexReader(bytes);
        try
        {
            var head = reader.ReadByte();
            var kind = (uint)(head & FrameLayout.KindMask);
            var length =
                ((head & FrameLayout.LastFrame) == 0 ? reader.ReadLength() : reader.UnreadLength)
                + reader.Offset;
            if (!FrameLayout.IsKnown(kind))
            {
                return null;
            }
            var frameKind = (FrameKind)kind;
            return new FrameHead(frameKind, length, ReadId(frameKind, reader));
        }
        catch (MalformedException)
        {
            return null;
        }
    }

    private static uint? ReadId(FrameKind kind, DuplexReader reader)
    {
        try
        {
            if (kind != FrameKind.Message)
            {
                return FrameLayout.IsControl(kind) || kind == FrameKind.Fragment
                    ? null
                    : reader.ReadU32();
            }
            var key = reader.ReadU32();
            return key % 2 == 1 ? reader.ReadU32() : null;
        }
        catch (MalformedException)
        {
            return null;
        }
    }
}
