
















using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;

namespace MonoCorefx.Text.RegularExpressions
{
    internal sealed class RegexCode
    {



        public const int Onerep = 0;
        public const int Notonerep = 1;
        public const int Setrep = 2;

        public const int Oneloop = 3;
        public const int Notoneloop = 4;
        public const int Setloop = 5;

        public const int Onelazy = 6;
        public const int Notonelazy = 7;
        public const int Setlazy = 8;

        public const int One = 9;
        public const int Notone = 10;
        public const int Set = 11;

        public const int Multi = 12;
        public const int Ref = 13;

        public const int Bol = 14;
        public const int Eol = 15;
        public const int Boundary = 16;
        public const int Nonboundary = 17;
        public const int Beginning = 18;
        public const int Start = 19;
        public const int EndZ = 20;
        public const int End = 21;

        public const int Nothing = 22;



        public const int Lazybranch = 23;
        public const int Branchmark = 24;
        public const int Lazybranchmark = 25;
        public const int Nullcount = 26;
        public const int Setcount = 27;
        public const int Branchcount = 28;
        public const int Lazybranchcount = 29;
        public const int Nullmark = 30;
        public const int Setmark = 31;
        public const int Capturemark = 32;
        public const int Getmark = 33;
        public const int Setjump = 34;
        public const int Backjump = 35;
        public const int Forejump = 36;
        public const int Testref = 37;
        public const int Goto = 38;

        public const int Prune = 39;
        public const int Stop = 40;

        public const int ECMABoundary = 41;
        public const int NonECMABoundary = 42;


        public const int Mask = 63;
        public const int Rtl = 64;
        public const int Back = 128;
        public const int Back2 = 256;
        public const int Ci = 512;

        public readonly int[] Codes;
        public readonly string[] Strings;
        public readonly int TrackCount;
        public readonly Hashtable Caps;
        public readonly int CapSize;
        public readonly RegexPrefix? FCPrefix;
        public readonly RegexBoyerMoore BMPrefix;
        public readonly int Anchors;
        public readonly bool RightToLeft;

        public RegexCode(int[] codes, List<string> stringlist, int trackcount,
                           Hashtable caps, int capsize,
                           RegexBoyerMoore bmPrefix, RegexPrefix? fcPrefix,
                           int anchors, bool rightToLeft)
        {
            Debug.Assert(codes != null, "codes cannot be null.");
            Debug.Assert(stringlist != null, "stringlist cannot be null.");

            Codes = codes;
            Strings = stringlist.ToArray();
            TrackCount = trackcount;
            Caps = caps;
            CapSize = capsize;
            BMPrefix = bmPrefix;
            FCPrefix = fcPrefix;
            Anchors = anchors;
            RightToLeft = rightToLeft;
        }

        public static bool OpcodeBacktracks(int Op)
        {
            Op &= Mask;

            switch (Op)
            {
                case Oneloop:
                case Notoneloop:
                case Setloop:
                case Onelazy:
                case Notonelazy:
                case Setlazy:
                case Lazybranch:
                case Branchmark:
                case Lazybranchmark:
                case Nullcount:
                case Setcount:
                case Branchcount:
                case Lazybranchcount:
                case Setmark:
                case Capturemark:
                case Getmark:
                case Setjump:
                case Backjump:
                case Forejump:
                case Goto:
                    return true;

                default:
                    return false;
            }
        }

        public static int OpcodeSize(int opcode)
        {
            opcode &= Mask;

            switch (opcode)
            {
                case Nothing:
                case Bol:
                case Eol:
                case Boundary:
                case Nonboundary:
                case ECMABoundary:
                case NonECMABoundary:
                case Beginning:
                case Start:
                case EndZ:
                case End:
                case Nullmark:
                case Setmark:
                case Getmark:
                case Setjump:
                case Backjump:
                case Forejump:
                case Stop:
                    return 1;

                case One:
                case Notone:
                case Multi:
                case Ref:
                case Testref:
                case Goto:
                case Nullcount:
                case Setcount:
                case Lazybranch:
                case Branchmark:
                case Lazybranchmark:
                case Prune:
                case Set:
                    return 2;

                case Capturemark:
                case Branchcount:
                case Lazybranchcount:
                case Onerep:
                case Notonerep:
                case Oneloop:
                case Notoneloop:
                case Onelazy:
                case Notonelazy:
                case Setlazy:
                case Setrep:
                case Setloop:
                    return 3;

                default:
                    throw new ArgumentException(SR.Format(SR.UnexpectedOpcode, opcode.ToString(CultureInfo.CurrentCulture)));
            }
        }

#if DEBUG
        private static readonly string[] s_codeStr = new string[]
        {
            "Onerep", "Notonerep", "Setrep",
            "Oneloop", "Notoneloop", "Setloop",
            "Onelazy", "Notonelazy", "Setlazy",
            "One", "Notone", "Set",
            "Multi", "Ref",
            "Bol", "Eol", "Boundary", "Nonboundary", "Beginning", "Start", "EndZ", "End",
            "Nothing",
            "Lazybranch", "Branchmark", "Lazybranchmark",
            "Nullcount", "Setcount", "Branchcount", "Lazybranchcount",
            "Nullmark", "Setmark", "Capturemark", "Getmark",
            "Setjump", "Backjump", "Forejump", "Testref", "Goto",
            "Prune", "Stop",
#if ECMA
            "ECMABoundary", "NonECMABoundary",
#endif
        };

        private static string OperatorDescription(int Opcode)
        {
            bool isCi = ((Opcode & Ci) != 0);
            bool isRtl = ((Opcode & Rtl) != 0);
            bool isBack = ((Opcode & Back) != 0);
            bool isBack2 = ((Opcode & Back2) != 0);

            return s_codeStr[Opcode & Mask] +
            (isCi ? "-Ci" : "") + (isRtl ? "-Rtl" : "") + (isBack ? "-Back" : "") + (isBack2 ? "-Back2" : "");
        }

        public string OpcodeDescription(int offset)
        {
            StringBuilder sb = new StringBuilder();
            int opcode = Codes[offset];

            sb.AppendFormat("{0:D6} ", offset);
            sb.Append(OpcodeBacktracks(opcode & Mask) ? '*' : ' ');
            sb.Append(OperatorDescription(opcode));
            sb.Append('(');

            opcode &= Mask;

            switch (opcode)
            {
                case One:
                case Notone:
                case Onerep:
                case Notonerep:
                case Oneloop:
                case Notoneloop:
                case Onelazy:
                case Notonelazy:
                    sb.Append("Ch = ");
                    sb.Append(RegexCharClass.CharDescription((char)Codes[offset + 1]));
                    break;

                case Set:
                case Setrep:
                case Setloop:
                case Setlazy:
                    sb.Append("Set = ");
                    sb.Append(RegexCharClass.SetDescription(Strings[Codes[offset + 1]]));
                    break;

                case Multi:
                    sb.Append("String = ");
                    sb.Append(Strings[Codes[offset + 1]]);
                    break;

                case Ref:
                case Testref:
                    sb.Append("Index = ");
                    sb.Append(Codes[offset + 1]);
                    break;

                case Capturemark:
                    sb.Append("Index = ");
                    sb.Append(Codes[offset + 1]);
                    if (Codes[offset + 2] != -1)
                    {
                        sb.Append(", Unindex = ");
                        sb.Append(Codes[offset + 2]);
                    }
                    break;

                case Nullcount:
                case Setcount:
                    sb.Append("Value = ");
                    sb.Append(Codes[offset + 1]);
                    break;

                case Goto:
                case Lazybranch:
                case Branchmark:
                case Lazybranchmark:
                case Branchcount:
                case Lazybranchcount:
                    sb.Append("Addr = ");
                    sb.Append(Codes[offset + 1]);
                    break;
            }

            switch (opcode)
            {
                case Onerep:
                case Notonerep:
                case Oneloop:
                case Notoneloop:
                case Onelazy:
                case Notonelazy:
                case Setrep:
                case Setloop:
                case Setlazy:
                    sb.Append(", Rep = ");
                    if (Codes[offset + 2] == int.MaxValue)
                        sb.Append("inf");
                    else
                        sb.Append(Codes[offset + 2]);
                    break;

                case Branchcount:
                case Lazybranchcount:
                    sb.Append(", Limit = ");
                    if (Codes[offset + 2] == int.MaxValue)
                        sb.Append("inf");
                    else
                        sb.Append(Codes[offset + 2]);
                    break;
            }

            sb.Append(')');

            return sb.ToString();
        }

        public void Dump()
        {
            int i;

            Debug.WriteLine("Direction:  " + (RightToLeft ? "right-to-left" : "left-to-right"));
            Debug.WriteLine("Firstchars: " + (FCPrefix == null ? "n/a" : RegexCharClass.SetDescription(FCPrefix.GetValueOrDefault().Prefix)));
            Debug.WriteLine("Prefix:     " + (BMPrefix == null ? "n/a" : Regex.Escape(BMPrefix.ToString())));
            Debug.WriteLine("Anchors:    " + RegexFCD.AnchorDescription(Anchors));
            Debug.WriteLine("");
            if (BMPrefix != null)
            {
                Debug.WriteLine("BoyerMoore:");
                Debug.WriteLine(BMPrefix.Dump("    "));
            }
            for (i = 0; i < Codes.Length;)
            {
                Debug.WriteLine(OpcodeDescription(i));
                i += OpcodeSize(Codes[i]);
            }

            Debug.WriteLine("");
        }
#endif
    }
}
