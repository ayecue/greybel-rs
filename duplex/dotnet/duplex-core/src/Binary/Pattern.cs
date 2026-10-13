#nullable enable
using System;

namespace Duplex;

public sealed class Pattern
{
    private const int StateCountOffset = 2;
    private const int SlotCountOffset = 6;
    private const int ClassTableOffset = 10;
    private const int ClassTableLength = 256;

    private readonly byte[] byteClasses;
    private readonly ushort start;
    private readonly uint[] bases;
    private readonly ushort[] next;
    private readonly ushort[] owner;
    private readonly byte[] accepting;

    private Pattern(
        byte[] byteClasses,
        ushort start,
        uint[] bases,
        ushort[] next,
        ushort[] owner,
        byte[] accepting
    )
    {
        this.byteClasses = byteClasses;
        this.start = start;
        this.bases = bases;
        this.next = next;
        this.owner = owner;
        this.accepting = accepting;
    }

    public static Pattern Decode(string base64)
    {
        var bytes = Convert.FromBase64String(base64);
        var reader = new DuplexReader(bytes);
        var start = (ushort)(bytes[0] | (bytes[1] << 8));
        reader.Offset = StateCountOffset;
        var states = (int)reader.ReadFixed32();
        reader.Offset = SlotCountOffset;
        var slots = (int)reader.ReadFixed32();
        var classes = new byte[ClassTableLength];
        Array.Copy(bytes, ClassTableOffset, classes, 0, ClassTableLength);
        reader.Offset = ClassTableOffset + ClassTableLength;
        var bases = new uint[states];
        for (var index = 0; index < states; index++)
        {
            bases[index] = reader.ReadFixed32();
        }
        var next = new ushort[slots];
        for (var index = 0; index < slots; index++)
        {
            next[index] = (ushort)(reader.ReadU8() | (reader.ReadU8() << 8));
        }
        var owner = new ushort[slots];
        for (var index = 0; index < slots; index++)
        {
            owner[index] = (ushort)(reader.ReadU8() | (reader.ReadU8() << 8));
        }
        var accepting = new byte[states];
        Array.Copy(bytes, reader.Offset, accepting, 0, states);
        return new Pattern(classes, start, bases, next, owner, accepting);
    }

    public bool Matches(byte[] bytes, int from, int to)
    {
        var state = start;
        for (var index = from; index < to; index++)
        {
            var slot = bases[state] + byteClasses[bytes[index]];
            if (slot >= owner.Length || owner[slot] != state)
            {
                return false;
            }
            state = next[slot];
        }
        return accepting[state] == 1;
    }
}
