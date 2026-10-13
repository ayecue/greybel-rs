using System.Runtime.InteropServices;

namespace MonoRegex;

[StructLayout(LayoutKind.Sequential)]
public unsafe struct Result
{
    public Status Status;
    public int IsMatch;
    public char* Text;
    public int TextLength;
    public int* Numbers;
    public int NumbersLength;

    internal static Result From(Response response)
    {
        var result = new Result { Status = response.Status, IsMatch = response.Matched ? 1 : 0 };
        if (response.Text is { } text)
        {
            result.Text = (char*)NativeMemory.Alloc((nuint)text.Length, sizeof(char));
            text.AsSpan().CopyTo(new Span<char>(result.Text, text.Length));
            result.TextLength = text.Length;
        }
        if (response.Numbers is { } numbers)
        {
            result.Numbers = (int*)NativeMemory.Alloc((nuint)numbers.Count, sizeof(int));
            CollectionsMarshal.AsSpan(numbers).CopyTo(new Span<int>(result.Numbers, numbers.Count));
            result.NumbersLength = numbers.Count;
        }
        return result;
    }

    internal void Release()
    {
        NativeMemory.Free(Text);
        NativeMemory.Free(Numbers);
        this = default;
    }
}
