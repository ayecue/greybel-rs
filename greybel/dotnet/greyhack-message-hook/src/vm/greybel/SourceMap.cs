using System.Collections.Generic;
using Newtonsoft.Json;

namespace GreyHackMessageHook.Greybel
{
    /// <summary>
    /// Describes built-code origins in the greybel runtime format. <see cref="Sources"/> contains file names; each
    /// mapping <c>[first, last, source, line]</c> maps built lines <c>first..last</c> to one-based lines
    /// <c>line..</c> in source <c>source</c>.
    /// </summary>
    public class SourceMap
    {
        [JsonProperty("sources")]
        public List<string> Sources = new List<string>();

        [JsonProperty("mappings")]
        public List<int[]> Mappings = new List<int[]>();

        /// <summary>Parses the map in <paramref name="json"/>, or returns null if none exists.</summary>
        public static SourceMap FromJson(string json)
        {
            if (string.IsNullOrEmpty(json))
            {
                return null;
            }

            SourceMap map = JsonConvert.DeserializeObject<SourceMap>(json);

            return map == null || map.Mappings.Count == 0 ? null : map;
        }

        /// <summary>Gets the original file and line for built line <paramref name="line"/> when mapped.</summary>
        public bool TryLocate(int line, out string source, out int sourceLine)
        {
            int low = 0;
            int high = Mappings.Count - 1;

            while (low <= high)
            {
                int middle = (low + high) / 2;
                int[] mapping = Mappings[middle];

                if (line < mapping[0])
                {
                    high = middle - 1;
                }
                else if (line > mapping[1])
                {
                    low = middle + 1;
                }
                else if (mapping[2] >= 0 && mapping[2] < Sources.Count)
                {
                    source = Sources[mapping[2]];
                    sourceLine = mapping[3] + (line - mapping[0]);
                    return true;
                }
                else
                {
                    break;
                }
            }

            source = null;
            sourceLine = line;
            return false;
        }
    }

    /// <summary>Source origins and compilation settings for a plugin-generated run binary.</summary>
    public class ProgramBinary
    {
        public readonly SourceMap SourceMap;

        /// <summary>True for strict GreyScript, in which <c>debugger</c> is an identifier.</summary>
        public readonly bool StrictMode;

        public ProgramBinary(SourceMap sourceMap, bool strictMode)
        {
            SourceMap = sourceMap;
            StrictMode = strictMode;
        }
    }

    /// <summary>
    /// Indexes plugin-generated run binaries by code. When the game compiles one, including during self-launch,
    /// error lines can be attributed to their original files.
    /// </summary>
    public static class ProgramBinaryTable
    {
        private static readonly Dictionary<string, ProgramBinary> binaries =
            new Dictionary<string, ProgramBinary>();

        public static void Register(string code, ProgramBinary binary)
        {
            if (string.IsNullOrEmpty(code))
            {
                return;
            }

            lock (binaries)
            {
                binaries[code] = binary;
            }
        }

        public static void Unregister(string code)
        {
            lock (binaries)
            {
                binaries.Remove(code);
            }
        }

        /// <summary>
        /// Gets the binary compiled from <paramref name="source"/>. The game prefixes launched binary code with the
        /// program <c>params</c> on its first line.
        /// </summary>
        public static ProgramBinary Find(string source)
        {
            if (source == null)
            {
                return null;
            }

            lock (binaries)
            {
                foreach (KeyValuePair<string, ProgramBinary> entry in binaries)
                {
                    if (source.EndsWith(entry.Key, System.StringComparison.Ordinal))
                    {
                        return entry.Value;
                    }
                }
            }

            return null;
        }
    }
}
