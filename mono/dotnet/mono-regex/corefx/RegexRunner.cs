















using System.Diagnostics;
using System.Globalization;

namespace MonoCorefx.Text.RegularExpressions
{
    public abstract class RegexRunner
    {
        protected internal int runtextbeg;
        protected internal int runtextend;
        protected internal int runtextstart;

        protected internal string runtext;
        protected internal int runtextpos;

        protected internal int[] runtrack;
        protected internal int runtrackpos;











        protected internal int[] runstack;
        protected internal int runstackpos;






        protected internal int[] runcrawl;
        protected internal int runcrawlpos;


        protected internal int runtrackcount;

        protected internal Match runmatch;
        protected internal Regex runregex;

        private int _timeout;
        private bool _ignoreTimeout;
        private int _timeoutOccursAt;






        private const int TimeoutCheckFrequency = 1000;
        private int _timeoutChecksToSkip;

        protected internal RegexRunner() { }

        protected internal Match Scan(Regex regex, string text, int textbeg, int textend, int textstart, int prevlen, bool quick)
        {
            return Scan(regex, text, textbeg, textend, textstart, prevlen, quick, regex.MatchTimeout);
        }

        protected internal Match Scan(Regex regex, string text, int textbeg, int textend, int textstart, int prevlen, bool quick, TimeSpan timeout)
        {
            int bump;
            int stoppos;
            bool initted = false;



            Regex.ValidateMatchTimeout(timeout);

            _ignoreTimeout = (Regex.InfiniteMatchTimeout == timeout);
            _timeout = _ignoreTimeout
                                    ? (int)Regex.InfiniteMatchTimeout.TotalMilliseconds
                                    : (int)(timeout.TotalMilliseconds + 0.5);

            runregex = regex;
            runtext = text;
            runtextbeg = textbeg;
            runtextend = textend;
            runtextstart = textstart;

            bump = runregex.RightToLeft ? -1 : 1;
            stoppos = runregex.RightToLeft ? runtextbeg : runtextend;

            runtextpos = textstart;



            if (prevlen == 0)
            {
                if (runtextpos == stoppos)
                    return Match.Empty;

                runtextpos += bump;
            }

            StartTimeoutWatch();

            for (; ; )
            {
#if DEBUG
                if (runregex.Debug)
                {
                    Debug.WriteLine("");
                    Debug.WriteLine("Search range: from " + runtextbeg.ToString(CultureInfo.InvariantCulture) + " to " + runtextend.ToString(CultureInfo.InvariantCulture));
                    Debug.WriteLine("Firstchar search starting at " + runtextpos.ToString(CultureInfo.InvariantCulture) + " stopping at " + stoppos.ToString(CultureInfo.InvariantCulture));
                }
#endif
                if (FindFirstChar())
                {
                    CheckTimeout();

                    if (!initted)
                    {
                        InitMatch();
                        initted = true;
                    }
#if DEBUG
                    if (runregex.Debug)
                    {
                        Debug.WriteLine("Executing engine starting at " + runtextpos.ToString(CultureInfo.InvariantCulture));
                        Debug.WriteLine("");
                    }
#endif
                    Go();

                    if (runmatch._matchcount[0] > 0)
                    {

                        return TidyMatch(quick);
                    }


                    runtrackpos = runtrack.Length;
                    runstackpos = runstack.Length;
                    runcrawlpos = runcrawl.Length;
                }



                if (runtextpos == stoppos)
                {
                    TidyMatch(true);
                    return Match.Empty;
                }





                runtextpos += bump;
            }

        }

        private void StartTimeoutWatch()
        {
            if (_ignoreTimeout)
                return;

            _timeoutChecksToSkip = TimeoutCheckFrequency;





            unchecked
            {
                _timeoutOccursAt = Environment.TickCount + _timeout;
            }
        }

        protected void CheckTimeout()
        {
            if (_ignoreTimeout)
                return;

            DoCheckTimeout();
        }

        private void DoCheckTimeout()
        {
            if (--_timeoutChecksToSkip != 0)
                return;

            _timeoutChecksToSkip = TimeoutCheckFrequency;




            int currentMillis = Environment.TickCount;

            if (currentMillis < _timeoutOccursAt)
                return;

            if (0 > _timeoutOccursAt && 0 < currentMillis)
                return;

#if DEBUG
            if (runregex.Debug)
            {
                Debug.WriteLine("");
                Debug.WriteLine("RegEx match timeout occurred!");
                Debug.WriteLine("Specified timeout:       " + TimeSpan.FromMilliseconds(_timeout).ToString());
                Debug.WriteLine("Timeout check frequency: " + TimeoutCheckFrequency);
                Debug.WriteLine("Search pattern:          " + runregex.pattern);
                Debug.WriteLine("Input:                   " + runtext);
                Debug.WriteLine("About to throw RegexMatchTimeoutException.");
            }
#endif

            throw new RegexMatchTimeoutException(runtext, runregex.pattern, TimeSpan.FromMilliseconds(_timeout));
        }

        protected abstract void Go();

        protected abstract bool FindFirstChar();

        protected abstract void InitTrackCount();

        private void InitMatch()
        {


            if (runmatch == null)
            {
                if (runregex.caps != null)
                    runmatch = new MatchSparse(runregex, runregex.caps, runregex.capsize, runtext, runtextbeg, runtextend - runtextbeg, runtextstart);
                else
                    runmatch = new Match(runregex, runregex.capsize, runtext, runtextbeg, runtextend - runtextbeg, runtextstart);
            }
            else
            {
                runmatch.Reset(runregex, runtext, runtextbeg, runtextend, runtextstart);
            }






            if (runcrawl != null)
            {
                runtrackpos = runtrack.Length;
                runstackpos = runstack.Length;
                runcrawlpos = runcrawl.Length;
                return;
            }

            InitTrackCount();

            int tracksize = runtrackcount * 8;
            int stacksize = runtrackcount * 8;

            if (tracksize < 32)
                tracksize = 32;
            if (stacksize < 16)
                stacksize = 16;

            runtrack = new int[tracksize];
            runtrackpos = tracksize;

            runstack = new int[stacksize];
            runstackpos = stacksize;

            runcrawl = new int[32];
            runcrawlpos = 32;
        }

        private Match TidyMatch(bool quick)
        {
            if (!quick)
            {
                Match match = runmatch;

                runmatch = null;

                match.Tidy(runtextpos);
                return match;
            }
            else
            {



                return null;
            }
        }

        protected void EnsureStorage()
        {
            if (runstackpos < runtrackcount * 4)
                DoubleStack();
            if (runtrackpos < runtrackcount * 4)
                DoubleTrack();
        }

        protected bool IsBoundary(int index, int startpos, int endpos)
        {
            return (index > startpos && RegexCharClass.IsWordChar(runtext[index - 1])) !=
                   (index < endpos && RegexCharClass.IsWordChar(runtext[index]));
        }

        protected bool IsECMABoundary(int index, int startpos, int endpos)
        {
            return (index > startpos && RegexCharClass.IsECMAWordChar(runtext[index - 1])) !=
                   (index < endpos && RegexCharClass.IsECMAWordChar(runtext[index]));
        }

        protected static bool CharInSet(char ch, string set, string category)
        {
            string charClass = RegexCharClass.ConvertOldStringsToClass(set, category);
            return RegexCharClass.CharInClass(ch, charClass);
        }

        protected static bool CharInClass(char ch, string charClass)
        {
            return RegexCharClass.CharInClass(ch, charClass);
        }

        protected void DoubleTrack()
        {
            int[] newtrack;

            newtrack = new int[runtrack.Length * 2];

            Array.Copy(runtrack, 0, newtrack, runtrack.Length, runtrack.Length);
            runtrackpos += runtrack.Length;
            runtrack = newtrack;
        }

        protected void DoubleStack()
        {
            int[] newstack;

            newstack = new int[runstack.Length * 2];

            Array.Copy(runstack, 0, newstack, runstack.Length, runstack.Length);
            runstackpos += runstack.Length;
            runstack = newstack;
        }

        protected void DoubleCrawl()
        {
            int[] newcrawl;

            newcrawl = new int[runcrawl.Length * 2];

            Array.Copy(runcrawl, 0, newcrawl, runcrawl.Length, runcrawl.Length);
            runcrawlpos += runcrawl.Length;
            runcrawl = newcrawl;
        }

        protected void Crawl(int i)
        {
            if (runcrawlpos == 0)
                DoubleCrawl();

            runcrawl[--runcrawlpos] = i;
        }

        protected int Popcrawl()
        {
            return runcrawl[runcrawlpos++];
        }

        protected int Crawlpos()
        {
            return runcrawl.Length - runcrawlpos;
        }

        protected void Capture(int capnum, int start, int end)
        {
            if (end < start)
            {
                int T;

                T = end;
                end = start;
                start = T;
            }

            Crawl(capnum);
            runmatch.AddMatch(capnum, start, end - start);
        }

        protected void TransferCapture(int capnum, int uncapnum, int start, int end)
        {
            int start2;
            int end2;



            if (end < start)
            {
                int T;

                T = end;
                end = start;
                start = T;
            }

            start2 = MatchIndex(uncapnum);
            end2 = start2 + MatchLength(uncapnum);



            if (start >= end2)
            {
                end = start;
                start = end2;
            }
            else if (end <= start2)
            {
                start = start2;
            }
            else
            {
                if (end > end2)
                    end = end2;
                if (start2 > start)
                    start = start2;
            }

            Crawl(uncapnum);
            runmatch.BalanceMatch(uncapnum);

            if (capnum != -1)
            {
                Crawl(capnum);
                runmatch.AddMatch(capnum, start, end - start);
            }
        }


        protected void Uncapture()
        {
            int capnum = Popcrawl();
            runmatch.RemoveMatch(capnum);
        }

        protected bool IsMatched(int cap)
        {
            return runmatch.IsMatched(cap);
        }

        protected int MatchIndex(int cap)
        {
            return runmatch.MatchIndex(cap);
        }

        protected int MatchLength(int cap)
        {
            return runmatch.MatchLength(cap);
        }

#if DEBUG
        internal virtual void DumpState()
        {
            Debug.WriteLine("Text:  " + TextposDescription());
            Debug.WriteLine("Track: " + StackDescription(runtrack, runtrackpos));
            Debug.WriteLine("Stack: " + StackDescription(runstack, runstackpos));
        }

        private static string StackDescription(int[] a, int index)
        {
            var sb = new StringBuilder();

            sb.Append(a.Length - index);
            sb.Append('/');
            sb.Append(a.Length);

            if (sb.Length < 8)
                sb.Append(' ', 8 - sb.Length);

            sb.Append('(');

            for (int i = index; i < a.Length; i++)
            {
                if (i > index)
                    sb.Append(' ');
                sb.Append(a[i]);
            }

            sb.Append(')');

            return sb.ToString();
        }

        internal virtual string TextposDescription()
        {
            var sb = new StringBuilder();
            int remaining;

            sb.Append(runtextpos);

            if (sb.Length < 8)
                sb.Append(' ', 8 - sb.Length);

            if (runtextpos > runtextbeg)
                sb.Append(RegexCharClass.CharDescription(runtext[runtextpos - 1]));
            else
                sb.Append('^');

            sb.Append('>');

            remaining = runtextend - runtextpos;

            for (int i = runtextpos; i < runtextend; i++)
            {
                sb.Append(RegexCharClass.CharDescription(runtext[i]));
            }
            if (sb.Length >= 64)
            {
                sb.Length = 61;
                sb.Append("...");
            }
            else
            {
                sb.Append('$');
            }

            return sb.ToString();
        }
#endif
    }
}
