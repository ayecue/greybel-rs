



namespace MonoCorefx.Text.RegularExpressions
{
    internal readonly struct RegexPrefix
    {
        internal RegexPrefix(string prefix, bool ci)
        {
            Prefix = prefix;
            CaseInsensitive = ci;
        }

        internal bool CaseInsensitive { get; }

        internal static RegexPrefix Empty { get; } = new RegexPrefix(string.Empty, false);

        internal string Prefix { get; }
    }
}
