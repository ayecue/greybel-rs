namespace GreyHackTestSupport.Game;

/// <summary>Reads the game log (<c>-logFile</c>) while the game writes it.</summary>
internal static class GameLog
{
    /// <summary>Log marker emitted by the test support plugin after the player desktop boots.</summary>
    public const string DesktopReady = "GreyHackTestSupport - desktop ready";

    /// <summary>Number of trailing log lines included in a failure.</summary>
    public const int TailLength = 80;

    public static bool Contains(string path, string line) => Read(path).Contains(line);

    /// <summary>Gets the last <paramref name="count"/> log lines, excluding Unity shader warnings.</summary>
    public static IReadOnlyList<string> ReadTail(string path, int count)
    {
        string[] lines = Read(path)
            .Split('\n')
            .Select(line => line.TrimEnd('\r'))
            .Where(line =>
                !line.Contains("Shader")
                && !line.Contains("#pragma")
                && !line.Contains("subshaders")
            )
            .ToArray();
        return lines[Math.Max(0, lines.Length - count)..];
    }

    /// <summary>Game failure exception containing trailing log output.</summary>
    public static TestSupportException CreateFailure(string failure, string path)
    {
        IEnumerable<string> lines =
        [
            failure,
            $"the end of the game's log ({path}):",
            .. ReadTail(path, TailLength),
        ];
        return new TestSupportException(string.Join('\n', lines));
    }

    private static string Read(string path)
    {
        try
        {
            using var stream = new FileStream(
                path,
                FileMode.Open,
                FileAccess.Read,
                FileShare.ReadWrite | FileShare.Delete
            );
            using var reader = new StreamReader(stream);
            return reader.ReadToEnd();
        }
        catch (Exception error) when (error is FileNotFoundException or DirectoryNotFoundException)
        {
            return "";
        }
    }
}
