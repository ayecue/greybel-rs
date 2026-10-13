






using System.Collections;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Reflection;
using System.Reflection.Emit;
#if FEATURE_COMPILED
using System.Runtime.CompilerServices;
#endif
using System.Runtime.Serialization;

namespace MonoCorefx.Text.RegularExpressions
{
    public partial class Regex : ISerializable
    {
        internal const int MaxOptionShift = 10;

        protected internal string pattern;
        protected internal RegexOptions roptions;
        protected internal RegexRunnerFactory factory;
        protected internal Hashtable caps;
        protected internal Hashtable capnames;
        protected internal string[] capslist;
        protected internal int capsize;

        internal ExclusiveReference _runnerref;
        internal WeakReference<RegexReplacement> _replref;
        internal RegexCode _code;
        internal bool _refsInitialized = false;

        protected Regex()
        {
            internalMatchTimeout = s_defaultMatchTimeout;
        }

        public Regex(string pattern)
            : this(pattern, RegexOptions.None, s_defaultMatchTimeout, false)
        {
        }

        public Regex(string pattern, RegexOptions options)
            : this(pattern, options, s_defaultMatchTimeout, false)
        {
        }

        public Regex(string pattern, RegexOptions options, TimeSpan matchTimeout)
            : this(pattern, options, matchTimeout, false)
        {
        }

        protected Regex(SerializationInfo info, StreamingContext context)
            : this(info.GetString("pattern"), (RegexOptions)info.GetInt32("options"))
        {
            throw new PlatformNotSupportedException();
        }

        void ISerializable.GetObjectData(SerializationInfo si, StreamingContext context)
        {
            throw new PlatformNotSupportedException();
        }

        private Regex(string pattern, RegexOptions options, TimeSpan matchTimeout, bool addToCache)
        {
            if (pattern == null)
            {
                throw new ArgumentNullException(nameof(pattern));
            }

            if (options < RegexOptions.None || (((int)options) >> MaxOptionShift) != 0)
            {
                throw new ArgumentOutOfRangeException(nameof(options));
            }

            if ((options & RegexOptions.ECMAScript) != 0
             && (options & ~(RegexOptions.ECMAScript |
                             RegexOptions.IgnoreCase |
                             RegexOptions.Multiline |
                             RegexOptions.Compiled |
                             RegexOptions.CultureInvariant
#if DEBUG
                           | RegexOptions.Debug
#endif
                                               )) != 0)
            {
                throw new ArgumentOutOfRangeException(nameof(options));
            }

            ValidateMatchTimeout(matchTimeout);


            this.pattern = pattern;
            roptions = options;
            internalMatchTimeout = matchTimeout;


            string cultureKey = (options & RegexOptions.CultureInvariant) != 0 ?
                    CultureInfo.InvariantCulture.ToString() :
                    CultureInfo.CurrentCulture.ToString();
            var key = new CachedCodeEntryKey(options, cultureKey, pattern);
            CachedCodeEntry cached = GetCachedCode(key, false);

            if (cached == null)
            {

                RegexTree tree = RegexParser.Parse(pattern, roptions);


                capnames = tree.CapNames;
                capslist = tree.CapsList;
                _code = RegexWriter.Write(tree);
                caps = _code.Caps;
                capsize = _code.CapSize;

                InitializeReferences();

                tree = null;
                if (addToCache)
                    cached = GetCachedCode(key, true);
            }
            else
            {
                caps = cached.Caps;
                capnames = cached.Capnames;
                capslist = cached.Capslist;
                capsize = cached.Capsize;
                _code = cached.Code;
#if FEATURE_COMPILED
                factory = cached.Factory;
#endif


                _runnerref = cached.Runnerref;
                _replref = cached.ReplRef;
                _refsInitialized = true;
            }

#if FEATURE_COMPILED
            if (UseOptionC() && factory == null)
            {
                factory = Compile(_code, roptions);

                if (addToCache && cached != null)
                {
                    cached.AddCompiled(factory);
                }

                _code = null;
            }
#endif
        }

        [CLSCompliant(false)]
        protected IDictionary Caps
        {
            get
            {
                return caps;
            }
            set
            {
                if (value == null)
                    throw new ArgumentNullException(nameof(value));

                caps = value as Hashtable ?? new Hashtable(value);
            }
        }

        [CLSCompliant(false)]
        protected IDictionary CapNames
        {
            get
            {
                return capnames;
            }
            set
            {
                if (value == null)
                    throw new ArgumentNullException(nameof(value));

                capnames = value as Hashtable ?? new Hashtable(value);
            }
        }

#if FEATURE_COMPILED
        [MethodImplAttribute(MethodImplOptions.NoInlining)]
        private RegexRunnerFactory Compile(RegexCode code, RegexOptions roptions)
        {
            return RegexCompiler.Compile(code, roptions);
        }

        public static void CompileToAssembly(RegexCompilationInfo[] regexinfos, AssemblyName assemblyname)
        {
            throw new PlatformNotSupportedException(SR.PlatformNotSupported_CompileToAssembly);
        }

        public static void CompileToAssembly(RegexCompilationInfo[] regexinfos, AssemblyName assemblyname, CustomAttributeBuilder[] attributes)
        {
            throw new PlatformNotSupportedException(SR.PlatformNotSupported_CompileToAssembly);
        }

        public static void CompileToAssembly(RegexCompilationInfo[] regexinfos, AssemblyName assemblyname, CustomAttributeBuilder[] attributes, string resourceFile)
        {
            throw new PlatformNotSupportedException(SR.PlatformNotSupported_CompileToAssembly);
        }
#endif

        public static string Escape(string str)
        {
            if (str == null)
                throw new ArgumentNullException(nameof(str));

            return RegexParser.Escape(str);
        }

        public static string Unescape(string str)
        {
            if (str == null)
                throw new ArgumentNullException(nameof(str));

            return RegexParser.Unescape(str);
        }

        public RegexOptions Options => roptions;

        public bool RightToLeft => UseOptionR();

        public override string ToString() => pattern;


        public string[] GetGroupNames()
        {
            string[] result;

            if (capslist == null)
            {
                int max = capsize;
                result = new string[max];

                for (int i = 0; i < max; i++)
                {
                    result[i] = Convert.ToString(i, CultureInfo.InvariantCulture);
                }
            }
            else
            {
                result = new string[capslist.Length];
                Array.Copy(capslist, 0, result, 0, capslist.Length);
            }

            return result;
        }


        public int[] GetGroupNumbers()
        {
            int[] result;

            if (caps == null)
            {
                int max = capsize;
                result = new int[max];

                for (int i = 0; i < result.Length; i++)
                {
                    result[i] = i;
                }
            }
            else
            {
                result = new int[caps.Count];


                IDictionaryEnumerator de = caps.GetEnumerator();
                while (de.MoveNext())
                {
                    result[(int)de.Value] = (int)de.Key;
                }
            }

            return result;
        }


        public string GroupNameFromNumber(int i)
        {
            if (capslist == null)
            {
                if (i >= 0 && i < capsize)
                    return i.ToString(CultureInfo.InvariantCulture);

                return string.Empty;
            }
            else
            {
                if (caps != null)
                {
                    if (!caps.TryGetValue(i, out i))
                        return string.Empty;
                }

                if (i >= 0 && i < capslist.Length)
                    return capslist[i];

                return string.Empty;
            }
        }


        public int GroupNumberFromName(string name)
        {
            int result = -1;

            if (name == null)
                throw new ArgumentNullException(nameof(name));


            if (capnames != null)
            {
                if (!capnames.TryGetValue(name, out result))
                    return -1;

                return result;
            }


            result = 0;
            for (int i = 0; i < name.Length; i++)
            {
                char ch = name[i];

                if (ch > '9' || ch < '0')
                    return -1;

                result *= 10;
                result += (ch - '0');
            }


            if (result >= 0 && result < capsize)
                return result;

            return -1;
        }

        protected void InitializeReferences()
        {
            if (_refsInitialized)
                throw new NotSupportedException(SR.OnlyAllowedOnce);

            _refsInitialized = true;
            _runnerref = new ExclusiveReference();
            _replref = new WeakReference<RegexReplacement>(null);
        }

        internal Match Run(bool quick, int prevlen, string input, int beginning, int length, int startat)
        {
            if (startat < 0 || startat > input.Length)
                throw new ArgumentOutOfRangeException(nameof(startat), SR.BeginIndexNotNegative);

            if (length < 0 || length > input.Length)
                throw new ArgumentOutOfRangeException(nameof(length), SR.LengthNotNegative);


            RegexRunner runner = _runnerref.Get();


            if (runner == null)
            {

                if (factory != null)
                    runner = factory.CreateInstance();
                else
                    runner = new RegexInterpreter(_code, UseOptionInvariant() ? CultureInfo.InvariantCulture : CultureInfo.CurrentCulture);
            }

            Match match;
            try
            {

                match = runner.Scan(this, input, beginning, beginning + length, startat, prevlen, quick, internalMatchTimeout);
            }
            finally
            {

                _runnerref.Release(runner);
            }

#if DEBUG
            if (Debug && match != null)
                match.Dump();
#endif
            return match;
        }

        protected bool UseOptionC()
        {
#if MOBILE
            return false;
#elif MONO
            if (Environment.GetEnvironmentVariable ("MONO_REGEX_COMPILED_ENABLE") == null)
                return false;
#endif

            return (roptions & RegexOptions.Compiled) != 0;
        }


        protected internal bool UseOptionR() => (roptions & RegexOptions.RightToLeft) != 0;

        internal bool UseOptionInvariant() => (roptions & RegexOptions.CultureInvariant) != 0;

#if DEBUG
        internal bool Debug => (roptions & RegexOptions.Debug) != 0;
#endif
    }
}
