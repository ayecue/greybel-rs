namespace MonoFixtures.Output;

internal static class Json
{
    public const string Null = "null";

    public static string FormatString(string? value)
    {
        if (value == null)
        {
            return Null;
        }
        var text = new StringBuilder("\"");
        foreach (char c in value)
        {
            switch (c)
            {
                case '"':
                    text.Append("\\\"");
                    break;
                case '\\':
                    text.Append("\\\\");
                    break;
                case < (char)0x20 or > (char)0x7E:
                    text.Append("\\u")
                        .Append(((int)c).ToString("x4", CultureInfo.InvariantCulture));
                    break;
                default:
                    text.Append(c);
                    break;
            }
        }
        return text.Append('"').ToString();
    }

    public static string FormatBits(double value) =>
        FormatString(
            BitConverter.DoubleToInt64Bits(value).ToString("x16", CultureInfo.InvariantCulture)
        );

    public static string FormatBits(float value) =>
        FormatString(
            BitConverter
                .ToInt32(BitConverter.GetBytes(value), 0)
                .ToString("x8", CultureInfo.InvariantCulture)
        );

    public static string FormatInt(long value) => value.ToString(CultureInfo.InvariantCulture);

    public static string FormatBool(bool value) => value ? "true" : "false";

    public static string FormatArray(IEnumerable<string> items) =>
        "[" + string.Join(", ", items) + "]";

    public static string FormatObject(params (string Name, string Value)[] members) =>
        "{"
        + string.Join(
            ", ",
            members.Select(member => FormatString(member.Name) + ": " + member.Value)
        )
        + "}";

    public static string FormatUnits(string text) =>
        FormatArray(text.Select(unit => FormatInt(unit)));
}
