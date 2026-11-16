using System.Collections.Generic;
using System.Runtime.CompilerServices;
using Miniscript;
using Newtonsoft.Json;

namespace GreyHackMessageHook
{
    public static class DebugValueTree
    {
        private const int Depth = 3;

        private const int ItemLimit = 200;

        private const int TextDepth = 4;

        private const int TextLength = 512;

        public class Entry
        {
            [JsonProperty("name")]
            public string Name;

            [JsonProperty("accessor")]
            public string Accessor;

            [JsonProperty("value")]
            public Node Value;
        }

        public class Node
        {
            [JsonProperty("typeName")]
            public string TypeName;

            [JsonProperty("text")]
            public string Text;

            [JsonProperty("children")]
            public List<Entry> Children = new List<Entry>();

            [JsonProperty("truncated")]
            public bool Truncated;
        }

        public static List<Entry> ToEntries(ValMap map)
        {
            if (map == null)
            {
                return new List<Entry>();
            }
            return WriteChildren(map, Depth, new HashSet<object>(new IdentityComparer()));
        }

        public static Node ToNode(Value value)
        {
            return ToNode(value, Depth, new HashSet<object>(new IdentityComparer()));
        }

        private static Node ToNode(Value value, int depth, HashSet<object> ancestors)
        {
            var node = new Node { TypeName = GetTypeName(value), Text = GetText(value) };
            if (!(value is ValList) && !(value is ValMap))
            {
                return node;
            }
            if (depth == 0 || ancestors.Contains(value))
            {
                node.Truncated = true;
                return node;
            }
            ancestors.Add(value);
            node.Children = WriteChildren(value, depth - 1, ancestors);
            ancestors.Remove(value);
            return node;
        }

        private static List<Entry> WriteChildren(Value value, int depth, HashSet<object> ancestors)
        {
            var entries = new List<Entry>();
            if (value is ValList list)
            {
                for (int index = 0; index < list.values.Count && index < ItemLimit; index++)
                {
                    entries.Add(
                        new Entry
                        {
                            Name = index.ToString(
                                System.Globalization.CultureInfo.InvariantCulture
                            ),
                            Accessor =
                                "["
                                + index.ToString(System.Globalization.CultureInfo.InvariantCulture)
                                + "]",
                            Value = ToNode(list.values[index], depth, ancestors),
                        }
                    );
                }
            }
            else if (value is ValMap map)
            {
                foreach (KeyValuePair<Value, Value> pair in map.map)
                {
                    if (entries.Count == ItemLimit)
                    {
                        break;
                    }
                    if (pair.Key == null)
                    {
                        continue;
                    }
                    entries.Add(
                        new Entry
                        {
                            Name = pair.Key is ValString key ? key.value : GetText(pair.Key),
                            Accessor = GetAccessor(pair.Key),
                            Value = ToNode(pair.Value, depth, ancestors),
                        }
                    );
                }
            }
            return entries;
        }

        private static string GetAccessor(Value key)
        {
            if (key is ValString text)
            {
                return "[\"" + text.value.Replace("\"", "\"\"") + "\"]";
            }
            if (key is ValNumber)
            {
                return "[" + key.CodeForm(null, 1) + "]";
            }
            return null;
        }

        private static string GetTypeName(Value value)
        {
            switch (value)
            {
                case null:
                case ValNull _:
                    return "null";
                case ValNumber _:
                    return "number";
                case ValString _:
                    return "string";
                case ValList _:
                    return "list";
                case ValMap _:
                    return "map";
                case ValFunction _:
                    return "function";
                default:
                    return value.GetType().Name;
            }
        }

        private static string GetText(Value value)
        {
            string text = value == null ? "null" : value.CodeForm(null, TextDepth);
            return text.Length > TextLength ? text.Substring(0, TextLength) + "..." : text;
        }

        private sealed class IdentityComparer : IEqualityComparer<object>
        {
            public new bool Equals(object left, object right) => ReferenceEquals(left, right);

            public int GetHashCode(object value) => RuntimeHelpers.GetHashCode(value);
        }
    }
}
