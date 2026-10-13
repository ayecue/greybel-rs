



namespace MonoCorefx.Text.RegularExpressions
{
    public partial class Regex
    {
        public static bool IsMatch(string input, string pattern)
        {
            return IsMatch(input, pattern, RegexOptions.None, s_defaultMatchTimeout);
        }

        public static bool IsMatch(string input, string pattern, RegexOptions options)
        {
            return IsMatch(input, pattern, options, s_defaultMatchTimeout);
        }

        public static bool IsMatch(string input, string pattern, RegexOptions options, TimeSpan matchTimeout)
        {
            return new Regex(pattern, options, matchTimeout, true).IsMatch(input);
        }


        public bool IsMatch(string input)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return IsMatch(input, UseOptionR() ? input.Length : 0);
        }


        public bool IsMatch(string input, int startat)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return (null == Run(true, -1, input, 0, input.Length, startat));
        }

        public static Match Match(string input, string pattern)
        {
            return Match(input, pattern, RegexOptions.None, s_defaultMatchTimeout);
        }

        public static Match Match(string input, string pattern, RegexOptions options)
        {
            return Match(input, pattern, options, s_defaultMatchTimeout);
        }

        public static Match Match(string input, string pattern, RegexOptions options, TimeSpan matchTimeout)
        {
            return new Regex(pattern, options, matchTimeout, true).Match(input);
        }


        public Match Match(string input)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Match(input, UseOptionR() ? input.Length : 0);
        }


        public Match Match(string input, int startat)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Run(false, -1, input, 0, input.Length, startat);
        }


        public Match Match(string input, int beginning, int length)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Run(false, -1, input, beginning, length, UseOptionR() ? beginning + length : beginning);
        }

        public static MatchCollection Matches(string input, string pattern)
        {
            return Matches(input, pattern, RegexOptions.None, s_defaultMatchTimeout);
        }

        public static MatchCollection Matches(string input, string pattern, RegexOptions options)
        {
            return Matches(input, pattern, options, s_defaultMatchTimeout);
        }

        public static MatchCollection Matches(string input, string pattern, RegexOptions options, TimeSpan matchTimeout)
        {
            return new Regex(pattern, options, matchTimeout, true).Matches(input);
        }


        public MatchCollection Matches(string input)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Matches(input, UseOptionR() ? input.Length : 0);
        }


        public MatchCollection Matches(string input, int startat)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return new MatchCollection(this, input, 0, input.Length, startat);
        }
    }
}
