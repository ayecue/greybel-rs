



using SysDebug = System.Diagnostics.Debug;
using System.Collections.Generic;
using System.Collections;
using System.Runtime.CompilerServices;
using System.Threading;

namespace MonoCorefx.Text.RegularExpressions
{
    public partial class Regex
    {
        private const int CacheDictionarySwitchLimit = 10;

        private static int s_cacheSize = 15;


        private static readonly Dictionary<CachedCodeEntryKey, CachedCodeEntry> s_cache = new Dictionary<CachedCodeEntryKey, CachedCodeEntry>(s_cacheSize);

        private static int s_cacheCount = 0;
        private static CachedCodeEntry s_cacheFirst;
        private static CachedCodeEntry s_cacheLast;

        public static int CacheSize
        {
            get
            {
                return s_cacheSize;
            }
            set
            {
                if (value < 0)
                    throw new ArgumentOutOfRangeException(nameof(value));

                lock (s_cache)
                {
                    s_cacheSize = value;
                    while (s_cacheCount > s_cacheSize)
                    {
                        CachedCodeEntry last = s_cacheLast;
                        if (s_cacheCount >= CacheDictionarySwitchLimit)
                        {
                            SysDebug.Assert(s_cache.ContainsKey(last.Key));
                            s_cache.Remove(last.Key);
                        }


                        s_cacheLast = last.Next;
                        if (last.Next != null)
                        {
                            SysDebug.Assert(s_cacheFirst != null);
                            SysDebug.Assert(s_cacheFirst != last);
                            SysDebug.Assert(last.Next.Previous == last);
                            last.Next.Previous = null;
                        }
                        else
                        {
                            SysDebug.Assert(s_cacheFirst == last);
                            s_cacheFirst = null;
                        }

                        s_cacheCount--;
                    }
                }
            }
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        private CachedCodeEntry GetCachedCode(CachedCodeEntryKey key, bool isToAdd)
        {

            CachedCodeEntry first = s_cacheFirst;
            if (first?.Key == key)
                return first;
            if (s_cacheSize == 0)
                return null;

            return GetCachedCodeEntryInternal(key, isToAdd);
        }

        private CachedCodeEntry GetCachedCodeEntryInternal(CachedCodeEntryKey key, bool isToAdd)
        {
            lock (s_cache)
            {

                CachedCodeEntry entry = LookupCachedAndPromote(key);

                if (entry == null && isToAdd && s_cacheSize != 0)
                {
                    entry = new CachedCodeEntry(key, capnames, capslist, _code, caps, capsize, _runnerref, _replref);

                    if (s_cacheFirst != null)
                    {
                        SysDebug.Assert(s_cacheFirst.Next == null);
                        s_cacheFirst.Next = entry;
                        entry.Previous = s_cacheFirst;
                    }
                    s_cacheFirst = entry;

                    s_cacheCount++;
                    if (s_cacheCount >= CacheDictionarySwitchLimit)
                    {
                        if (s_cacheCount == CacheDictionarySwitchLimit)
                            FillCacheDictionary();
                        else
                            s_cache.Add(key, entry);
                        SysDebug.Assert(s_cacheCount == s_cache.Count);
                    }


                    if (s_cacheLast == null)
                    {
                        s_cacheLast = entry;
                    }
                    else if (s_cacheCount > s_cacheSize)
                    {
                        CachedCodeEntry last = s_cacheLast;
                        if (s_cacheCount >= CacheDictionarySwitchLimit)
                        {
                            SysDebug.Assert(s_cache[last.Key] == s_cacheLast);
                            s_cache.Remove(last.Key);
                        }

                        SysDebug.Assert(last.Previous == null);
                        SysDebug.Assert(last.Next != null);
                        SysDebug.Assert(last.Next.Previous == last);
                        last.Next.Previous = null;
                        s_cacheLast = last.Next;
                        s_cacheCount--;
                    }

                }
                return entry;
            }
        }

        private void FillCacheDictionary()
        {
            s_cache.Clear();
            CachedCodeEntry next = s_cacheFirst;
            while (next != null)
            {
                s_cache.Add(next.Key, next);
                next = next.Previous;
            }
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        private static bool TryGetCacheValue(CachedCodeEntryKey key, out CachedCodeEntry entry)
        {
            if (s_cacheCount >= CacheDictionarySwitchLimit)
            {
                SysDebug.Assert((s_cacheFirst != null && s_cacheLast != null && s_cache.Count > 0) ||
                                (s_cacheFirst == null && s_cacheLast == null && s_cache.Count == 0),
                                "Linked list and Dict should be synchronized");
                return s_cache.TryGetValue(key, out entry);
            }

            return TryGetCacheValueSmall(key, out entry);
        }

        private static bool TryGetCacheValueSmall(CachedCodeEntryKey key, out CachedCodeEntry entry)
        {
            entry = s_cacheFirst?.Previous;
            while (entry != null)
            {
                if (entry.Key == key)
                    return true;
                entry = entry.Previous;
            }

            return false;
        }

        private static CachedCodeEntry LookupCachedAndPromote(CachedCodeEntryKey key)
        {
            SysDebug.Assert(Monitor.IsEntered(s_cache));
            if (s_cacheFirst?.Key == key)
                return s_cacheFirst;

            if (TryGetCacheValue(key, out CachedCodeEntry entry))
            {

                SysDebug.Assert(s_cacheFirst != entry, "key should not get s_livecode_first");
                SysDebug.Assert(s_cacheFirst != null, "as Dict has at least one");
                SysDebug.Assert(s_cacheFirst.Next == null);
                SysDebug.Assert(s_cacheFirst.Previous != null);
                SysDebug.Assert(entry.Next != null, "not first so Next should exist");
                SysDebug.Assert(entry.Next.Previous == entry);
                if (s_cacheLast == entry)
                {
                    SysDebug.Assert(entry.Previous == null, "last");
                    s_cacheLast = entry.Next;
                }
                else
                {
                    SysDebug.Assert(entry.Previous != null, "in middle");
                    SysDebug.Assert(entry.Previous.Next == entry);
                    entry.Previous.Next = entry.Next;
                }
                entry.Next.Previous = entry.Previous;

                s_cacheFirst.Next = entry;
                entry.Previous = s_cacheFirst;
                entry.Next = null;
                s_cacheFirst = entry;
            }

            return entry;
        }

        internal readonly struct CachedCodeEntryKey : IEquatable<CachedCodeEntryKey>
        {
            private readonly RegexOptions _options;
            private readonly string _cultureKey;
            private readonly string _pattern;

            public CachedCodeEntryKey(RegexOptions options, string cultureKey, string pattern)
            {
                SysDebug.Assert(cultureKey != null, "Culture must be provided");
                SysDebug.Assert(pattern != null, "Pattern must be provided");

                _options = options;
                _cultureKey = cultureKey;
                _pattern = pattern;
            }

            public override bool Equals(object obj)
            {
                return obj is CachedCodeEntryKey && Equals((CachedCodeEntryKey)obj);
            }

            public bool Equals(CachedCodeEntryKey other)
            {
                return _pattern.Equals(other._pattern) && _options == other._options && _cultureKey.Equals(other._cultureKey);
            }

            public static bool operator ==(CachedCodeEntryKey left, CachedCodeEntryKey right)
            {
                return left.Equals(right);
            }

            public static bool operator !=(CachedCodeEntryKey left, CachedCodeEntryKey right)
            {
                return !left.Equals(right);
            }

            public override int GetHashCode()
            {
                return ((int)_options) ^ _cultureKey.GetHashCode() ^ _pattern.GetHashCode();
            }
        }

        internal sealed class CachedCodeEntry
        {
            public CachedCodeEntry Next;
            public CachedCodeEntry Previous;
            public readonly CachedCodeEntryKey Key;
            public RegexCode Code;
            public readonly Hashtable Caps;
            public readonly Hashtable Capnames;
            public readonly string[] Capslist;
#if FEATURE_COMPILED
            public RegexRunnerFactory Factory;
#endif
            public readonly int Capsize;
            public readonly ExclusiveReference Runnerref;
            public readonly WeakReference<RegexReplacement> ReplRef;

            public CachedCodeEntry(CachedCodeEntryKey key, Hashtable capnames, string[] capslist, RegexCode code,
                Hashtable caps, int capsize, ExclusiveReference runner, WeakReference<RegexReplacement> replref)
            {
                Key = key;
                Capnames = capnames;
                Capslist = capslist;
                Code = code;
                Caps = caps;
                Capsize = capsize;
                Runnerref = runner;
                ReplRef = replref;
            }

#if FEATURE_COMPILED
            public void AddCompiled(RegexRunnerFactory factory)
            {
                Factory = factory;
                Code = null;
            }
#endif
        }
    }
}
