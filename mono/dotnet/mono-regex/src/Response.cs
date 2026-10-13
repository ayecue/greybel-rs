using MonoCorefx.Text.RegularExpressions;

namespace MonoRegex;

internal sealed record Response(Status Status, bool Matched, string Text, List<int> Numbers)
{
    public static Response CreateTimeout() => new(Status.Timeout, false, null, null);

    public static Response CreateFailure(string message) =>
        new(Status.Failure, false, message, null);

    public static Response CreateMatch(bool matched) => new(Status.Ok, matched, null, null);

    public static Response CreateReplacement(string text) => new(Status.Ok, false, text, null);

    public static Response CreateMatches(MatchCollection matches)
    {
        var text = new StringBuilder();
        var numbers = new List<int>();
        foreach (Match match in matches)
        {
            text.Append(match.Value);
            numbers.Add(match.Index);
            numbers.Add(match.Length);
        }
        return new(Status.Ok, false, text.ToString(), numbers);
    }

    public static Response CreateSplit(string[] parts)
    {
        var numbers = new List<int>(parts.Length);
        foreach (string part in parts)
        {
            numbers.Add(part.Length);
        }
        return new(Status.Ok, false, string.Concat(parts), numbers);
    }
}
