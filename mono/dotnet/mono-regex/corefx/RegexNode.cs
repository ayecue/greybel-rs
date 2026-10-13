








































using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;

namespace MonoCorefx.Text.RegularExpressions
{
    internal sealed class RegexNode
    {




        public const int Oneloop = RegexCode.Oneloop;
        public const int Notoneloop = RegexCode.Notoneloop;
        public const int Setloop = RegexCode.Setloop;

        public const int Onelazy = RegexCode.Onelazy;
        public const int Notonelazy = RegexCode.Notonelazy;
        public const int Setlazy = RegexCode.Setlazy;

        public const int One = RegexCode.One;
        public const int Notone = RegexCode.Notone;
        public const int Set = RegexCode.Set;

        public const int Multi = RegexCode.Multi;
        public const int Ref = RegexCode.Ref;

        public const int Bol = RegexCode.Bol;
        public const int Eol = RegexCode.Eol;
        public const int Boundary = RegexCode.Boundary;
        public const int Nonboundary = RegexCode.Nonboundary;
        public const int ECMABoundary = RegexCode.ECMABoundary;
        public const int NonECMABoundary = RegexCode.NonECMABoundary;
        public const int Beginning = RegexCode.Beginning;
        public const int Start = RegexCode.Start;
        public const int EndZ = RegexCode.EndZ;
        public const int End = RegexCode.End;






        public const int Nothing = 22;
        public const int Empty = 23;

        public const int Alternate = 24;
        public const int Concatenate = 25;

        public const int Loop = 26;
        public const int Lazyloop = 27;

        public const int Capture = 28;
        public const int Group = 29;
        public const int Require = 30;
        public const int Prevent = 31;
        public const int Greedy = 32;
        public const int Testref = 33;
        public const int Testgroup = 34;

        public int NType;
        public List<RegexNode> Children;
        public string Str;
        public char Ch;
        public int M;
        public int N;
        public readonly RegexOptions Options;
        public RegexNode Next;

        public RegexNode(int type, RegexOptions options)
        {
            NType = type;
            Options = options;
        }

        public RegexNode(int type, RegexOptions options, char ch)
        {
            NType = type;
            Options = options;
            Ch = ch;
        }

        public RegexNode(int type, RegexOptions options, string str)
        {
            NType = type;
            Options = options;
            Str = str;
        }

        public RegexNode(int type, RegexOptions options, int m)
        {
            NType = type;
            Options = options;
            M = m;
        }

        public RegexNode(int type, RegexOptions options, int m, int n)
        {
            NType = type;
            Options = options;
            M = m;
            N = n;
        }

        public bool UseOptionR()
        {
            return (Options & RegexOptions.RightToLeft) != 0;
        }

        public RegexNode ReverseLeft()
        {
            if (UseOptionR() && NType == Concatenate && Children != null)
            {
                Children.Reverse(0, Children.Count);
            }

            return this;
        }

        private void MakeRep(int type, int min, int max)
        {
            NType += (type - One);
            M = min;
            N = max;
        }

        private RegexNode Reduce()
        {
            RegexNode n;

            switch (Type())
            {
                case Alternate:
                    n = ReduceAlternation();
                    break;

                case Concatenate:
                    n = ReduceConcatenation();
                    break;

                case Loop:
                case Lazyloop:
                    n = ReduceRep();
                    break;

                case Group:
                    n = ReduceGroup();
                    break;

                case Set:
                case Setloop:
                    n = ReduceSet();
                    break;

                default:
                    n = this;
                    break;
            }

            return n;
        }

        private RegexNode StripEnation(int emptyType)
        {
            switch (ChildCount())
            {
                case 0:
                    return new RegexNode(emptyType, Options);
                case 1:
                    return Child(0);
                default:
                    return this;
            }
        }

        private RegexNode ReduceGroup()
        {
            RegexNode u;

            for (u = this; u.Type() == Group;)
                u = u.Child(0);

            return u;
        }

        private RegexNode ReduceRep()
        {
            RegexNode u = this;
            RegexNode child;
            int type = Type();
            int min = M;
            int max = N;

            for (; ; )
            {
                if (u.ChildCount() == 0)
                    break;

                child = u.Child(0);


                if (child.Type() != type)
                {
                    int childType = child.Type();

                    if (!(childType >= Oneloop && childType <= Setloop && type == Loop ||
                          childType >= Onelazy && childType <= Setlazy && type == Lazyloop))
                        break;
                }



                if (u.M == 0 && child.M > 1 || child.N < child.M * 2)
                    break;

                u = child;
                if (u.M > 0)
                    u.M = min = ((int.MaxValue - 1) / u.M < min) ? int.MaxValue : u.M * min;
                if (u.N > 0)
                    u.N = max = ((int.MaxValue - 1) / u.N < max) ? int.MaxValue : u.N * max;
            }

            return min == int.MaxValue ? new RegexNode(Nothing, Options) : u;
        }

        private RegexNode ReduceSet()
        {


            if (RegexCharClass.IsEmpty(Str))
            {
                NType = Nothing;
                Str = null;
            }
            else if (RegexCharClass.IsSingleton(Str))
            {
                Ch = RegexCharClass.SingletonChar(Str);
                Str = null;
                NType += (One - Set);
            }
            else if (RegexCharClass.IsSingletonInverse(Str))
            {
                Ch = RegexCharClass.SingletonChar(Str);
                Str = null;
                NType += (Notone - Set);
            }

            return this;
        }

        private RegexNode ReduceAlternation()
        {
            if (Children == null)
                return new RegexNode(Nothing, Options);

            bool wasLastSet = false;
            bool lastNodeCannotMerge = false;
            RegexOptions optionsLast = 0;
            RegexOptions optionsAt;
            int i;
            int j;
            RegexNode at;
            RegexNode prev;

            for (i = 0, j = 0; i < Children.Count; i++, j++)
            {
                at = Children[i];

                if (j < i)
                    Children[j] = at;

                for (; ; )
                {
                    if (at.NType == Alternate)
                    {
                        for (int k = 0; k < at.Children.Count; k++)
                            at.Children[k].Next = this;

                        Children.InsertRange(i + 1, at.Children);
                        j--;
                    }
                    else if (at.NType == Set || at.NType == One)
                    {

                        optionsAt = at.Options & (RegexOptions.RightToLeft | RegexOptions.IgnoreCase);


                        if (at.NType == Set)
                        {
                            if (!wasLastSet || optionsLast != optionsAt || lastNodeCannotMerge || !RegexCharClass.IsMergeable(at.Str))
                            {
                                wasLastSet = true;
                                lastNodeCannotMerge = !RegexCharClass.IsMergeable(at.Str);
                                optionsLast = optionsAt;
                                break;
                            }
                        }
                        else if (!wasLastSet || optionsLast != optionsAt || lastNodeCannotMerge)
                        {
                            wasLastSet = true;
                            lastNodeCannotMerge = false;
                            optionsLast = optionsAt;
                            break;
                        }




                        j--;
                        prev = Children[j];

                        RegexCharClass prevCharClass;
                        if (prev.NType == One)
                        {
                            prevCharClass = new RegexCharClass();
                            prevCharClass.AddChar(prev.Ch);
                        }
                        else
                        {
                            prevCharClass = RegexCharClass.Parse(prev.Str);
                        }

                        if (at.NType == One)
                        {
                            prevCharClass.AddChar(at.Ch);
                        }
                        else
                        {
                            RegexCharClass atCharClass = RegexCharClass.Parse(at.Str);
                            prevCharClass.AddCharClass(atCharClass);
                        }

                        prev.NType = Set;
                        prev.Str = prevCharClass.ToStringClass();
                    }
                    else if (at.NType == Nothing)
                    {
                        j--;
                    }
                    else
                    {
                        wasLastSet = false;
                        lastNodeCannotMerge = false;
                    }
                    break;
                }
            }

            if (j < i)
                Children.RemoveRange(j, i - j);

            return StripEnation(Nothing);
        }

        private RegexNode ReduceConcatenation()
        {
            if (Children == null)
                return new RegexNode(Empty, Options);

            bool wasLastString = false;
            RegexOptions optionsLast = 0;
            RegexOptions optionsAt;
            int i;
            int j;

            for (i = 0, j = 0; i < Children.Count; i++, j++)
            {
                RegexNode at;
                RegexNode prev;

                at = Children[i];

                if (j < i)
                    Children[j] = at;

                if (at.NType == Concatenate &&
                    ((at.Options & RegexOptions.RightToLeft) == (Options & RegexOptions.RightToLeft)))
                {
                    for (int k = 0; k < at.Children.Count; k++)
                        at.Children[k].Next = this;

                    Children.InsertRange(i + 1, at.Children);
                    j--;
                }
                else if (at.NType == Multi ||
                         at.NType == One)
                {

                    optionsAt = at.Options & (RegexOptions.RightToLeft | RegexOptions.IgnoreCase);

                    if (!wasLastString || optionsLast != optionsAt)
                    {
                        wasLastString = true;
                        optionsLast = optionsAt;
                        continue;
                    }

                    prev = Children[--j];

                    if (prev.NType == One)
                    {
                        prev.NType = Multi;
                        prev.Str = Convert.ToString(prev.Ch, CultureInfo.InvariantCulture);
                    }

                    if ((optionsAt & RegexOptions.RightToLeft) == 0)
                    {
                        if (at.NType == One)
                            prev.Str += at.Ch.ToString();
                        else
                            prev.Str += at.Str;
                    }
                    else
                    {
                        if (at.NType == One)
                            prev.Str = at.Ch.ToString() + prev.Str;
                        else
                            prev.Str = at.Str + prev.Str;
                    }
                }
                else if (at.NType == Empty)
                {
                    j--;
                }
                else
                {
                    wasLastString = false;
                }
            }

            if (j < i)
                Children.RemoveRange(j, i - j);

            return StripEnation(Empty);
        }

        public RegexNode MakeQuantifier(bool lazy, int min, int max)
        {
            if (min == 0 && max == 0)
                return new RegexNode(Empty, Options);

            if (min == 1 && max == 1)
                return this;

            switch (NType)
            {
                case One:
                case Notone:
                case Set:
                    MakeRep(lazy ? Onelazy : Oneloop, min, max);
                    return this;

                default:
                    var result = new RegexNode(lazy ? Lazyloop : Loop, Options, min, max);
                    result.AddChild(this);
                    return result;
            }
        }

        public void AddChild(RegexNode newChild)
        {
            if (Children == null)
                Children = new List<RegexNode>(4);

            RegexNode reducedChild = newChild.Reduce();
            Children.Add(reducedChild);
            reducedChild.Next = this;
        }

        public RegexNode Child(int i)
        {
            return Children[i];
        }

        public int ChildCount()
        {
            return Children == null ? 0 : Children.Count;
        }

        public int Type()
        {
            return NType;
        }

#if DEBUG
        private const string Space = "                                ";
        private static readonly string[] s_typeStr = new string[] {
            "Onerep", "Notonerep", "Setrep",
            "Oneloop", "Notoneloop", "Setloop",
            "Onelazy", "Notonelazy", "Setlazy",
            "One", "Notone", "Set",
            "Multi", "Ref",
            "Bol", "Eol", "Boundary", "Nonboundary",
            "ECMABoundary", "NonECMABoundary",
            "Beginning", "Start", "EndZ", "End",
            "Nothing", "Empty",
            "Alternate", "Concatenate",
            "Loop", "Lazyloop",
            "Capture", "Group", "Require", "Prevent", "Greedy",
            "Testref", "Testgroup"};

        private string Description()
        {
            StringBuilder ArgSb = new StringBuilder();

            ArgSb.Append(s_typeStr[NType]);

            if ((Options & RegexOptions.ExplicitCapture) != 0)
                ArgSb.Append("-C");
            if ((Options & RegexOptions.IgnoreCase) != 0)
                ArgSb.Append("-I");
            if ((Options & RegexOptions.RightToLeft) != 0)
                ArgSb.Append("-L");
            if ((Options & RegexOptions.Multiline) != 0)
                ArgSb.Append("-M");
            if ((Options & RegexOptions.Singleline) != 0)
                ArgSb.Append("-S");
            if ((Options & RegexOptions.IgnorePatternWhitespace) != 0)
                ArgSb.Append("-X");
            if ((Options & RegexOptions.ECMAScript) != 0)
                ArgSb.Append("-E");

            switch (NType)
            {
                case Oneloop:
                case Notoneloop:
                case Onelazy:
                case Notonelazy:
                case One:
                case Notone:
                    ArgSb.Append("(Ch = " + RegexCharClass.CharDescription(Ch) + ")");
                    break;
                case Capture:
                    ArgSb.Append("(index = " + M.ToString(CultureInfo.InvariantCulture) + ", unindex = " + N.ToString(CultureInfo.InvariantCulture) + ")");
                    break;
                case Ref:
                case Testref:
                    ArgSb.Append("(index = " + M.ToString(CultureInfo.InvariantCulture) + ")");
                    break;
                case Multi:
                    ArgSb.Append("(String = " + Str + ")");
                    break;
                case Set:
                case Setloop:
                case Setlazy:
                    ArgSb.Append("(Set = " + RegexCharClass.SetDescription(Str) + ")");
                    break;
            }

            switch (NType)
            {
                case Oneloop:
                case Notoneloop:
                case Onelazy:
                case Notonelazy:
                case Setloop:
                case Setlazy:
                case Loop:
                case Lazyloop:
                    ArgSb.Append("(Min = " + M.ToString(CultureInfo.InvariantCulture) + ", Max = " + (N == int.MaxValue ? "inf" : Convert.ToString(N, CultureInfo.InvariantCulture)) + ")");
                    break;
            }

            return ArgSb.ToString();
        }

        public void Dump()
        {
            List<int> Stack = new List<int>();
            RegexNode CurNode;
            int CurChild;

            CurNode = this;
            CurChild = 0;

            Debug.WriteLine(CurNode.Description());

            for (; ; )
            {
                if (CurNode.Children != null && CurChild < CurNode.Children.Count)
                {
                    Stack.Add(CurChild + 1);
                    CurNode = CurNode.Children[CurChild];
                    CurChild = 0;

                    int Depth = Stack.Count;
                    if (Depth > 32)
                        Depth = 32;

                    Debug.WriteLine(Space.Substring(0, Depth) + CurNode.Description());
                }
                else
                {
                    if (Stack.Count == 0)
                        break;

                    CurChild = Stack[Stack.Count - 1];
                    Stack.RemoveAt(Stack.Count - 1);
                    CurNode = CurNode.Next;
                }
            }
        }
#endif
    }
}
