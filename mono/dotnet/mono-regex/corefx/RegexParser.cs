











using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace MonoCorefx.Text.RegularExpressions
{
    internal sealed class RegexParser
    {
        private const int MaxValueDiv10 = int.MaxValue / 10;
        private const int MaxValueMod10 = int.MaxValue % 10;

        private RegexNode _stack;
        private RegexNode _group;
        private RegexNode _alternation;
        private RegexNode _concatenation;
        private RegexNode _unit;

        private string _pattern;
        private int _currentPos;
        private CultureInfo _culture;

        private int _autocap;
        private int _capcount;
        private int _captop;
        private int _capsize;

        private Hashtable _caps;
        private Hashtable _capnames;

        private int[] _capnumlist;
        private List<string> _capnamelist;

        private RegexOptions _options;
        private List<RegexOptions> _optionsStack;

        private bool _ignoreNextParen = false;


        public static RegexTree Parse(string re, RegexOptions op)
        {
            RegexParser p;
            RegexNode root;
            string[] capnamelist;

            p = new RegexParser((op & RegexOptions.CultureInvariant) != 0 ? CultureInfo.InvariantCulture : CultureInfo.CurrentCulture);

            p._options = op;

            p.SetPattern(re);
            p.CountCaptures();
            p.Reset(op);
            root = p.ScanRegex();

            if (p._capnamelist == null)
                capnamelist = null;
            else
                capnamelist = p._capnamelist.ToArray();

            return new RegexTree(root, p._caps, p._capnumlist, p._captop, p._capnames, capnamelist, op);
        }


        public static RegexReplacement ParseReplacement(string rep, Hashtable caps, int capsize, Hashtable capnames, RegexOptions op)
        {
            RegexParser p;
            RegexNode root;

            p = new RegexParser((op & RegexOptions.CultureInvariant) != 0 ? CultureInfo.InvariantCulture : CultureInfo.CurrentCulture);

            p._options = op;

            p.NoteCaptures(caps, capsize, capnames);
            p.SetPattern(rep);
            root = p.ScanReplacement();

            return new RegexReplacement(rep, root, caps);
        }


        public static string Escape(string input)
        {
            for (int i = 0; i < input.Length; i++)
            {
                if (IsMetachar(input[i]))
                {
                    StringBuilder sb = StringBuilderCache.Acquire();
                    char ch = input[i];
                    int lastpos;

                    sb.Append(input, 0, i);
                    do
                    {
                        sb.Append('\\');
                        switch (ch)
                        {
                            case '\n':
                                ch = 'n';
                                break;
                            case '\r':
                                ch = 'r';
                                break;
                            case '\t':
                                ch = 't';
                                break;
                            case '\f':
                                ch = 'f';
                                break;
                        }
                        sb.Append(ch);
                        i++;
                        lastpos = i;

                        while (i < input.Length)
                        {
                            ch = input[i];
                            if (IsMetachar(ch))
                                break;

                            i++;
                        }

                        sb.Append(input, lastpos, i - lastpos);
                    } while (i < input.Length);

                    return StringBuilderCache.GetStringAndRelease(sb);
                }
            }

            return input;
        }


        public static string Unescape(string input)
        {
            for (int i = 0; i < input.Length; i++)
            {
                if (input[i] == '\\')
                {
                    StringBuilder sb = StringBuilderCache.Acquire();
                    RegexParser p = new RegexParser(CultureInfo.InvariantCulture);
                    int lastpos;
                    p.SetPattern(input);

                    sb.Append(input, 0, i);
                    do
                    {
                        i++;
                        p.Textto(i);
                        if (i < input.Length)
                            sb.Append(p.ScanCharEscape());
                        i = p.Textpos();
                        lastpos = i;
                        while (i < input.Length && input[i] != '\\')
                            i++;
                        sb.Append(input, lastpos, i - lastpos);
                    } while (i < input.Length);

                    return StringBuilderCache.GetStringAndRelease(sb);
                }
            }

            return input;
        }


        private RegexParser(CultureInfo culture)
        {
            _culture = culture;
            _optionsStack = new List<RegexOptions>();
            _caps = new Hashtable();
        }


        private void SetPattern(string Re)
        {
            if (Re == null)
                Re = string.Empty;
            _pattern = Re;
            _currentPos = 0;
        }


        private void Reset(RegexOptions topopts)
        {
            _currentPos = 0;
            _autocap = 1;
            _ignoreNextParen = false;

            if (_optionsStack.Count > 0)
                _optionsStack.RemoveRange(0, _optionsStack.Count - 1);

            _options = topopts;
            _stack = null;
        }


        private RegexNode ScanRegex()
        {
            char ch = '@';
            bool isQuantifier = false;

            StartGroup(new RegexNode(RegexNode.Capture, _options, 0, -1));

            while (CharsRight() > 0)
            {
                bool wasPrevQuantifier = isQuantifier;
                isQuantifier = false;

                ScanBlank();

                int startpos = Textpos();



                if (UseOptionX())
                    while (CharsRight() > 0 && (!IsStopperX(ch = RightChar()) || ch == '{' && !IsTrueQuantifier()))
                        MoveRight();
                else
                    while (CharsRight() > 0 && (!IsSpecial(ch = RightChar()) || ch == '{' && !IsTrueQuantifier()))
                        MoveRight();

                int endpos = Textpos();

                ScanBlank();

                if (CharsRight() == 0)
                    ch = '!';
                else if (IsSpecial(ch = RightChar()))
                {
                    isQuantifier = IsQuantifier(ch);
                    MoveRight();
                }
                else
                    ch = ' ';

                if (startpos < endpos)
                {
                    int cchUnquantified = endpos - startpos - (isQuantifier ? 1 : 0);

                    wasPrevQuantifier = false;

                    if (cchUnquantified > 0)
                        AddConcatenate(startpos, cchUnquantified, false);

                    if (isQuantifier)
                        AddUnitOne(CharAt(endpos - 1));
                }

                switch (ch)
                {
                    case '!':
                        goto BreakOuterScan;

                    case ' ':
                        goto ContinueOuterScan;

                    case '[':
                        AddUnitSet(ScanCharClass(UseOptionI(), scanOnly: false).ToStringClass());
                        break;

                    case '(':
                        {
                            RegexNode grouper;

                            PushOptions();

                            if (null == (grouper = ScanGroupOpen()))
                            {
                                PopKeepOptions();
                            }
                            else
                            {
                                PushGroup();
                                StartGroup(grouper);
                            }
                        }
                        continue;

                    case '|':
                        AddAlternate();
                        goto ContinueOuterScan;

                    case ')':
                        if (EmptyStack())
                            throw MakeException(SR.TooManyParens);

                        AddGroup();
                        PopGroup();
                        PopOptions();

                        if (Unit() == null)
                            goto ContinueOuterScan;
                        break;

                    case '\\':
                        AddUnitNode(ScanBackslash(scanOnly: false));
                        break;

                    case '^':
                        AddUnitType(UseOptionM() ? RegexNode.Bol : RegexNode.Beginning);
                        break;

                    case '$':
                        AddUnitType(UseOptionM() ? RegexNode.Eol : RegexNode.EndZ);
                        break;

                    case '.':
                        if (UseOptionS())
                            AddUnitSet(RegexCharClass.AnyClass);
                        else
                            AddUnitNotone('\n');
                        break;

                    case '{':
                    case '*':
                    case '+':
                    case '?':
                        if (Unit() == null)
                            throw MakeException(wasPrevQuantifier ?
                                                SR.Format(SR.NestedQuantify, ch.ToString()) :
                                                SR.QuantifyAfterNothing);
                        MoveLeft();
                        break;

                    default:
                        throw MakeException(SR.InternalError);
                }

                ScanBlank();

                if (CharsRight() == 0 || !(isQuantifier = IsTrueQuantifier()))
                {
                    AddConcatenate();
                    goto ContinueOuterScan;
                }

                ch = RightCharMoveRight();


                while (Unit() != null)
                {
                    int min;
                    int max;
                    bool lazy;

                    switch (ch)
                    {
                        case '*':
                            min = 0;
                            max = int.MaxValue;
                            break;

                        case '?':
                            min = 0;
                            max = 1;
                            break;

                        case '+':
                            min = 1;
                            max = int.MaxValue;
                            break;

                        case '{':
                            {
                                startpos = Textpos();
                                max = min = ScanDecimal();
                                if (startpos < Textpos())
                                {
                                    if (CharsRight() > 0 && RightChar() == ',')
                                    {
                                        MoveRight();
                                        if (CharsRight() == 0 || RightChar() == '}')
                                            max = int.MaxValue;
                                        else
                                            max = ScanDecimal();
                                    }
                                }

                                if (startpos == Textpos() || CharsRight() == 0 || RightCharMoveRight() != '}')
                                {
                                    AddConcatenate();
                                    Textto(startpos - 1);
                                    goto ContinueOuterScan;
                                }
                            }

                            break;

                        default:
                            throw MakeException(SR.InternalError);
                    }

                    ScanBlank();

                    if (CharsRight() == 0 || RightChar() != '?')
                        lazy = false;
                    else
                    {
                        MoveRight();
                        lazy = true;
                    }

                    if (min > max)
                        throw MakeException(SR.IllegalRange);

                    AddConcatenate(lazy, min, max);
                }

            ContinueOuterScan:
                ;
            }

        BreakOuterScan:
            ;

            if (!EmptyStack())
                throw MakeException(SR.NotEnoughParens);

            AddGroup();

            return Unit();
        }


        private RegexNode ScanReplacement()
        {
            int c;
            int startpos;

            _concatenation = new RegexNode(RegexNode.Concatenate, _options);

            for (; ; )
            {
                c = CharsRight();
                if (c == 0)
                    break;

                startpos = Textpos();

                while (c > 0 && RightChar() != '$')
                {
                    MoveRight();
                    c--;
                }

                AddConcatenate(startpos, Textpos() - startpos, true);

                if (c > 0)
                {
                    if (RightCharMoveRight() == '$')
                        AddUnitNode(ScanDollar());
                    AddConcatenate();
                }
            }

            return _concatenation;
        }


        private RegexCharClass ScanCharClass(bool caseInsensitive, bool scanOnly)
        {
            char ch = '\0';
            char chPrev = '\0';
            bool inRange = false;
            bool firstChar = true;
            bool closed = false;

            RegexCharClass cc;

            cc = scanOnly ? null : new RegexCharClass();

            if (CharsRight() > 0 && RightChar() == '^')
            {
                MoveRight();
                if (!scanOnly)
                    cc.Negate = true;
            }

            for (; CharsRight() > 0; firstChar = false)
            {
                bool fTranslatedChar = false;
                ch = RightCharMoveRight();
                if (ch == ']')
                {
                    if (!firstChar)
                    {
                        closed = true;
                        break;
                    }
                }
                else if (ch == '\\' && CharsRight() > 0)
                {
                    switch (ch = RightCharMoveRight())
                    {
                        case 'D':
                        case 'd':
                            if (!scanOnly)
                            {
                                if (inRange)
                                    throw MakeException(SR.Format(SR.BadClassInCharRange, ch.ToString()));
                                cc.AddDigit(UseOptionE(), ch == 'D', _pattern);
                            }
                            continue;

                        case 'S':
                        case 's':
                            if (!scanOnly)
                            {
                                if (inRange)
                                    throw MakeException(SR.Format(SR.BadClassInCharRange, ch.ToString()));
                                cc.AddSpace(UseOptionE(), ch == 'S');
                            }
                            continue;

                        case 'W':
                        case 'w':
                            if (!scanOnly)
                            {
                                if (inRange)
                                    throw MakeException(SR.Format(SR.BadClassInCharRange, ch.ToString()));

                                cc.AddWord(UseOptionE(), ch == 'W');
                            }
                            continue;

                        case 'p':
                        case 'P':
                            if (!scanOnly)
                            {
                                if (inRange)
                                    throw MakeException(SR.Format(SR.BadClassInCharRange, ch.ToString()));
                                cc.AddCategoryFromName(ParseProperty(), (ch != 'p'), caseInsensitive, _pattern);
                            }
                            else
                                ParseProperty();

                            continue;

                        case '-':
                            if (!scanOnly)
                                cc.AddRange(ch, ch);
                            continue;

                        default:
                            MoveLeft();
                            ch = ScanCharEscape();
                            fTranslatedChar = true;
                            break;
                    }
                }
                else if (ch == '[')
                {


                    if (CharsRight() > 0 && RightChar() == ':' && !inRange)
                    {
                        string name;
                        int savePos = Textpos();

                        MoveRight();
                        name = ScanCapname();
                        if (CharsRight() < 2 || RightCharMoveRight() != ':' || RightCharMoveRight() != ']')
                            Textto(savePos);

                    }
                }


                if (inRange)
                {
                    inRange = false;
                    if (!scanOnly)
                    {
                        if (ch == '[' && !fTranslatedChar && !firstChar)
                        {



                            cc.AddChar(chPrev);
                            cc.AddSubtraction(ScanCharClass(caseInsensitive, scanOnly));

                            if (CharsRight() > 0 && RightChar() != ']')
                                throw MakeException(SR.SubtractionMustBeLast);
                        }
                        else
                        {

                            if (chPrev > ch)
                                throw MakeException(SR.ReversedCharRange);
                            cc.AddRange(chPrev, ch);
                        }
                    }
                }
                else if (CharsRight() >= 2 && RightChar() == '-' && RightChar(1) != ']')
                {

                    chPrev = ch;
                    inRange = true;
                    MoveRight();
                }
                else if (CharsRight() >= 1 && ch == '-' && !fTranslatedChar && RightChar() == '[' && !firstChar)
                {


                    if (!scanOnly)
                    {
                        MoveRight(1);
                        cc.AddSubtraction(ScanCharClass(caseInsensitive, scanOnly));

                        if (CharsRight() > 0 && RightChar() != ']')
                            throw MakeException(SR.SubtractionMustBeLast);
                    }
                    else
                    {
                        MoveRight(1);
                        ScanCharClass(caseInsensitive, scanOnly);
                    }
                }
                else
                {
                    if (!scanOnly)
                        cc.AddRange(ch, ch);
                }
            }

            if (!closed)
                throw MakeException(SR.UnterminatedBracket);

            if (!scanOnly && caseInsensitive)
                cc.AddLowercase(_culture);

            return cc;
        }


        private RegexNode ScanGroupOpen()
        {
            char ch = '\0';
            int NodeType;
            char close = '>';






            if (CharsRight() == 0 || RightChar() != '?' || (RightChar() == '?' && (CharsRight() > 1 && RightChar(1) == ')')))
            {
                if (UseOptionN() || _ignoreNextParen)
                {
                    _ignoreNextParen = false;
                    return new RegexNode(RegexNode.Group, _options);
                }
                else
                    return new RegexNode(RegexNode.Capture, _options, _autocap++, -1);
            }

            MoveRight();

            for (; ; )
            {
                if (CharsRight() == 0)
                    break;

                switch (ch = RightCharMoveRight())
                {
                    case ':':

                        NodeType = RegexNode.Group;
                        break;

                    case '=':

                        _options &= ~(RegexOptions.RightToLeft);
                        NodeType = RegexNode.Require;
                        break;

                    case '!':

                        _options &= ~(RegexOptions.RightToLeft);
                        NodeType = RegexNode.Prevent;
                        break;

                    case '>':

                        NodeType = RegexNode.Greedy;
                        break;

                    case '\'':
                        close = '\'';
                        goto case '<';


                    case '<':
                        if (CharsRight() == 0)
                            goto BreakRecognize;

                        switch (ch = RightCharMoveRight())
                        {
                            case '=':
                                if (close == '\'')
                                    goto BreakRecognize;


                                _options |= RegexOptions.RightToLeft;
                                NodeType = RegexNode.Require;
                                break;

                            case '!':
                                if (close == '\'')
                                    goto BreakRecognize;


                                _options |= RegexOptions.RightToLeft;
                                NodeType = RegexNode.Prevent;
                                break;

                            default:
                                MoveLeft();
                                int capnum = -1;
                                int uncapnum = -1;
                                bool proceed = false;



                                if (ch >= '0' && ch <= '9')
                                {
                                    capnum = ScanDecimal();

                                    if (!IsCaptureSlot(capnum))
                                        capnum = -1;


                                    if (CharsRight() > 0 && !(RightChar() == close || RightChar() == '-'))
                                        throw MakeException(SR.InvalidGroupName);
                                    if (capnum == 0)
                                        throw MakeException(SR.CapnumNotZero);
                                }
                                else if (RegexCharClass.IsWordChar(ch))
                                {
                                    string capname = ScanCapname();

                                    if (IsCaptureName(capname))
                                        capnum = CaptureSlotFromName(capname);


                                    if (CharsRight() > 0 && !(RightChar() == close || RightChar() == '-'))
                                        throw MakeException(SR.InvalidGroupName);
                                }
                                else if (ch == '-')
                                {
                                    proceed = true;
                                }
                                else
                                {

                                    throw MakeException(SR.InvalidGroupName);
                                }



                                if ((capnum != -1 || proceed == true) && CharsRight() > 1 && RightChar() == '-')
                                {
                                    MoveRight();
                                    ch = RightChar();

                                    if (ch >= '0' && ch <= '9')
                                    {
                                        uncapnum = ScanDecimal();

                                        if (!IsCaptureSlot(uncapnum))
                                            throw MakeException(SR.Format(SR.UndefinedBackref, uncapnum));


                                        if (CharsRight() > 0 && RightChar() != close)
                                            throw MakeException(SR.InvalidGroupName);
                                    }
                                    else if (RegexCharClass.IsWordChar(ch))
                                    {
                                        string uncapname = ScanCapname();

                                        if (IsCaptureName(uncapname))
                                            uncapnum = CaptureSlotFromName(uncapname);
                                        else
                                            throw MakeException(SR.Format(SR.UndefinedNameRef, uncapname));


                                        if (CharsRight() > 0 && RightChar() != close)
                                            throw MakeException(SR.InvalidGroupName);
                                    }
                                    else
                                    {

                                        throw MakeException(SR.InvalidGroupName);
                                    }
                                }



                                if ((capnum != -1 || uncapnum != -1) && CharsRight() > 0 && RightCharMoveRight() == close)
                                {
                                    return new RegexNode(RegexNode.Capture, _options, capnum, uncapnum);
                                }
                                goto BreakRecognize;
                        }
                        break;

                    case '(':


                        int parenPos = Textpos();
                        if (CharsRight() > 0)
                        {
                            ch = RightChar();


                            if (ch >= '0' && ch <= '9')
                            {
                                int capnum = ScanDecimal();
                                if (CharsRight() > 0 && RightCharMoveRight() == ')')
                                {
                                    if (IsCaptureSlot(capnum))
                                        return new RegexNode(RegexNode.Testref, _options, capnum);
                                    else
                                        throw MakeException(SR.Format(SR.UndefinedReference, capnum.ToString(CultureInfo.CurrentCulture)));
                                }
                                else
                                    throw MakeException(SR.Format(SR.MalformedReference, capnum.ToString(CultureInfo.CurrentCulture)));
                            }
                            else if (RegexCharClass.IsWordChar(ch))
                            {
                                string capname = ScanCapname();

                                if (IsCaptureName(capname) && CharsRight() > 0 && RightCharMoveRight() == ')')
                                    return new RegexNode(RegexNode.Testref, _options, CaptureSlotFromName(capname));
                            }
                        }

                        NodeType = RegexNode.Testgroup;
                        Textto(parenPos - 1);
                        _ignoreNextParen = true;

                        int charsRight = CharsRight();
                        if (charsRight >= 3 && RightChar(1) == '?')
                        {
                            char rightchar2 = RightChar(2);

                            if (rightchar2 == '#')
                                throw MakeException(SR.AlternationCantHaveComment);


                            if (rightchar2 == '\'')
                                throw MakeException(SR.AlternationCantCapture);
                            else
                            {
                                if (charsRight >= 4 && (rightchar2 == '<' && RightChar(3) != '!' && RightChar(3) != '='))
                                    throw MakeException(SR.AlternationCantCapture);
                            }
                        }

                        break;


                    default:
                        MoveLeft();

                        NodeType = RegexNode.Group;

                        if (_group.NType != RegexNode.Testgroup)
                            ScanOptions();
                        if (CharsRight() == 0)
                            goto BreakRecognize;

                        if ((ch = RightCharMoveRight()) == ')')
                            return null;

                        if (ch != ':')
                            goto BreakRecognize;
                        break;
                }

                return new RegexNode(NodeType, _options);
            }

        BreakRecognize:
            ;


            throw MakeException(SR.UnrecognizedGrouping);
        }


        private void ScanBlank()
        {
            if (UseOptionX())
            {
                for (; ; )
                {
                    while (CharsRight() > 0 && IsSpace(RightChar()))
                        MoveRight();

                    if (CharsRight() == 0)
                        break;

                    if (RightChar() == '#')
                    {
                        while (CharsRight() > 0 && RightChar() != '\n')
                            MoveRight();
                    }
                    else if (CharsRight() >= 3 && RightChar(2) == '#' &&
                             RightChar(1) == '?' && RightChar() == '(')
                    {
                        while (CharsRight() > 0 && RightChar() != ')')
                            MoveRight();
                        if (CharsRight() == 0)
                            throw MakeException(SR.UnterminatedComment);
                        MoveRight();
                    }
                    else
                        break;
                }
            }
            else
            {
                for (; ; )
                {
                    if (CharsRight() < 3 || RightChar(2) != '#' ||
                        RightChar(1) != '?' || RightChar() != '(')
                        return;

                    while (CharsRight() > 0 && RightChar() != ')')
                        MoveRight();
                    if (CharsRight() == 0)
                        throw MakeException(SR.UnterminatedComment);
                    MoveRight();
                }
            }
        }


        private RegexNode ScanBackslash(bool scanOnly)
        {
            char ch;
            RegexCharClass cc;

            if (CharsRight() == 0)
                throw MakeException(SR.IllegalEndEscape);

            switch (ch = RightChar())
            {
                case 'b':
                case 'B':
                case 'A':
                case 'G':
                case 'Z':
                case 'z':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    return new RegexNode(TypeFromCode(ch), _options);

                case 'w':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    if (UseOptionE())
                        return new RegexNode(RegexNode.Set, _options, RegexCharClass.ECMAWordClass);
                    return new RegexNode(RegexNode.Set, _options, RegexCharClass.WordClass);

                case 'W':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    if (UseOptionE())
                        return new RegexNode(RegexNode.Set, _options, RegexCharClass.NotECMAWordClass);
                    return new RegexNode(RegexNode.Set, _options, RegexCharClass.NotWordClass);

                case 's':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    if (UseOptionE())
                        return new RegexNode(RegexNode.Set, _options, RegexCharClass.ECMASpaceClass);
                    return new RegexNode(RegexNode.Set, _options, RegexCharClass.SpaceClass);

                case 'S':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    if (UseOptionE())
                        return new RegexNode(RegexNode.Set, _options, RegexCharClass.NotECMASpaceClass);
                    return new RegexNode(RegexNode.Set, _options, RegexCharClass.NotSpaceClass);

                case 'd':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    if (UseOptionE())
                        return new RegexNode(RegexNode.Set, _options, RegexCharClass.ECMADigitClass);
                    return new RegexNode(RegexNode.Set, _options, RegexCharClass.DigitClass);

                case 'D':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    if (UseOptionE())
                        return new RegexNode(RegexNode.Set, _options, RegexCharClass.NotECMADigitClass);
                    return new RegexNode(RegexNode.Set, _options, RegexCharClass.NotDigitClass);

                case 'p':
                case 'P':
                    MoveRight();
                    if (scanOnly)
                        return null;
                    cc = new RegexCharClass();
                    cc.AddCategoryFromName(ParseProperty(), (ch != 'p'), UseOptionI(), _pattern);
                    if (UseOptionI())
                        cc.AddLowercase(_culture);

                    return new RegexNode(RegexNode.Set, _options, cc.ToStringClass());

                default:
                    return ScanBasicBackslash(scanOnly);
            }
        }


        private RegexNode ScanBasicBackslash(bool scanOnly)
        {
            if (CharsRight() == 0)
                throw MakeException(SR.IllegalEndEscape);

            char ch;
            bool angled = false;
            char close = '\0';
            int backpos;

            backpos = Textpos();
            ch = RightChar();



            if (ch == 'k')
            {
                if (CharsRight() >= 2)
                {
                    MoveRight();
                    ch = RightCharMoveRight();

                    if (ch == '<' || ch == '\'')
                    {
                        angled = true;
                        close = (ch == '\'') ? '\'' : '>';
                    }
                }

                if (!angled || CharsRight() <= 0)
                    throw MakeException(SR.MalformedNameRef);

                ch = RightChar();
            }



            else if ((ch == '<' || ch == '\'') && CharsRight() > 1)
            {
                angled = true;
                close = (ch == '\'') ? '\'' : '>';

                MoveRight();
                ch = RightChar();
            }



            if (angled && ch >= '0' && ch <= '9')
            {
                int capnum = ScanDecimal();

                if (CharsRight() > 0 && RightCharMoveRight() == close)
                {
                    if (scanOnly)
                        return null;
                    if (IsCaptureSlot(capnum))
                        return new RegexNode(RegexNode.Ref, _options, capnum);
                    else
                        throw MakeException(SR.Format(SR.UndefinedBackref, capnum.ToString(CultureInfo.CurrentCulture)));
                }
            }



            else if (!angled && ch >= '1' && ch <= '9')
            {
                if (UseOptionE())
                {
                    int capnum = -1;
                    int newcapnum = (int)(ch - '0');
                    int pos = Textpos() - 1;
                    while (newcapnum <= _captop)
                    {
                        if (IsCaptureSlot(newcapnum) && (_caps == null || (int)_caps[newcapnum] < pos))
                            capnum = newcapnum;
                        MoveRight();
                        if (CharsRight() == 0 || (ch = RightChar()) < '0' || ch > '9')
                            break;
                        newcapnum = newcapnum * 10 + (int)(ch - '0');
                    }
                    if (capnum >= 0)
                        return scanOnly ? null : new RegexNode(RegexNode.Ref, _options, capnum);
                }
                else
                {
                    int capnum = ScanDecimal();
                    if (scanOnly)
                        return null;
                    if (IsCaptureSlot(capnum))
                        return new RegexNode(RegexNode.Ref, _options, capnum);
                    else if (capnum <= 9)
                        throw MakeException(SR.Format(SR.UndefinedBackref, capnum.ToString(CultureInfo.CurrentCulture)));
                }
            }




            else if (angled && RegexCharClass.IsWordChar(ch))
            {
                string capname = ScanCapname();

                if (CharsRight() > 0 && RightCharMoveRight() == close)
                {
                    if (scanOnly)
                        return null;
                    if (IsCaptureName(capname))
                        return new RegexNode(RegexNode.Ref, _options, CaptureSlotFromName(capname));
                    else
                        throw MakeException(SR.Format(SR.UndefinedNameRef, capname));
                }
            }



            Textto(backpos);
            ch = ScanCharEscape();

            if (UseOptionI())
                ch = _culture.TextInfo.ToLower(ch);

            return scanOnly ? null : new RegexNode(RegexNode.One, _options, ch);
        }


        private RegexNode ScanDollar()
        {
            if (CharsRight() == 0)
                return new RegexNode(RegexNode.One, _options, '$');

            char ch = RightChar();
            bool angled;
            int backpos = Textpos();
            int lastEndPos = backpos;



            if (ch == '{' && CharsRight() > 1)
            {
                angled = true;
                MoveRight();
                ch = RightChar();
            }
            else
            {
                angled = false;
            }



            if (ch >= '0' && ch <= '9')
            {
                if (!angled && UseOptionE())
                {
                    int capnum = -1;
                    int newcapnum = (int)(ch - '0');
                    MoveRight();
                    if (IsCaptureSlot(newcapnum))
                    {
                        capnum = newcapnum;
                        lastEndPos = Textpos();
                    }

                    while (CharsRight() > 0 && (ch = RightChar()) >= '0' && ch <= '9')
                    {
                        int digit = (int)(ch - '0');
                        if (newcapnum > (MaxValueDiv10) || (newcapnum == (MaxValueDiv10) && digit > (MaxValueMod10)))
                            throw MakeException(SR.CaptureGroupOutOfRange);

                        newcapnum = newcapnum * 10 + digit;

                        MoveRight();
                        if (IsCaptureSlot(newcapnum))
                        {
                            capnum = newcapnum;
                            lastEndPos = Textpos();
                        }
                    }
                    Textto(lastEndPos);
                    if (capnum >= 0)
                        return new RegexNode(RegexNode.Ref, _options, capnum);
                }
                else
                {
                    int capnum = ScanDecimal();
                    if (!angled || CharsRight() > 0 && RightCharMoveRight() == '}')
                    {
                        if (IsCaptureSlot(capnum))
                            return new RegexNode(RegexNode.Ref, _options, capnum);
                    }
                }
            }
            else if (angled && RegexCharClass.IsWordChar(ch))
            {
                string capname = ScanCapname();

                if (CharsRight() > 0 && RightCharMoveRight() == '}')
                {
                    if (IsCaptureName(capname))
                        return new RegexNode(RegexNode.Ref, _options, CaptureSlotFromName(capname));
                }
            }
            else if (!angled)
            {
                int capnum = 1;

                switch (ch)
                {
                    case '$':
                        MoveRight();
                        return new RegexNode(RegexNode.One, _options, '$');

                    case '&':
                        capnum = 0;
                        break;

                    case '`':
                        capnum = RegexReplacement.LeftPortion;
                        break;

                    case '\'':
                        capnum = RegexReplacement.RightPortion;
                        break;

                    case '+':
                        capnum = RegexReplacement.LastGroup;
                        break;

                    case '_':
                        capnum = RegexReplacement.WholeString;
                        break;
                }

                if (capnum != 1)
                {
                    MoveRight();
                    return new RegexNode(RegexNode.Ref, _options, capnum);
                }
            }



            Textto(backpos);
            return new RegexNode(RegexNode.One, _options, '$');
        }


        private string ScanCapname()
        {
            int startpos = Textpos();

            while (CharsRight() > 0)
            {
                if (!RegexCharClass.IsWordChar(RightCharMoveRight()))
                {
                    MoveLeft();
                    break;
                }
            }

            return _pattern.Substring(startpos, Textpos() - startpos);
        }



        private char ScanOctal()
        {
            int d;
            int i;
            int c;



            c = 3;

            if (c > CharsRight())
                c = CharsRight();

            for (i = 0; c > 0 && unchecked((uint)(d = RightChar() - '0')) <= 7; c -= 1)
            {
                MoveRight();
                i *= 8;
                i += d;
                if (UseOptionE() && i >= 0x20)
                    break;
            }



            i &= 0xFF;

            return (char)i;
        }


        private int ScanDecimal()
        {
            int i = 0;
            int d;

            while (CharsRight() > 0 && unchecked((uint)(d = (char)(RightChar() - '0'))) <= 9)
            {
                MoveRight();

                if (i > (MaxValueDiv10) || (i == (MaxValueDiv10) && d > (MaxValueMod10)))
                    throw MakeException(SR.CaptureGroupOutOfRange);

                i *= 10;
                i += d;
            }

            return i;
        }


        private char ScanHex(int c)
        {
            int i;
            int d;

            i = 0;

            if (CharsRight() >= c)
            {
                for (; c > 0 && ((d = HexDigit(RightCharMoveRight())) >= 0); c -= 1)
                {
                    i *= 0x10;
                    i += d;
                }
            }

            if (c > 0)
                throw MakeException(SR.TooFewHex);

            return (char)i;
        }


        private static int HexDigit(char ch)
        {
            int d;

            if ((uint)(d = ch - '0') <= 9)
                return d;

            if (unchecked((uint)(d = ch - 'a')) <= 5)
                return d + 0xa;

            if ((uint)(d = ch - 'A') <= 5)
                return d + 0xa;

            return -1;
        }


        private char ScanControl()
        {
            char ch;

            if (CharsRight() <= 0)
                throw MakeException(SR.MissingControl);

            ch = RightCharMoveRight();



            if (ch >= 'a' && ch <= 'z')
                ch = (char)(ch - ('a' - 'A'));

            if (unchecked(ch = (char)(ch - '@')) < ' ')
                return ch;

            throw MakeException(SR.UnrecognizedControl);
        }


        private bool IsOnlyTopOption(RegexOptions option)
        {
            return (option == RegexOptions.RightToLeft
                || option == RegexOptions.CultureInvariant
                || option == RegexOptions.ECMAScript
            );
        }


        private void ScanOptions()
        {
            char ch;
            bool off;
            RegexOptions option;

            for (off = false; CharsRight() > 0; MoveRight())
            {
                ch = RightChar();

                if (ch == '-')
                {
                    off = true;
                }
                else if (ch == '+')
                {
                    off = false;
                }
                else
                {
                    option = OptionFromCode(ch);
                    if (option == 0 || IsOnlyTopOption(option))
                        return;

                    if (off)
                        _options &= ~option;
                    else
                        _options |= option;
                }
            }
        }


        private char ScanCharEscape()
        {
            char ch;

            ch = RightCharMoveRight();

            if (ch >= '0' && ch <= '7')
            {
                MoveLeft();
                return ScanOctal();
            }

            switch (ch)
            {
                case 'x':
                    return ScanHex(2);
                case 'u':
                    return ScanHex(4);
                case 'a':
                    return '\u0007';
                case 'b':
                    return '\b';
                case 'e':
                    return '\u001B';
                case 'f':
                    return '\f';
                case 'n':
                    return '\n';
                case 'r':
                    return '\r';
                case 't':
                    return '\t';
                case 'v':
                    return '\u000B';
                case 'c':
                    return ScanControl();
                default:
                    if (!UseOptionE() && RegexCharClass.IsWordChar(ch))
                        throw MakeException(SR.Format(SR.UnrecognizedEscape, ch.ToString()));
                    return ch;
            }
        }


        private string ParseProperty()
        {
            if (CharsRight() < 3)
            {
                throw MakeException(SR.IncompleteSlashP);
            }
            char ch = RightCharMoveRight();
            if (ch != '{')
            {
                throw MakeException(SR.MalformedSlashP);
            }

            int startpos = Textpos();
            while (CharsRight() > 0)
            {
                ch = RightCharMoveRight();
                if (!(RegexCharClass.IsWordChar(ch) || ch == '-'))
                {
                    MoveLeft();
                    break;
                }
            }
            string capname = _pattern.Substring(startpos, Textpos() - startpos);

            if (CharsRight() == 0 || RightCharMoveRight() != '}')
                throw MakeException(SR.IncompleteSlashP);

            return capname;
        }


        private int TypeFromCode(char ch)
        {
            switch (ch)
            {
                case 'b':
                    return UseOptionE() ? RegexNode.ECMABoundary : RegexNode.Boundary;
                case 'B':
                    return UseOptionE() ? RegexNode.NonECMABoundary : RegexNode.Nonboundary;
                case 'A':
                    return RegexNode.Beginning;
                case 'G':
                    return RegexNode.Start;
                case 'Z':
                    return RegexNode.EndZ;
                case 'z':
                    return RegexNode.End;
                default:
                    return RegexNode.Nothing;
            }
        }


        private static RegexOptions OptionFromCode(char ch)
        {

            if (ch >= 'A' && ch <= 'Z')
                ch += (char)('a' - 'A');

            switch (ch)
            {
                case 'i':
                    return RegexOptions.IgnoreCase;
                case 'r':
                    return RegexOptions.RightToLeft;
                case 'm':
                    return RegexOptions.Multiline;
                case 'n':
                    return RegexOptions.ExplicitCapture;
                case 's':
                    return RegexOptions.Singleline;
                case 'x':
                    return RegexOptions.IgnorePatternWhitespace;
#if DEBUG
                case 'd':
                    return RegexOptions.Debug;
#endif
                case 'e':
                    return RegexOptions.ECMAScript;
                default:
                    return 0;
            }
        }


        private void CountCaptures()
        {
            char ch;

            NoteCaptureSlot(0, 0);

            _autocap = 1;

            while (CharsRight() > 0)
            {
                int pos = Textpos();
                ch = RightCharMoveRight();
                switch (ch)
                {
                    case '\\':
                        if (CharsRight() > 0)
                            ScanBackslash(scanOnly: true);
                        break;

                    case '#':
                        if (UseOptionX())
                        {
                            MoveLeft();
                            ScanBlank();
                        }
                        break;

                    case '[':
                        ScanCharClass(caseInsensitive: false, scanOnly: true);
                        break;

                    case ')':
                        if (!EmptyOptionsStack())
                            PopOptions();
                        break;

                    case '(':
                        if (CharsRight() >= 2 && RightChar(1) == '#' && RightChar() == '?')
                        {
                            MoveLeft();
                            ScanBlank();
                        }
                        else
                        {
                            PushOptions();
                            if (CharsRight() > 0 && RightChar() == '?')
                            {

                                MoveRight();

                                if (CharsRight() > 1 && (RightChar() == '<' || RightChar() == '\''))
                                {


                                    MoveRight();
                                    ch = RightChar();

                                    if (ch != '0' && RegexCharClass.IsWordChar(ch))
                                    {


                                        if (ch >= '1' && ch <= '9')
                                            NoteCaptureSlot(ScanDecimal(), pos);
                                        else
                                            NoteCaptureName(ScanCapname(), pos);
                                    }
                                }
                                else
                                {



                                    ScanOptions();

                                    if (CharsRight() > 0)
                                    {
                                        if (RightChar() == ')')
                                        {

                                            MoveRight();
                                            PopKeepOptions();
                                        }
                                        else if (RightChar() == '(')
                                        {


                                            _ignoreNextParen = true;


                                            break;
                                        }
                                    }
                                }
                            }
                            else
                            {
                                if (!UseOptionN() && !_ignoreNextParen)
                                    NoteCaptureSlot(_autocap++, pos);
                            }
                        }

                        _ignoreNextParen = false;
                        break;
                }
            }

            AssignNameSlots();
        }


        private void NoteCaptureSlot(int i, int pos)
        {
            if (!_caps.ContainsKey(i))
            {


                _caps.Add(i, pos);
                _capcount++;

                if (_captop <= i)
                {
                    if (i == int.MaxValue)
                        _captop = i;
                    else
                        _captop = i + 1;
                }
            }
        }


        private void NoteCaptureName(string name, int pos)
        {
            if (_capnames == null)
            {
                _capnames = new Hashtable();
                _capnamelist = new List<string>();
            }

            if (!_capnames.ContainsKey(name))
            {
                _capnames.Add(name, pos);
                _capnamelist.Add(name);
            }
        }


        private void NoteCaptures(Hashtable caps, int capsize, Hashtable capnames)
        {
            _caps = caps;
            _capsize = capsize;
            _capnames = capnames;
        }


        private void AssignNameSlots()
        {
            if (_capnames != null)
            {
                for (int i = 0; i < _capnamelist.Count; i++)
                {
                    while (IsCaptureSlot(_autocap))
                        _autocap++;
                    string name = _capnamelist[i];
                    int pos = (int)_capnames[name];
                    _capnames[name] = _autocap;
                    NoteCaptureSlot(_autocap, pos);

                    _autocap++;
                }
            }



            if (_capcount < _captop)
            {
                _capnumlist = new int[_capcount];
                int i = 0;


                IDictionaryEnumerator de = _caps.GetEnumerator();
                while (de.MoveNext())
                {
                    _capnumlist[i++] = (int)de.Key;
                }

                Array.Sort(_capnumlist, Comparer<int>.Default);
            }



            if (_capnames != null || _capnumlist != null)
            {
                List<string> oldcapnamelist;
                int next;
                int k = 0;

                if (_capnames == null)
                {
                    oldcapnamelist = null;
                    _capnames = new Hashtable();
                    _capnamelist = new List<string>();
                    next = -1;
                }
                else
                {
                    oldcapnamelist = _capnamelist;
                    _capnamelist = new List<string>();
                    next = (int)_capnames[oldcapnamelist[0]];
                }

                for (int i = 0; i < _capcount; i++)
                {
                    int j = (_capnumlist == null) ? i : (int)_capnumlist[i];

                    if (next == j)
                    {
                        _capnamelist.Add(oldcapnamelist[k++]);
                        next = (k == oldcapnamelist.Count) ? -1 : (int)_capnames[oldcapnamelist[k]];
                    }
                    else
                    {
                        string str = Convert.ToString(j, _culture);
                        _capnamelist.Add(str);
                        _capnames[str] = j;
                    }
                }
            }
        }


        private int CaptureSlotFromName(string capname)
        {
            return (int)_capnames[capname];
        }


        private bool IsCaptureSlot(int i)
        {
            if (_caps != null)
                return _caps.ContainsKey(i);

            return (i >= 0 && i < _capsize);
        }


        private bool IsCaptureName(string capname)
        {
            if (_capnames == null)
                return false;

            return _capnames.ContainsKey(capname);
        }


        private bool UseOptionN()
        {
            return (_options & RegexOptions.ExplicitCapture) != 0;
        }


        private bool UseOptionI()
        {
            return (_options & RegexOptions.IgnoreCase) != 0;
        }


        private bool UseOptionM()
        {
            return (_options & RegexOptions.Multiline) != 0;
        }


        private bool UseOptionS()
        {
            return (_options & RegexOptions.Singleline) != 0;
        }


        private bool UseOptionX()
        {
            return (_options & RegexOptions.IgnorePatternWhitespace) != 0;
        }


        private bool UseOptionE()
        {
            return (_options & RegexOptions.ECMAScript) != 0;
        }

        private const byte Q = 5;
        private const byte S = 4;
        private const byte Z = 3;
        private const byte X = 2;
        private const byte E = 1;


        private static readonly byte[] s_category = new byte[] {

               0,0,0,0,0,0,0,0,0,X,X,0,X,X,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,

               X,0,0,Z,S,0,0,0,S,S,Q,Q,0,0,S,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,Q,

               0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,S,S,0,S,0,

               0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,Q,S,0,0,0};


        private static bool IsSpecial(char ch)
        {
            return (ch <= '|' && s_category[ch] >= S);
        }


        private static bool IsStopperX(char ch)
        {
            return (ch <= '|' && s_category[ch] >= X);
        }


        private static bool IsQuantifier(char ch)
        {
            return (ch <= '{' && s_category[ch] >= Q);
        }

        private bool IsTrueQuantifier()
        {
            int nChars = CharsRight();
            if (nChars == 0)
                return false;
            int startpos = Textpos();
            char ch = CharAt(startpos);
            if (ch != '{')
                return ch <= '{' && s_category[ch] >= Q;
            int pos = startpos;
            while (--nChars > 0 && (ch = CharAt(++pos)) >= '0' && ch <= '9') ;
            if (nChars == 0 || pos - startpos == 1)
                return false;
            if (ch == '}')
                return true;
            if (ch != ',')
                return false;
            while (--nChars > 0 && (ch = CharAt(++pos)) >= '0' && ch <= '9') ;
            return nChars > 0 && ch == '}';
        }


        private static bool IsSpace(char ch)
        {
            return (ch <= ' ' && s_category[ch] == X);
        }


        private static bool IsMetachar(char ch)
        {
            return (ch <= '|' && s_category[ch] >= E);
        }



        private void AddConcatenate(int pos, int cch, bool isReplacement)
        {
            RegexNode node;

            if (cch == 0)
                return;

            if (cch > 1)
            {
                string str = _pattern.Substring(pos, cch);

                if (UseOptionI() && !isReplacement)
                {




                    StringBuilder sb = StringBuilderCache.Acquire(str.Length);
                    for (int i = 0; i < str.Length; i++)
                        sb.Append(_culture.TextInfo.ToLower(str[i]));
                    str = StringBuilderCache.GetStringAndRelease(sb);
                }

                node = new RegexNode(RegexNode.Multi, _options, str);
            }
            else
            {
                char ch = _pattern[pos];

                if (UseOptionI() && !isReplacement)
                    ch = _culture.TextInfo.ToLower(ch);

                node = new RegexNode(RegexNode.One, _options, ch);
            }

            _concatenation.AddChild(node);
        }


        private void PushGroup()
        {
            _group.Next = _stack;
            _alternation.Next = _group;
            _concatenation.Next = _alternation;
            _stack = _concatenation;
        }


        private void PopGroup()
        {
            _concatenation = _stack;
            _alternation = _concatenation.Next;
            _group = _alternation.Next;
            _stack = _group.Next;


            if (_group.Type() == RegexNode.Testgroup && _group.ChildCount() == 0)
            {
                if (_unit == null)
                    throw MakeException(SR.IllegalCondition);

                _group.AddChild(_unit);
                _unit = null;
            }
        }


        private bool EmptyStack()
        {
            return _stack == null;
        }


        private void StartGroup(RegexNode openGroup)
        {
            _group = openGroup;
            _alternation = new RegexNode(RegexNode.Alternate, _options);
            _concatenation = new RegexNode(RegexNode.Concatenate, _options);
        }


        private void AddAlternate()
        {


            if (_group.Type() == RegexNode.Testgroup || _group.Type() == RegexNode.Testref)
            {
                _group.AddChild(_concatenation.ReverseLeft());
            }
            else
            {
                _alternation.AddChild(_concatenation.ReverseLeft());
            }

            _concatenation = new RegexNode(RegexNode.Concatenate, _options);
        }


        private void AddConcatenate()
        {


            _concatenation.AddChild(_unit);
            _unit = null;
        }


        private void AddConcatenate(bool lazy, int min, int max)
        {
            _concatenation.AddChild(_unit.MakeQuantifier(lazy, min, max));
            _unit = null;
        }


        private RegexNode Unit()
        {
            return _unit;
        }


        private void AddUnitOne(char ch)
        {
            if (UseOptionI())
                ch = _culture.TextInfo.ToLower(ch);

            _unit = new RegexNode(RegexNode.One, _options, ch);
        }


        private void AddUnitNotone(char ch)
        {
            if (UseOptionI())
                ch = _culture.TextInfo.ToLower(ch);

            _unit = new RegexNode(RegexNode.Notone, _options, ch);
        }


        private void AddUnitSet(string cc)
        {
            _unit = new RegexNode(RegexNode.Set, _options, cc);
        }


        private void AddUnitNode(RegexNode node)
        {
            _unit = node;
        }


        private void AddUnitType(int type)
        {
            _unit = new RegexNode(type, _options);
        }


        private void AddGroup()
        {
            if (_group.Type() == RegexNode.Testgroup || _group.Type() == RegexNode.Testref)
            {
                _group.AddChild(_concatenation.ReverseLeft());

                if (_group.Type() == RegexNode.Testref && _group.ChildCount() > 2 || _group.ChildCount() > 3)
                    throw MakeException(SR.TooManyAlternates);
            }
            else
            {
                _alternation.AddChild(_concatenation.ReverseLeft());
                _group.AddChild(_alternation);
            }

            _unit = _group;
        }


        private void PushOptions()
        {
            _optionsStack.Add(_options);
        }


        private void PopOptions()
        {
            _options = _optionsStack[_optionsStack.Count - 1];
            _optionsStack.RemoveAt(_optionsStack.Count - 1);
        }


        private bool EmptyOptionsStack()
        {
            return (_optionsStack.Count == 0);
        }


        private void PopKeepOptions()
        {
            _optionsStack.RemoveAt(_optionsStack.Count - 1);
        }


        private ArgumentException MakeException(string message)
        {
            return new ArgumentException(SR.Format(SR.MakeException, _pattern, message));
        }


        private int Textpos()
        {
            return _currentPos;
        }


        private void Textto(int pos)
        {
            _currentPos = pos;
        }


        private char RightCharMoveRight()
        {
            return _pattern[_currentPos++];
        }


        private void MoveRight()
        {
            MoveRight(1);
        }

        private void MoveRight(int i)
        {
            _currentPos += i;
        }


        private void MoveLeft()
        {
            --_currentPos;
        }


        private char CharAt(int i)
        {
            return _pattern[i];
        }


        internal char RightChar()
        {
            return _pattern[_currentPos];
        }


        private char RightChar(int i)
        {
            return _pattern[_currentPos + i];
        }


        private int CharsRight()
        {
            return _pattern.Length - _currentPos;
        }
    }
}
