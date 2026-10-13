using System.Collections.Generic;
using System.Text.RegularExpressions;

namespace TextMeshReference
{
    public static class TerminalLineContainer
    {
        private const int MaxLineLength = 256;
        private const int ChunkLineSize = 180;
        private const int ChunkMultiplier = 5;
        private const int TagCharacterLimit = 2048;

        private static readonly Regex Tags = new Regex("<.*?>", RegexOptions.Compiled);

        public static List<string> Split(string rawText)
        {
            var lines = new List<string>();
            foreach (var line in rawText.Replace("\r\n", "\n").Replace("\r", "\n").Split('\n'))
            {
                if (line.Length <= MaxLineLength)
                {
                    lines.Add(line);
                    continue;
                }
                var length = line.Length;
                var strippedLine = Tags.Replace(line, string.Empty);
                var tagHeavy = length - strippedLine.Length > TagCharacterLimit;
                if (strippedLine.Length <= MaxLineLength && !tagHeavy)
                {
                    lines.Add(line);
                    continue;
                }
                var chunk = ChunkLineSize;
                if (!tagHeavy && strippedLine.Length < length / 2)
                {
                    chunk *= ChunkMultiplier;
                }
                for (var start = 0; start < length; start += chunk)
                {
                    lines.Add(line.Substring(start, System.Math.Min(chunk, length - start)));
                }
            }
            return lines;
        }
    }
}
