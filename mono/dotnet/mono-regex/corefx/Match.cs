


























using System.Collections;
using System.Globalization;

namespace MonoCorefx.Text.RegularExpressions
{
    [Serializable]
    public class Match : Group
    {
        internal GroupCollection _groupcoll;


        internal Regex _regex;
        internal int _textbeg;
        internal int _textpos;
        internal int _textend;
        internal int _textstart;


        internal int[][] _matches;
        internal int[] _matchcount;
        internal bool _balancing;


        internal Match(Regex regex, int capcount, string text, int begpos, int len, int startpos)
            : base(text, new int[2], 0, "0")
        {
            _regex = regex;
            _matchcount = new int[capcount];
            _matches = new int[capcount][];
            _matches[0] = _caps;
            _textbeg = begpos;
            _textend = begpos + len;
            _textstart = startpos;
            _balancing = false;


            System.Diagnostics.Debug.Assert(!(_textbeg < 0 || _textstart < _textbeg || _textend < _textstart || Text.Length < _textend),
                                            "The parameters are out of range.");
        }

        public static Match Empty { get; } = new Match(null, 1, string.Empty, 0, 0, 0);

        internal virtual void Reset(Regex regex, string text, int textbeg, int textend, int textstart)
        {
            _regex = regex;
            Text = text;
            _textbeg = textbeg;
            _textend = textend;
            _textstart = textstart;

            for (int i = 0; i < _matchcount.Length; i++)
            {
                _matchcount[i] = 0;
            }

            _balancing = false;
        }

        public virtual GroupCollection Groups
        {
            get
            {
                if (_groupcoll == null)
                    _groupcoll = new GroupCollection(this, null);

                return _groupcoll;
            }
        }

        public Match NextMatch()
        {
            if (_regex == null)
                return this;

            return _regex.Run(false, Length, Text, _textbeg, _textend - _textbeg, _textpos);
        }

        public virtual string Result(string replacement)
        {
            if (replacement == null)
                throw new ArgumentNullException(nameof(replacement));

            if (_regex == null)
                throw new NotSupportedException(SR.NoResultOnFailed);


            RegexReplacement repl = RegexReplacement.GetOrCreate(_regex._replref, replacement, _regex.caps, _regex.capsize,
                _regex.capnames, _regex.roptions);

            return repl.Replacement(this);
        }

        internal virtual ReadOnlySpan<char> GroupToStringImpl(int groupnum)
        {
            int c = _matchcount[groupnum];
            if (c == 0)
                return string.Empty;

            int[] matches = _matches[groupnum];

            return Text.AsSpan(matches[(c - 1) * 2], matches[(c * 2) - 1]);
        }

        internal ReadOnlySpan<char> LastGroupToStringImpl()
        {
            return GroupToStringImpl(_matchcount.Length - 1);
        }

        public static Match Synchronized(Match inner)
        {
            if (inner == null)
                throw new ArgumentNullException(nameof(inner));

            int numgroups = inner._matchcount.Length;


            for (int i = 0; i < numgroups; i++)
            {
                Group group = inner.Groups[i];



                Group.Synchronized(group);
            }

            return inner;
        }

        internal virtual void AddMatch(int cap, int start, int len)
        {
            int capcount;

            if (_matches[cap] == null)
                _matches[cap] = new int[2];

            capcount = _matchcount[cap];

            if (capcount * 2 + 2 > _matches[cap].Length)
            {
                int[] oldmatches = _matches[cap];
                int[] newmatches = new int[capcount * 8];
                for (int j = 0; j < capcount * 2; j++)
                    newmatches[j] = oldmatches[j];
                _matches[cap] = newmatches;
            }

            _matches[cap][capcount * 2] = start;
            _matches[cap][capcount * 2 + 1] = len;
            _matchcount[cap] = capcount + 1;
        }


        internal virtual void BalanceMatch(int cap)
        {
            _balancing = true;


            int capcount = _matchcount[cap];
            int target = capcount * 2 - 2;



            if (_matches[cap][target] < 0)
                target = -3 - _matches[cap][target];


            target -= 2;


            if (target >= 0 && _matches[cap][target] < 0)
                AddMatch(cap, _matches[cap][target], _matches[cap][target + 1]);
            else
                AddMatch(cap, -3 - target, -4 - target);
        }

        internal virtual void RemoveMatch(int cap)
        {
            _matchcount[cap]--;
        }

        internal virtual bool IsMatched(int cap)
        {
            return cap < _matchcount.Length && _matchcount[cap] > 0 && _matches[cap][_matchcount[cap] * 2 - 1] != (-3 + 1);
        }

        internal virtual int MatchIndex(int cap)
        {
            int i = _matches[cap][_matchcount[cap] * 2 - 2];
            if (i >= 0)
                return i;

            return _matches[cap][-3 - i];
        }

        internal virtual int MatchLength(int cap)
        {
            int i = _matches[cap][_matchcount[cap] * 2 - 1];
            if (i >= 0)
                return i;

            return _matches[cap][-3 - i];
        }

        internal virtual void Tidy(int textpos)
        {
            int[] interval = _matches[0];
            Index = interval[0];
            Length = interval[1];
            _textpos = textpos;
            _capcount = _matchcount[0];

            if (_balancing)
            {






                for (int cap = 0; cap < _matchcount.Length; cap++)
                {
                    int limit;
                    int[] matcharray;

                    limit = _matchcount[cap] * 2;
                    matcharray = _matches[cap];

                    int i = 0;
                    int j;

                    for (i = 0; i < limit; i++)
                    {
                        if (matcharray[i] < 0)
                            break;
                    }

                    for (j = i; i < limit; i++)
                    {
                        if (matcharray[i] < 0)
                        {

                            j--;
                        }
                        else
                        {


                            if (i != j)
                                matcharray[j] = matcharray[i];
                            j++;
                        }
                    }

                    _matchcount[cap] = j / 2;
                }

                _balancing = false;
            }
        }

#if DEBUG
        internal bool Debug
        {
            get
            {
                if (_regex == null)
                    return false;

                return _regex.Debug;
            }
        }

        internal virtual void Dump()
        {
            int i, j;

            for (i = 0; i < _matchcount.Length; i++)
            {
                System.Diagnostics.Debug.WriteLine("Capnum " + i.ToString(CultureInfo.InvariantCulture) + ":");

                for (j = 0; j < _matchcount[i]; j++)
                {
                    string text = "";

                    if (_matches[i][j * 2] >= 0)
                        text = Text.Substring(_matches[i][j * 2], _matches[i][j * 2 + 1]);

                    System.Diagnostics.Debug.WriteLine("  (" + _matches[i][j * 2].ToString(CultureInfo.InvariantCulture) + "," + _matches[i][j * 2 + 1].ToString(CultureInfo.InvariantCulture) + ") " + text);
                }
            }
        }
#endif
    }

    internal class MatchSparse : Match
    {

        new internal readonly Hashtable _caps;

        internal MatchSparse(Regex regex, Hashtable caps, int capcount, string text, int begpos, int len, int startpos)
            : base(regex, capcount, text, begpos, len, startpos)
        {
            _caps = caps;
        }

        public override GroupCollection Groups
        {
            get
            {
                if (_groupcoll == null)
                    _groupcoll = new GroupCollection(this, _caps);

                return _groupcoll;
            }
        }

#if DEBUG
        internal override void Dump()
        {
            if (_caps != null)
            {
                foreach (DictionaryEntry kvp in _caps)
                {
                    System.Diagnostics.Debug.WriteLine("Slot " + kvp.Key.ToString() + " -> " + kvp.Value.ToString());
                }
            }

            base.Dump();
        }
#endif
    }
}
