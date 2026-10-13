using GreyHackUtil.Contracts;

namespace GreyHackUtil.Protocol;

internal static class ProtocolNameTable
{
    private static readonly HashSet<string> GeneratedMembers =
    [
        "Write",
        "Read",
        "MessageType",
        "GetMessageType",
        "Equals",
        "GetHashCode",
        "ToString",
        "GetType",
    ];

    private const string PropertySuffix = "Field";

    public static string ToTypeName(FieldType fieldType) => fieldType.ShortClrType;

    public static string ToFieldName(string name)
    {
        string snake = ToSnakeCase(name);
        if (
            snake.Length == 0
            || !char.IsAsciiLetterLower(snake[0])
            || snake.Any(c => !(char.IsAsciiLetterLower(c) || char.IsAsciiDigit(c) || c == '_'))
        )
        {
            throw new ContractException($"the field {name} has no snake_case name ({snake})");
        }
        return snake;
    }

    public static string ToMemberName(string name)
    {
        string pascal = string.Concat(
            name.Split('_', StringSplitOptions.RemoveEmptyEntries)
                .Select(part =>
                {
                    string word = part.All(c => !char.IsLetter(c) || char.IsUpper(c))
                        ? part.ToLowerInvariant()
                        : part;
                    return char.ToUpperInvariant(word[0]) + word[1..];
                })
        );
        if (
            pascal.Length == 0
            || !char.IsAsciiLetterUpper(pascal[0])
            || !pascal.All(char.IsAsciiLetterOrDigit)
        )
        {
            throw new ContractException($"the member {name} has no PascalCase name ({pascal})");
        }
        return pascal;
    }

    public static string ToPropertyName(string field, string owner)
    {
        string pascal = string.Concat(
            field
                .Split('_', StringSplitOptions.RemoveEmptyEntries)
                .Select(word => char.ToUpperInvariant(word[0]) + word[1..])
        );
        return pascal == owner || GeneratedMembers.Contains(pascal)
            ? pascal + PropertySuffix
            : pascal;
    }

    public static string ToSnakeCase(string name)
    {
        return string.Join('_', SplitWords(name)).ToLowerInvariant();
    }

    private static IEnumerable<string> SplitWords(string name)
    {
        int start = 0;
        for (int i = 1; i < name.Length; i++)
        {
            char previous = name[i - 1];
            char c = name[i];
            bool boundary =
                char.IsDigit(c) != char.IsDigit(previous)
                || (char.IsUpper(c) && char.IsLower(previous))
                || (
                    char.IsUpper(c)
                    && char.IsUpper(previous)
                    && i + 1 < name.Length
                    && char.IsLower(name[i + 1])
                    && !IsPluralS(name, i + 1)
                );
            if (boundary)
            {
                yield return name[start..i];
                start = i;
            }
        }
        yield return name[start..];
    }

    private static bool IsPluralS(string name, int index)
    {
        return name[index] == 's' && (index + 1 == name.Length || !char.IsLower(name[index + 1]));
    }
}
