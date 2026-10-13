#nullable enable
using System;
using System.Collections.Generic;

namespace Duplex;

public sealed class DuplexReader
{
    private const byte Continuation = 0x80;
    private const byte Payload = 0x7f;
    private const int Shift = 7;
    private const int MaxVaruintBytes = 10;

    private readonly byte[] buffer;
    private List<int>? path;

    public DuplexReader(byte[] buffer)
        : this(buffer, 0, buffer.Length) { }

    public DuplexReader(byte[] buffer, int start, int end)
    {
        this.buffer = buffer;
        Offset = start;
        End = end;
    }

    public ushort Minor { get; set; } = DuplexWriter.Newest;

    public bool Validating { get; set; }

    public int Offset { get; internal set; }

    public int End { get; internal set; }

    internal byte[] Buffer => buffer;

    public int UnreadLength => End - Offset;

    public DuplexReader GetRemainder() => new DuplexReader(buffer, Offset, End) { Minor = Minor };

    public void Finish()
    {
        if (Offset != End)
        {
            throw new MalformedException("trailing input");
        }
    }

    public void Skip(int length)
    {
        if (length < 0 || length > End - Offset)
        {
            throw new MalformedException("unexpected end of input");
        }
        Offset += length;
    }

    public byte ReadByte()
    {
        if (Offset >= End)
        {
            throw new MalformedException("unexpected end of input");
        }
        return buffer[Offset++];
    }

    public byte ReadFlags(byte mask)
    {
        var flags = ReadByte();
        if ((flags & ~mask) != 0)
        {
            throw new MalformedException("invalid flags");
        }
        return flags;
    }

    public bool ReadBool()
    {
        var value = ReadByte();
        if (value > 1)
        {
            throw new MalformedException("invalid boolean");
        }
        return value == 1;
    }

    public ulong ReadVaruint()
    {
        ulong value = 0;
        for (var index = 0; index < MaxVaruintBytes; index++)
        {
            var current = ReadByte();
            if (index == MaxVaruintBytes - 1 && current > 1)
            {
                throw new MalformedException("a varuint past 64 bits");
            }
            value |= (ulong)(current & Payload) << (Shift * index);
            if ((current & Continuation) == 0)
            {
                return value;
            }
        }
        throw new MalformedException("a varuint past 64 bits");
    }

    private ulong ReadVaruint(ulong max)
    {
        var value = ReadVaruint();
        if (value > max)
        {
            throw new MalformedException("a number too large");
        }
        return value;
    }

    public int ReadLength() => (int)ReadVaruint(int.MaxValue);

    public string ReadString()
    {
        var length = ReadLength();
        var start = Offset;
        Skip(length);
        return Decode(start, length);
    }

    private string Decode(int start, int length)
    {
        try
        {
            return DuplexWriter.Utf8.GetString(buffer, start, length);
        }
        catch (ArgumentException error)
        {
            throw new MalformedException($"a string that isn't UTF-8: {error.Message}");
        }
    }

    public void Enter(int position)
    {
        if (Validating)
        {
            path = FieldValidation.Enter(path, position);
        }
    }

    public void Advance()
    {
        if (Validating)
        {
            FieldValidation.Advance(path);
        }
    }

    public void Exit()
    {
        if (Validating && path != null && path.Count > 0)
        {
            path.RemoveAt(path.Count - 1);
        }
    }

    public int ReadValidatedLength(long min, long max)
    {
        var length = ReadLength();
        if (Validating)
        {
            FieldValidation.ValidateLength(path, length, min, max);
        }
        return length;
    }

    public string ReadValidatedString(long min, long max, Pattern? pattern)
    {
        if (!Validating)
        {
            return ReadString();
        }
        var length = ReadLength();
        FieldValidation.ValidateLength(path, length, min, max);
        var start = Offset;
        Skip(length);
        FieldValidation.ValidateMatch(path, pattern, buffer, start, Offset);
        return Decode(start, length);
    }

    public byte[] ReadValidatedBytes(long min, long max)
    {
        if (!Validating)
        {
            return ReadBytes();
        }
        var length = ReadLength();
        FieldValidation.ValidateLength(path, length, min, max);
        return Take(length);
    }

    public long ValidateRange(long value, long min, long max) =>
        Validating ? FieldValidation.ValidateRange(path, value, min, max) : value;

    public ulong ValidateRange(ulong value, ulong min, ulong max) =>
        Validating ? FieldValidation.ValidateRange(path, value, min, max) : value;

    public double ValidateRange(double value, double min, double max) =>
        Validating ? FieldValidation.ValidateRange(path, value, min, max) : value;

    public byte[] ReadBytes() => Take(ReadLength());

    private byte[] Take(int length)
    {
        var start = Offset;
        Skip(length);
        var bytes = new byte[length];
        System.Buffer.BlockCopy(buffer, start, bytes, 0, length);
        return bytes;
    }

    public int ReadChar()
    {
        var value = ReadVaruint();
        if (value > 0x10ffff || (value >= 0xd800 && value <= 0xdfff))
        {
            throw new MalformedException("no Unicode scalar value");
        }
        return (int)value;
    }

    public byte ReadU8() => ReadByte();

    public sbyte ReadI8() => unchecked((sbyte)ReadByte());

    public ushort ReadU16() => (ushort)ReadVaruint(ushort.MaxValue);

    public uint ReadU32() => (uint)ReadVaruint(uint.MaxValue);

    public ulong ReadU64() => ReadVaruint();

    public short ReadI16()
    {
        var value = DecodeZigzag(ReadVaruint(uint.MaxValue));
        if (value < short.MinValue || value > short.MaxValue)
        {
            throw new MalformedException("a number too large");
        }
        return (short)value;
    }

    public int ReadI32() => (int)DecodeZigzag(ReadVaruint(uint.MaxValue));

    public long ReadI64() => DecodeZigzag(ReadVaruint());

    private static long DecodeZigzag(ulong value) =>
        unchecked((long)(value >> 1) ^ -(long)(value & 1));

    public float ReadF32() => BitConverter.Int32BitsToSingle(unchecked((int)ReadFixed32()));

    public double ReadF64() => BitConverter.Int64BitsToDouble(unchecked((long)ReadFixed64()));

    public uint ReadFixed32()
    {
        var start = Offset;
        Skip(4);
        return (uint)(
            buffer[start]
            | (buffer[start + 1] << 8)
            | (buffer[start + 2] << 16)
            | (buffer[start + 3] << 24)
        );
    }

    public ulong ReadFixed64()
    {
        var low = ReadFixed32();
        var high = ReadFixed32();
        return ((ulong)high << 32) | low;
    }

    internal byte[] TakeAll()
    {
        var bytes = Take(End - Offset);
        return bytes;
    }
}
