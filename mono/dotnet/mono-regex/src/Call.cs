using MonoCorefx.Text.RegularExpressions;

namespace MonoRegex;

internal readonly unsafe struct Call(
    char* input,
    int inputLength,
    char* pattern,
    int patternLength,
    int options,
    int timeoutMilliseconds
)
{
    public string Input { get; } = new(input, 0, inputLength);

    public string Pattern { get; } = new(pattern, 0, patternLength);

    public RegexOptions Options { get; } = (RegexOptions)options;

    public TimeSpan Timeout { get; } = TimeSpan.FromMilliseconds(timeoutMilliseconds);
}
