












using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text;

namespace MonoCorefx.Text.RegularExpressions
{
    internal sealed class RegexBoyerMoore
    {
        public readonly int[] Positive;
        public readonly int[] NegativeASCII;
        public readonly int[][] NegativeUnicode;
        public readonly string Pattern;
        public readonly int LowASCII;
        public readonly int HighASCII;
        public readonly bool RightToLeft;
        public readonly bool CaseInsensitive;
        private readonly CultureInfo _culture;

        public RegexBoyerMoore(string pattern, bool caseInsensitive, bool rightToLeft, CultureInfo culture)
        {


            Debug.Assert(pattern.Length != 0, "RegexBoyerMoore called with an empty string. This is bad for perf");





            if (caseInsensitive)
            {
                StringBuilder sb = StringBuilderCache.Acquire(pattern.Length);
                for (int i = 0; i < pattern.Length; i++)
                    sb.Append(culture.TextInfo.ToLower(pattern[i]));
                pattern = StringBuilderCache.GetStringAndRelease(sb);
            }

            Pattern = pattern;
            RightToLeft = rightToLeft;
            CaseInsensitive = caseInsensitive;
            _culture = culture;

            int beforefirst;
            int last;
            int bump;

            if (!rightToLeft)
            {
                beforefirst = -1;
                last = pattern.Length - 1;
                bump = 1;
            }
            else
            {
                beforefirst = pattern.Length;
                last = 0;
                bump = -1;
            }










            Positive = new int[pattern.Length];

            int examine = last;
            char ch = pattern[examine];
            Positive[examine] = bump;
            examine -= bump;
            int scan;
            int match;

            for (; ; )
            {


                for (; ; )
                {
                    if (examine == beforefirst)
                        goto OuterloopBreak;
                    if (pattern[examine] == ch)
                        break;
                    examine -= bump;
                }

                match = last;
                scan = examine;



                for (; ; )
                {
                    if (scan == beforefirst || pattern[match] != pattern[scan])
                    {



                        if (Positive[match] == 0)
                            Positive[match] = match - scan;



                        break;
                    }

                    scan -= bump;
                    match -= bump;
                }

                examine -= bump;
            }

        OuterloopBreak:

            match = last - bump;








            while (match != beforefirst)
            {
                if (Positive[match] == 0)
                    Positive[match] = bump;

                match -= bump;
            }













            NegativeASCII = new int[128];

            for (int i = 0; i < 128; i++)
                NegativeASCII[i] = last - beforefirst;

            LowASCII = 127;
            HighASCII = 0;

            for (examine = last; examine != beforefirst; examine -= bump)
            {
                ch = pattern[examine];

                if (ch < 128)
                {
                    if (LowASCII > ch)
                        LowASCII = ch;

                    if (HighASCII < ch)
                        HighASCII = ch;

                    if (NegativeASCII[ch] == last - beforefirst)
                        NegativeASCII[ch] = last - examine;
                }
                else
                {
                    int i = ch >> 8;
                    int j = ch & 0xFF;

                    if (NegativeUnicode == null)
                    {
                        NegativeUnicode = new int[256][];
                    }

                    if (NegativeUnicode[i] == null)
                    {
                        int[] newarray = new int[256];

                        for (int k = 0; k < newarray.Length; k++)
                            newarray[k] = last - beforefirst;

                        if (i == 0)
                        {
                            Array.Copy(NegativeASCII, 0, newarray, 0, 128);
                            NegativeASCII = newarray;
                        }

                        NegativeUnicode[i] = newarray;
                    }

                    if (NegativeUnicode[i][j] == last - beforefirst)
                        NegativeUnicode[i][j] = last - examine;
                }
            }
        }

        private bool MatchPattern(string text, int index)
        {
            if (CaseInsensitive)
            {
                if (text.Length - index < Pattern.Length)
                {
                    return false;
                }

                TextInfo textinfo = _culture.TextInfo;
                for (int i = 0; i < Pattern.Length; i++)
                {
                    Debug.Assert(textinfo.ToLower(Pattern[i]) == Pattern[i], "pattern should be converted to lower case in constructor!");
                    if (textinfo.ToLower(text[index + i]) != Pattern[i])
                    {
                        return false;
                    }
                }

                return true;
            }
            else
            {
                return (0 == string.CompareOrdinal(Pattern, 0, text, index, Pattern.Length));
            }
        }

        public bool IsMatch(string text, int index, int beglimit, int endlimit)
        {
            if (!RightToLeft)
            {
                if (index < beglimit || endlimit - index < Pattern.Length)
                    return false;

                return MatchPattern(text, index);
            }
            else
            {
                if (index > endlimit || index - beglimit < Pattern.Length)
                    return false;

                return MatchPattern(text, index - Pattern.Length);
            }
        }

        public int Scan(string text, int index, int beglimit, int endlimit)
        {
            int defadv;
            int test;
            int startmatch;
            int endmatch;
            int bump;

            if (!RightToLeft)
            {
                defadv = Pattern.Length;
                startmatch = Pattern.Length - 1;
                endmatch = 0;
                test = index + defadv - 1;
                bump = 1;
            }
            else
            {
                defadv = -Pattern.Length;
                startmatch = 0;
                endmatch = -defadv - 1;
                test = index + defadv;
                bump = -1;
            }

            char chMatch = Pattern[startmatch];
            char chTest;
            int test2;
            int match;
            int advance;
            int[] unicodeLookup;

            for (; ; )
            {
                if (test >= endlimit || test < beglimit)
                    return -1;

                chTest = text[test];

                if (CaseInsensitive)
                    chTest = _culture.TextInfo.ToLower(chTest);

                if (chTest != chMatch)
                {
                    if (chTest < 128)
                        advance = NegativeASCII[chTest];
                    else if (null != NegativeUnicode && (null != (unicodeLookup = NegativeUnicode[chTest >> 8])))
                        advance = unicodeLookup[chTest & 0xFF];
                    else
                        advance = defadv;

                    test += advance;
                }
                else
                {
                    test2 = test;
                    match = startmatch;

                    for (; ; )
                    {
                        if (match == endmatch)
                            return (RightToLeft ? test2 + 1 : test2);

                        match -= bump;
                        test2 -= bump;

                        chTest = text[test2];

                        if (CaseInsensitive)
                            chTest = _culture.TextInfo.ToLower(chTest);

                        if (chTest != Pattern[match])
                        {
                            advance = Positive[match];
                            if ((chTest & 0xFF80) == 0)
                                test2 = (match - startmatch) + NegativeASCII[chTest];
                            else if (null != NegativeUnicode && (null != (unicodeLookup = NegativeUnicode[chTest >> 8])))
                                test2 = (match - startmatch) + unicodeLookup[chTest & 0xFF];
                            else
                            {
                                test += advance;
                                break;
                            }

                            if (RightToLeft ? test2 < advance : test2 > advance)
                                advance = test2;

                            test += advance;
                            break;
                        }
                    }
                }
            }
        }

#if DEBUG
        public override string ToString() => Pattern;

        public string Dump(string indent)
        {
            StringBuilder sb = new StringBuilder();

            sb.Append(indent + "BM Pattern: " + Pattern + "\n");
            sb.Append(indent + "Positive: ");
            for (int i = 0; i < Positive.Length; i++)
            {
                sb.Append(Positive[i].ToString(CultureInfo.InvariantCulture) + " ");
            }
            sb.Append("\n");

            if (NegativeASCII != null)
            {
                sb.Append(indent + "Negative table\n");
                for (int i = 0; i < NegativeASCII.Length; i++)
                {
                    if (NegativeASCII[i] != Pattern.Length)
                    {
                        sb.Append(indent + "  " + Regex.Escape(Convert.ToString((char)i, CultureInfo.InvariantCulture)) + " " + NegativeASCII[i].ToString(CultureInfo.InvariantCulture) + "\n");
                    }
                }
            }

            return sb.ToString();
        }
#endif
    }
}
