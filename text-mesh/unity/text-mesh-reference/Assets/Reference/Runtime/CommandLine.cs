using System;

namespace TextMeshReference
{
    public static class CommandLine
    {
        public static string GetOptionalArgument(string name)
        {
            var arguments = Environment.GetCommandLineArgs();
            var position = Array.IndexOf(arguments, name);
            return position >= 0 && position + 1 < arguments.Length
                ? arguments[position + 1]
                : null;
        }

        public static string GetRequiredArgument(string name)
        {
            return GetOptionalArgument(name)
                ?? throw new ArgumentException($"missing {name} <value>");
        }
    }
}
