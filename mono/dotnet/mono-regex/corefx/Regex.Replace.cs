



using System.Collections.Generic;
using System.IO;
using System.Text;

namespace MonoCorefx.Text.RegularExpressions
{

    public delegate string MatchEvaluator(Match match);

    public partial class Regex
    {
        public static string Replace(string input, string pattern, string replacement)
        {
            return Replace(input, pattern, replacement, RegexOptions.None, s_defaultMatchTimeout);
        }

        public static string Replace(string input, string pattern, string replacement, RegexOptions options)
        {
            return Replace(input, pattern, replacement, options, s_defaultMatchTimeout);
        }

        public static string Replace(string input, string pattern, string replacement, RegexOptions options, TimeSpan matchTimeout)
        {
            return new Regex(pattern, options, matchTimeout, true).Replace(input, replacement);
        }

        public string Replace(string input, string replacement)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Replace(input, replacement, -1, UseOptionR() ? input.Length : 0);
        }

        public string Replace(string input, string replacement, int count)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Replace(input, replacement, count, UseOptionR() ? input.Length : 0);
        }

        public string Replace(string input, string replacement, int count, int startat)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            if (replacement == null)
                throw new ArgumentNullException(nameof(replacement));


            RegexReplacement repl = RegexReplacement.GetOrCreate(_replref, replacement, caps, capsize, capnames, roptions);

            return repl.Replace(this, input, count, startat);
        }

        public static string Replace(string input, string pattern, MatchEvaluator evaluator)
        {
            return Replace(input, pattern, evaluator, RegexOptions.None, s_defaultMatchTimeout);
        }

        public static string Replace(string input, string pattern, MatchEvaluator evaluator, RegexOptions options)
        {
            return Replace(input, pattern, evaluator, options, s_defaultMatchTimeout);
        }

        public static string Replace(string input, string pattern, MatchEvaluator evaluator, RegexOptions options, TimeSpan matchTimeout)
        {
            return new Regex(pattern, options, matchTimeout, true).Replace(input, evaluator);
        }

        public string Replace(string input, MatchEvaluator evaluator)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Replace(input, evaluator, -1, UseOptionR() ? input.Length : 0);
        }

        public string Replace(string input, MatchEvaluator evaluator, int count)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Replace(input, evaluator, count, UseOptionR() ? input.Length : 0);
        }

        public string Replace(string input, MatchEvaluator evaluator, int count, int startat)
        {
            if (input == null)
                throw new ArgumentNullException(nameof(input));

            return Replace(evaluator, this, input, count, startat);
        }

        private static string Replace(MatchEvaluator evaluator, Regex regex, string input, int count, int startat)
        {
            if (evaluator == null)
                throw new ArgumentNullException(nameof(evaluator));
            if (count < -1)
                throw new ArgumentOutOfRangeException(nameof(count), SR.CountTooSmall);
            if (startat < 0 || startat > input.Length)
                throw new ArgumentOutOfRangeException(nameof(startat), SR.BeginIndexNotNegative);

            if (count == 0)
                return input;

            Match match = regex.Match(input, startat);

            if (!match.Success)
            {
                return input;
            }
            else
            {
                StringBuilder sb = StringBuilderCache.Acquire();

                if (!regex.RightToLeft)
                {
                    int prevat = 0;

                    do
                    {
                        if (match.Index != prevat)
                            sb.Append(input, prevat, match.Index - prevat);

                        prevat = match.Index + match.Length;

                        sb.Append(evaluator(match));

                        if (--count == 0)
                            break;

                        match = match.NextMatch();
                    } while (match.Success);

                    if (prevat < input.Length)
                        sb.Append(input, prevat, input.Length - prevat);
                }
                else
                {
                    List<string> al = new List<string>();
                    int prevat = input.Length;

                    do
                    {
                        if (match.Index + match.Length != prevat)
                            al.Add(input.Substring(match.Index + match.Length, prevat - match.Index - match.Length));

                        prevat = match.Index;

                        al.Add(evaluator(match));

                        if (--count == 0)
                            break;

                        match = match.NextMatch();
                    } while (match.Success);

                    if (prevat > 0)
                        sb.Append(input, 0, prevat);

                    for (int i = al.Count - 1; i >= 0; i--)
                    {
                        sb.Append(al[i]);
                    }
                }

                return StringBuilderCache.GetStringAndRelease(sb);
            }
        }
    }
}
