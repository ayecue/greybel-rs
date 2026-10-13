#nullable enable
using System;
using System.Collections.Generic;
using System.Text;

namespace Duplex;

public sealed class DuplexWriter
{
    public const ushort Newest = ushort.MaxValue;

    private const int GrowthFactor = 2;
    private const byte Continuation = 0x80;
    private const byte Payload = 0x7f;
    private const int Shift = 7;

    internal static readonly UTF8Encoding Utf8 = new UTF8Encoding(false, true);

    private byte[] buffer;
    private int offset;
    private List<int>? path;

    public DuplexWriter(int capacity = 256)
    {
        buffer = new byte[Math.Max(capacity, 16)];
    }

    public ushort Minor { get; set; } = Newest;

    public bool Validating { get; private set; }

    public int Size => offset;

    internal byte[] Buffer => buffer;

    private void Reserve(int length)
    {
        var end = offset + length;
        if (end <= buffer.Length)
        {
            return;
        }
        var capacity = buffer.Length;
        while (capacity < end)
        {
            capacity *= GrowthFactor;
        }
        Array.Resize(ref buffer, capacity);
    }

    public void WriteByte(byte value)
    {
        Reserve(1);
        buffer[offset++] = value;
    }

    public void WriteRaw(byte[] value) => WriteRaw(value, 0, value.Length);

    public void WriteRaw(byte[] source, int start, int end)
    {
        var length = end - start;
        Reserve(length);
        System.Buffer.BlockCopy(source, start, buffer, offset, length);
        offset += length;
    }

    public void Validate(bool validating)
    {
        Validating = validating;
        path?.Clear();
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

    public void WriteValidatedLength(int length, long min, long max)
    {
        if (Validating)
        {
            FieldValidation.ValidateLength(path, length, min, max);
        }
        WriteLength(length);
    }

    public void WriteValidatedString(string value, long min, long max, Pattern? pattern)
    {
        var bytes = Utf8.GetBytes(value);
        if (Validating)
        {
            FieldValidation.ValidateLength(path, bytes.Length, min, max);
            FieldValidation.ValidateMatch(path, pattern, bytes, 0, bytes.Length);
        }
        WriteLength(bytes.Length);
        WriteRaw(bytes);
    }

    public void WriteValidatedBytes(byte[] value, long min, long max)
    {
        if (Validating)
        {
            FieldValidation.ValidateLength(path, value.Length, min, max);
        }
        WriteBytes(value);
    }

    public long ValidateRange(long value, long min, long max) =>
        Validating ? FieldValidation.ValidateRange(path, value, min, max) : value;

    public ulong ValidateRange(ulong value, ulong min, ulong max) =>
        Validating ? FieldValidation.ValidateRange(path, value, min, max) : value;

    public double ValidateRange(double value, double min, double max) =>
        Validating ? FieldValidation.ValidateRange(path, value, min, max) : value;

    public void WriteBytes(byte[] value)
    {
        WriteLength(value.Length);
        WriteRaw(value);
    }

    public void WriteBool(bool value) => WriteByte(value ? (byte)1 : (byte)0);

    public void WriteVaruint(ulong value)
    {
        Reserve(10);
        while (value >= Continuation)
        {
            buffer[offset++] = (byte)((value & Payload) | Continuation);
            value >>= Shift;
        }
        buffer[offset++] = (byte)value;
    }

    public void WriteLength(int length)
    {
        if (length < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(length));
        }
        WriteVaruint((ulong)length);
    }

    public void WriteString(string value)
    {
        var bytes = Utf8.GetBytes(value);
        WriteLength(bytes.Length);
        WriteRaw(bytes);
    }

    public void WriteChar(int value)
    {
        if (value < 0 || value > 0x10ffff || (value >= 0xd800 && value <= 0xdfff))
        {
            throw new ArgumentOutOfRangeException(
                nameof(value),
                $"{value} is no Unicode scalar value"
            );
        }
        WriteVaruint((ulong)value);
    }

    public void WriteU8(byte value) => WriteByte(value);

    public void WriteI8(sbyte value) => WriteByte(unchecked((byte)value));

    public void WriteU16(ushort value) => WriteVaruint(value);

    public void WriteU32(uint value) => WriteVaruint(value);

    public void WriteU64(ulong value) => WriteVaruint(value);

    public void WriteI16(short value) =>
        WriteVaruint(unchecked((uint)((value << 1) ^ (value >> 31))));

    public void WriteI32(int value) =>
        WriteVaruint(unchecked((uint)((value << 1) ^ (value >> 31))));

    public void WriteI64(long value) =>
        WriteVaruint(unchecked((ulong)((value << 1) ^ (value >> 63))));

    public void WriteF32(float value) =>
        WriteFixed32(unchecked((uint)BitConverter.SingleToInt32Bits(value)));

    public void WriteF64(double value) =>
        WriteFixed64(unchecked((ulong)BitConverter.DoubleToInt64Bits(value)));

    public void WriteFixed32(uint value)
    {
        Reserve(4);
        for (var index = 0; index < 4; index++)
        {
            buffer[offset++] = (byte)(value >> (index * 8));
        }
    }

    public void WriteFixed64(ulong value)
    {
        WriteFixed32((uint)value);
        WriteFixed32((uint)(value >> 32));
    }

    internal void FinishLength(int position)
    {
        var length = offset - position - 1;
        if (length < Continuation)
        {
            buffer[position] = (byte)length;
            return;
        }
        var size = 1;
        for (var bits = length >> Shift; bits > 0; bits >>= Shift)
        {
            size++;
        }
        var end = offset;
        Reserve(size - 1);
        System.Buffer.BlockCopy(buffer, position + 1, buffer, position + size, end - position - 1);
        var remainder = length;
        for (var index = position; index < position + size - 1; index++)
        {
            buffer[index] = (byte)((remainder & Payload) | Continuation);
            remainder >>= Shift;
        }
        buffer[position + size - 1] = (byte)remainder;
        offset = end + size - 1;
    }

    internal void Truncate(int length) => offset = Math.Min(offset, length);

    public void Reset() => offset = 0;

    public byte[] ToArray()
    {
        var bytes = new byte[offset];
        System.Buffer.BlockCopy(buffer, 0, bytes, 0, offset);
        return bytes;
    }
}
