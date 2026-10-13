using System.Reflection;
using Duplex;

namespace GreyHackMessageHook.Tests.Support;

internal sealed class Filler
{
    private int next = 1;

    public static Duplex.Message CreateMessage(MessageType type)
    {
        var filler = new Filler();
        return type.Create(type.Fields.Select(field => filler.CreateValue(field.Type)).ToArray());
    }

    private object CreateValue(Type type)
    {
        int n = next++;
        if (type == typeof(string))
            return "text" + n;
        if (type == typeof(int))
            return n;
        if (type == typeof(long))
            return 1_000_000_000_000L + n;
        if (type == typeof(ulong))
            return 2_000_000_000_000UL + (ulong)n;
        if (type == typeof(bool))
            return n % 2 == 0;
        if (type == typeof(float))
            return n + 0.5f;
        if (type == typeof(byte[]))
            return new[] { (byte)n, (byte)(n + 1) };
        if (type == typeof(double))
            return n + 0.25;
        if (type == typeof(IReadOnlyList<int>))
            return new[] { n, n + 100 };
        if (type == typeof(IReadOnlyList<string>))
            return new[] { "item" + n, "item" + (n + 100) };
        if (Nullable.GetUnderlyingType(type) is Type underlying)
            return CreateValue(underlying);
        if (type.IsEnum)
        {
            Array members = Enum.GetValues(type);
            return members.GetValue(n % members.Length)!;
        }

        var shape = (StructType)(
            type.GetField(nameof(StructType), BindingFlags.Public | BindingFlags.Static)
                ?.GetValue(null)
            ?? throw new ArgumentException($"{type.Name} is no struct of the protocol")
        );
        return shape.Create(shape.Fields.Select(field => CreateValue(field.Type)).ToArray());
    }
}
