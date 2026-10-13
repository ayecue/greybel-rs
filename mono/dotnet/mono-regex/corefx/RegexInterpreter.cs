






using System.Diagnostics;
using System.Globalization;

namespace MonoCorefx.Text.RegularExpressions
{
    internal sealed class RegexInterpreter : RegexRunner
    {
        private readonly RegexCode _code;
        private readonly CultureInfo _culture;
        private int _operator;
        private int _codepos;
        private bool _rightToLeft;
        private bool _caseInsensitive;

        public RegexInterpreter(RegexCode code, CultureInfo culture)
        {
            Debug.Assert(code != null, "code cannot be null.");
            Debug.Assert(culture != null, "culture cannot be null.");

            _code = code;
            _culture = culture;
        }

        protected override void InitTrackCount()
        {
            runtrackcount = _code.TrackCount;
        }

        private void Advance(int i)
        {
            _codepos += (i + 1);
            SetOperator(_code.Codes[_codepos]);
        }

        private void Goto(int newpos)
        {

            if (newpos < _codepos)
                EnsureStorage();

            SetOperator(_code.Codes[newpos]);
            _codepos = newpos;
        }

        private void Textto(int newpos)
        {
            runtextpos = newpos;
        }

        private void Trackto(int newpos)
        {
            runtrackpos = runtrack.Length - newpos;
        }

        private int Textstart()
        {
            return runtextstart;
        }

        private int Textpos()
        {
            return runtextpos;
        }


        private int Trackpos()
        {
            return runtrack.Length - runtrackpos;
        }

        private void TrackPush()
        {
            runtrack[--runtrackpos] = _codepos;
        }

        private void TrackPush(int I1)
        {
            runtrack[--runtrackpos] = I1;
            runtrack[--runtrackpos] = _codepos;
        }

        private void TrackPush(int I1, int I2)
        {
            runtrack[--runtrackpos] = I1;
            runtrack[--runtrackpos] = I2;
            runtrack[--runtrackpos] = _codepos;
        }

        private void TrackPush(int I1, int I2, int I3)
        {
            runtrack[--runtrackpos] = I1;
            runtrack[--runtrackpos] = I2;
            runtrack[--runtrackpos] = I3;
            runtrack[--runtrackpos] = _codepos;
        }

        private void TrackPush2(int I1)
        {
            runtrack[--runtrackpos] = I1;
            runtrack[--runtrackpos] = -_codepos;
        }

        private void TrackPush2(int I1, int I2)
        {
            runtrack[--runtrackpos] = I1;
            runtrack[--runtrackpos] = I2;
            runtrack[--runtrackpos] = -_codepos;
        }

        private void Backtrack()
        {
            int newpos = runtrack[runtrackpos++];
#if DEBUG
            if (runmatch.Debug)
            {
                if (newpos < 0)
                    Debug.WriteLine("       Backtracking (back2) to code position " + (-newpos));
                else
                    Debug.WriteLine("       Backtracking to code position " + newpos);
            }
#endif

            if (newpos < 0)
            {
                newpos = -newpos;
                SetOperator(_code.Codes[newpos] | RegexCode.Back2);
            }
            else
            {
                SetOperator(_code.Codes[newpos] | RegexCode.Back);
            }


            if (newpos < _codepos)
                EnsureStorage();

            _codepos = newpos;
        }

        private void SetOperator(int op)
        {
            _caseInsensitive = (0 != (op & RegexCode.Ci));
            _rightToLeft = (0 != (op & RegexCode.Rtl));
            _operator = op & ~(RegexCode.Rtl | RegexCode.Ci);
        }

        private void TrackPop()
        {
            runtrackpos++;
        }


        private void TrackPop(int framesize)
        {
            runtrackpos += framesize;
        }





        private int TrackPeek()
        {
            return runtrack[runtrackpos - 1];
        }


        private int TrackPeek(int i)
        {
            return runtrack[runtrackpos - i - 1];
        }


        private void StackPush(int I1)
        {
            runstack[--runstackpos] = I1;
        }

        private void StackPush(int I1, int I2)
        {
            runstack[--runstackpos] = I1;
            runstack[--runstackpos] = I2;
        }

        private void StackPop()
        {
            runstackpos++;
        }


        private void StackPop(int framesize)
        {
            runstackpos += framesize;
        }





        private int StackPeek()
        {
            return runstack[runstackpos - 1];
        }


        private int StackPeek(int i)
        {
            return runstack[runstackpos - i - 1];
        }

        private int Operator()
        {
            return _operator;
        }

        private int Operand(int i)
        {
            return _code.Codes[_codepos + i + 1];
        }

        private int Leftchars()
        {
            return runtextpos - runtextbeg;
        }

        private int Rightchars()
        {
            return runtextend - runtextpos;
        }

        private int Bump()
        {
            return _rightToLeft ? -1 : 1;
        }

        private int Forwardchars()
        {
            return _rightToLeft ? runtextpos - runtextbeg : runtextend - runtextpos;
        }

        private char Forwardcharnext()
        {
            char ch = (_rightToLeft ? runtext[--runtextpos] : runtext[runtextpos++]);

            return (_caseInsensitive ? _culture.TextInfo.ToLower(ch) : ch);
        }

        private bool Stringmatch(string str)
        {
            int c;
            int pos;

            if (!_rightToLeft)
            {
                if (runtextend - runtextpos < (c = str.Length))
                    return false;

                pos = runtextpos + c;
            }
            else
            {
                if (runtextpos - runtextbeg < (c = str.Length))
                    return false;

                pos = runtextpos;
            }

            if (!_caseInsensitive)
            {
                while (c != 0)
                    if (str[--c] != runtext[--pos])
                        return false;
            }
            else
            {
                while (c != 0)
                    if (str[--c] != _culture.TextInfo.ToLower(runtext[--pos]))
                        return false;
            }

            if (!_rightToLeft)
            {
                pos += str.Length;
            }

            runtextpos = pos;

            return true;
        }

        private bool Refmatch(int index, int len)
        {
            int c;
            int pos;
            int cmpos;

            if (!_rightToLeft)
            {
                if (runtextend - runtextpos < len)
                    return false;

                pos = runtextpos + len;
            }
            else
            {
                if (runtextpos - runtextbeg < len)
                    return false;

                pos = runtextpos;
            }
            cmpos = index + len;

            c = len;

            if (!_caseInsensitive)
            {
                while (c-- != 0)
                    if (runtext[--cmpos] != runtext[--pos])
                        return false;
            }
            else
            {
                while (c-- != 0)
                    if (_culture.TextInfo.ToLower(runtext[--cmpos]) != _culture.TextInfo.ToLower(runtext[--pos]))
                        return false;
            }

            if (!_rightToLeft)
            {
                pos += len;
            }

            runtextpos = pos;

            return true;
        }

        private void Backwardnext()
        {
            runtextpos += _rightToLeft ? 1 : -1;
        }

        private char CharAt(int j)
        {
            return runtext[j];
        }

        protected override bool FindFirstChar()
        {
            if (0 != (_code.Anchors & (RegexFCD.Beginning | RegexFCD.Start | RegexFCD.EndZ | RegexFCD.End)))
            {
                if (!_code.RightToLeft)
                {
                    if ((0 != (_code.Anchors & RegexFCD.Beginning) && runtextpos > runtextbeg) ||
                        (0 != (_code.Anchors & RegexFCD.Start) && runtextpos > runtextstart))
                    {
                        runtextpos = runtextend;
                        return false;
                    }
                    if (0 != (_code.Anchors & RegexFCD.EndZ) && runtextpos < runtextend - 1)
                    {
                        runtextpos = runtextend - 1;
                    }
                    else if (0 != (_code.Anchors & RegexFCD.End) && runtextpos < runtextend)
                    {
                        runtextpos = runtextend;
                    }
                }
                else
                {
                    if ((0 != (_code.Anchors & RegexFCD.End) && runtextpos < runtextend) ||
                        (0 != (_code.Anchors & RegexFCD.EndZ) && (runtextpos < runtextend - 1 ||
                                                               (runtextpos == runtextend - 1 && CharAt(runtextpos) != '\n'))) ||
                        (0 != (_code.Anchors & RegexFCD.Start) && runtextpos < runtextstart))
                    {
                        runtextpos = runtextbeg;
                        return false;
                    }
                    if (0 != (_code.Anchors & RegexFCD.Beginning) && runtextpos > runtextbeg)
                    {
                        runtextpos = runtextbeg;
                    }
                }

                if (_code.BMPrefix != null)
                {
                    return _code.BMPrefix.IsMatch(runtext, runtextpos, runtextbeg, runtextend);
                }

                return true;
            }
            else if (_code.BMPrefix != null)
            {
                runtextpos = _code.BMPrefix.Scan(runtext, runtextpos, runtextbeg, runtextend);

                if (runtextpos == -1)
                {
                    runtextpos = (_code.RightToLeft ? runtextbeg : runtextend);
                    return false;
                }

                return true;
            }
            else if (_code.FCPrefix == null)
            {
                return true;
            }

            _rightToLeft = _code.RightToLeft;
            _caseInsensitive = _code.FCPrefix.GetValueOrDefault().CaseInsensitive;
            string set = _code.FCPrefix.GetValueOrDefault().Prefix;

            if (RegexCharClass.IsSingleton(set))
            {
                char ch = RegexCharClass.SingletonChar(set);

                for (int i = Forwardchars(); i > 0; i--)
                {
                    if (ch == Forwardcharnext())
                    {
                        Backwardnext();
                        return true;
                    }
                }
            }
            else
            {
                for (int i = Forwardchars(); i > 0; i--)
                {
                    if (RegexCharClass.CharInClass(Forwardcharnext(), set))
                    {
                        Backwardnext();
                        return true;
                    }
                }
            }

            return false;
        }

        protected override void Go()
        {
            Goto(0);

            int advance = -1;
            for (; ; )
            {
                if (advance >= 0)
                {


                    Advance(advance);
                    advance = -1;
                }
#if DEBUG
                if (runmatch.Debug)
                {
                    DumpState();
                }
#endif

                CheckTimeout();

                switch (Operator())
                {
                    case RegexCode.Stop:
                        return;

                    case RegexCode.Nothing:
                        break;

                    case RegexCode.Goto:
                        Goto(Operand(0));
                        continue;

                    case RegexCode.Testref:
                        if (!IsMatched(Operand(0)))
                            break;
                        advance = 1;
                        continue;

                    case RegexCode.Lazybranch:
                        TrackPush(Textpos());
                        advance = 1;
                        continue;

                    case RegexCode.Lazybranch | RegexCode.Back:
                        TrackPop();
                        Textto(TrackPeek());
                        Goto(Operand(0));
                        continue;

                    case RegexCode.Setmark:
                        StackPush(Textpos());
                        TrackPush();
                        advance = 0;
                        continue;

                    case RegexCode.Nullmark:
                        StackPush(-1);
                        TrackPush();
                        advance = 0;
                        continue;

                    case RegexCode.Setmark | RegexCode.Back:
                    case RegexCode.Nullmark | RegexCode.Back:
                        StackPop();
                        break;

                    case RegexCode.Getmark:
                        StackPop();
                        TrackPush(StackPeek());
                        Textto(StackPeek());
                        advance = 0;
                        continue;

                    case RegexCode.Getmark | RegexCode.Back:
                        TrackPop();
                        StackPush(TrackPeek());
                        break;

                    case RegexCode.Capturemark:
                        if (Operand(1) != -1 && !IsMatched(Operand(1)))
                            break;
                        StackPop();
                        if (Operand(1) != -1)
                            TransferCapture(Operand(0), Operand(1), StackPeek(), Textpos());
                        else
                            Capture(Operand(0), StackPeek(), Textpos());
                        TrackPush(StackPeek());

                        advance = 2;

                        continue;

                    case RegexCode.Capturemark | RegexCode.Back:
                        TrackPop();
                        StackPush(TrackPeek());
                        Uncapture();
                        if (Operand(0) != -1 && Operand(1) != -1)
                            Uncapture();

                        break;

                    case RegexCode.Branchmark:
                        {
                            int matched;
                            StackPop();

                            matched = Textpos() - StackPeek();

                            if (matched != 0)
                            {
                                TrackPush(StackPeek(), Textpos());
                                StackPush(Textpos());
                                Goto(Operand(0));
                            }
                            else
                            {
                                TrackPush2(StackPeek());
                                advance = 1;
                            }
                            continue;
                        }

                    case RegexCode.Branchmark | RegexCode.Back:
                        TrackPop(2);
                        StackPop();
                        Textto(TrackPeek(1));
                        TrackPush2(TrackPeek());
                        advance = 1;
                        continue;

                    case RegexCode.Branchmark | RegexCode.Back2:
                        TrackPop();
                        StackPush(TrackPeek());
                        break;

                    case RegexCode.Lazybranchmark:
                        {



                            StackPop();

                            int oldMarkPos = StackPeek();

                            if (Textpos() != oldMarkPos)
                            {
                                if (oldMarkPos != -1)
                                    TrackPush(oldMarkPos, Textpos());
                                else
                                    TrackPush(Textpos(), Textpos());
                            }
                            else
                            {




                                StackPush(oldMarkPos);

                                TrackPush2(StackPeek());
                            }
                            advance = 1;
                            continue;
                        }

                    case RegexCode.Lazybranchmark | RegexCode.Back:
                        {





                            int pos;

                            TrackPop(2);
                            pos = TrackPeek(1);
                            TrackPush2(TrackPeek());
                            StackPush(pos);
                            Textto(pos);
                            Goto(Operand(0));
                            continue;
                        }

                    case RegexCode.Lazybranchmark | RegexCode.Back2:


                        StackPop();
                        TrackPop();
                        StackPush(TrackPeek());
                        break;

                    case RegexCode.Setcount:
                        StackPush(Textpos(), Operand(0));
                        TrackPush();
                        advance = 1;
                        continue;

                    case RegexCode.Nullcount:
                        StackPush(-1, Operand(0));
                        TrackPush();
                        advance = 1;
                        continue;

                    case RegexCode.Setcount | RegexCode.Back:
                        StackPop(2);
                        break;

                    case RegexCode.Nullcount | RegexCode.Back:
                        StackPop(2);
                        break;

                    case RegexCode.Branchcount:



                        {
                            StackPop(2);
                            int mark = StackPeek();
                            int count = StackPeek(1);
                            int matched = Textpos() - mark;

                            if (count >= Operand(1) || (matched == 0 && count >= 0))
                            {
                                TrackPush2(mark, count);
                                advance = 2;
                            }
                            else
                            {
                                TrackPush(mark);
                                StackPush(Textpos(), count + 1);
                                Goto(Operand(0));
                            }
                            continue;
                        }

                    case RegexCode.Branchcount | RegexCode.Back:





                        TrackPop();
                        StackPop(2);
                        if (StackPeek(1) > 0)
                        {
                            Textto(StackPeek());
                            TrackPush2(TrackPeek(), StackPeek(1) - 1);
                            advance = 2;
                            continue;
                        }
                        StackPush(TrackPeek(), StackPeek(1) - 1);
                        break;

                    case RegexCode.Branchcount | RegexCode.Back2:



                        TrackPop(2);
                        StackPush(TrackPeek(), TrackPeek(1));
                        break;


                    case RegexCode.Lazybranchcount:



                        {
                            StackPop(2);
                            int mark = StackPeek();
                            int count = StackPeek(1);

                            if (count < 0)
                            {
                                TrackPush2(mark);
                                StackPush(Textpos(), count + 1);
                                Goto(Operand(0));
                            }
                            else
                            {
                                TrackPush(mark, count, Textpos());
                                advance = 2;
                            }
                            continue;
                        }

                    case RegexCode.Lazybranchcount | RegexCode.Back:




                        {
                            TrackPop(3);
                            int mark = TrackPeek();
                            int textpos = TrackPeek(2);

                            if (TrackPeek(1) < Operand(1) && textpos != mark)
                            {
                                Textto(textpos);
                                StackPush(textpos, TrackPeek(1) + 1);
                                TrackPush2(mark);
                                Goto(Operand(0));
                                continue;
                            }
                            else
                            {
                                StackPush(TrackPeek(), TrackPeek(1));
                                break;
                            }
                        }

                    case RegexCode.Lazybranchcount | RegexCode.Back2:





                        TrackPop();
                        StackPop(2);
                        StackPush(TrackPeek(), StackPeek(1) - 1);
                        break;

                    case RegexCode.Setjump:
                        StackPush(Trackpos(), Crawlpos());
                        TrackPush();
                        advance = 0;
                        continue;

                    case RegexCode.Setjump | RegexCode.Back:
                        StackPop(2);
                        break;

                    case RegexCode.Backjump:



                        StackPop(2);
                        Trackto(StackPeek());

                        while (Crawlpos() != StackPeek(1))
                            Uncapture();

                        break;

                    case RegexCode.Forejump:



                        StackPop(2);
                        Trackto(StackPeek());
                        TrackPush(StackPeek(1));
                        advance = 0;
                        continue;

                    case RegexCode.Forejump | RegexCode.Back:


                        TrackPop();

                        while (Crawlpos() != TrackPeek())
                            Uncapture();

                        break;

                    case RegexCode.Bol:
                        if (Leftchars() > 0 && CharAt(Textpos() - 1) != '\n')
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.Eol:
                        if (Rightchars() > 0 && CharAt(Textpos()) != '\n')
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.Boundary:
                        if (!IsBoundary(Textpos(), runtextbeg, runtextend))
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.Nonboundary:
                        if (IsBoundary(Textpos(), runtextbeg, runtextend))
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.ECMABoundary:
                        if (!IsECMABoundary(Textpos(), runtextbeg, runtextend))
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.NonECMABoundary:
                        if (IsECMABoundary(Textpos(), runtextbeg, runtextend))
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.Beginning:
                        if (Leftchars() > 0)
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.Start:
                        if (Textpos() != Textstart())
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.EndZ:
                        if (Rightchars() > 1 || Rightchars() == 1 && CharAt(Textpos()) != '\n')
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.End:
                        if (Rightchars() > 0)
                            break;
                        advance = 0;
                        continue;

                    case RegexCode.One:
                        if (Forwardchars() < 1 || Forwardcharnext() != (char)Operand(0))
                            break;

                        advance = 1;
                        continue;

                    case RegexCode.Notone:
                        if (Forwardchars() < 1 || Forwardcharnext() == (char)Operand(0))
                            break;

                        advance = 1;
                        continue;

                    case RegexCode.Set:
                        if (Forwardchars() < 1 || !RegexCharClass.CharInClass(Forwardcharnext(), _code.Strings[Operand(0)]))
                            break;

                        advance = 1;
                        continue;

                    case RegexCode.Multi:
                        {
                            if (!Stringmatch(_code.Strings[Operand(0)]))
                                break;

                            advance = 1;
                            continue;
                        }

                    case RegexCode.Ref:
                        {
                            int capnum = Operand(0);

                            if (IsMatched(capnum))
                            {
                                if (!Refmatch(MatchIndex(capnum), MatchLength(capnum)))
                                    break;
                            }
                            else
                            {
                                if ((runregex.roptions & RegexOptions.ECMAScript) == 0)
                                    break;
                            }

                            advance = 1;
                            continue;
                        }

                    case RegexCode.Onerep:
                        {
                            int c = Operand(1);

                            if (Forwardchars() < c)
                                break;

                            char ch = (char)Operand(0);

                            while (c-- > 0)
                                if (Forwardcharnext() != ch)
                                    goto BreakBackward;

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Notonerep:
                        {
                            int c = Operand(1);

                            if (Forwardchars() < c)
                                break;

                            char ch = (char)Operand(0);

                            while (c-- > 0)
                                if (Forwardcharnext() == ch)
                                    goto BreakBackward;

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Setrep:
                        {
                            int c = Operand(1);

                            if (Forwardchars() < c)
                                break;

                            string set = _code.Strings[Operand(0)];

                            while (c-- > 0)
                                if (!RegexCharClass.CharInClass(Forwardcharnext(), set))
                                    goto BreakBackward;

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Oneloop:
                        {
                            int c = Operand(1);

                            if (c > Forwardchars())
                                c = Forwardchars();

                            char ch = (char)Operand(0);
                            int i;

                            for (i = c; i > 0; i--)
                            {
                                if (Forwardcharnext() != ch)
                                {
                                    Backwardnext();
                                    break;
                                }
                            }

                            if (c > i)
                                TrackPush(c - i - 1, Textpos() - Bump());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Notoneloop:
                        {
                            int c = Operand(1);

                            if (c > Forwardchars())
                                c = Forwardchars();

                            char ch = (char)Operand(0);
                            int i;

                            for (i = c; i > 0; i--)
                            {
                                if (Forwardcharnext() == ch)
                                {
                                    Backwardnext();
                                    break;
                                }
                            }

                            if (c > i)
                                TrackPush(c - i - 1, Textpos() - Bump());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Setloop:
                        {
                            int c = Operand(1);

                            if (c > Forwardchars())
                                c = Forwardchars();

                            string set = _code.Strings[Operand(0)];
                            int i;

                            for (i = c; i > 0; i--)
                            {
                                if (!RegexCharClass.CharInClass(Forwardcharnext(), set))
                                {
                                    Backwardnext();
                                    break;
                                }
                            }

                            if (c > i)
                                TrackPush(c - i - 1, Textpos() - Bump());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Oneloop | RegexCode.Back:
                    case RegexCode.Notoneloop | RegexCode.Back:
                        {
                            TrackPop(2);
                            int i = TrackPeek();
                            int pos = TrackPeek(1);

                            Textto(pos);

                            if (i > 0)
                                TrackPush(i - 1, pos - Bump());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Setloop | RegexCode.Back:
                        {
                            TrackPop(2);
                            int i = TrackPeek();
                            int pos = TrackPeek(1);

                            Textto(pos);

                            if (i > 0)
                                TrackPush(i - 1, pos - Bump());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Onelazy:
                    case RegexCode.Notonelazy:
                        {
                            int c = Operand(1);

                            if (c > Forwardchars())
                                c = Forwardchars();

                            if (c > 0)
                                TrackPush(c - 1, Textpos());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Setlazy:
                        {
                            int c = Operand(1);

                            if (c > Forwardchars())
                                c = Forwardchars();

                            if (c > 0)
                                TrackPush(c - 1, Textpos());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Onelazy | RegexCode.Back:
                        {
                            TrackPop(2);
                            int pos = TrackPeek(1);
                            Textto(pos);

                            if (Forwardcharnext() != (char)Operand(0))
                                break;

                            int i = TrackPeek();

                            if (i > 0)
                                TrackPush(i - 1, pos + Bump());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Notonelazy | RegexCode.Back:
                        {
                            TrackPop(2);
                            int pos = TrackPeek(1);
                            Textto(pos);

                            if (Forwardcharnext() == (char)Operand(0))
                                break;

                            int i = TrackPeek();

                            if (i > 0)
                                TrackPush(i - 1, pos + Bump());

                            advance = 2;
                            continue;
                        }

                    case RegexCode.Setlazy | RegexCode.Back:
                        {
                            TrackPop(2);
                            int pos = TrackPeek(1);
                            Textto(pos);

                            if (!RegexCharClass.CharInClass(Forwardcharnext(), _code.Strings[Operand(0)]))
                                break;

                            int i = TrackPeek();

                            if (i > 0)
                                TrackPush(i - 1, pos + Bump());

                            advance = 2;
                            continue;
                        }

                    default:
                        throw NotImplemented.ByDesignWithMessage(SR.UnimplementedState);
                }

            BreakBackward:
                ;


                Backtrack();
            }
        }

#if DEBUG
        internal override void DumpState()
        {
            base.DumpState();
            Debug.WriteLine("       " + _code.OpcodeDescription(_codepos) +
                              ((_operator & RegexCode.Back) != 0 ? " Back" : "") +
                              ((_operator & RegexCode.Back2) != 0 ? " Back2" : ""));
            Debug.WriteLine("");
        }
#endif
    }
}
