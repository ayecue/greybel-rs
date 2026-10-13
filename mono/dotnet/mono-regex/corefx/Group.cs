







namespace MonoCorefx.Text.RegularExpressions
{
    [Serializable]
    public class Group : Capture
    {
        internal static readonly Group s_emptyGroup = new Group(string.Empty, Array.Empty<int>(), 0, string.Empty);

        internal readonly int[] _caps;
        internal int _capcount;
        internal CaptureCollection _capcoll;

        internal Group(string text, int[] caps, int capcount, string name)
            : base(text, capcount == 0 ? 0 : caps[(capcount - 1) * 2],
               capcount == 0 ? 0 : caps[(capcount * 2) - 1])
        {
            _caps = caps;
            _capcount = capcount;
            Name = name;
        }

        public bool Success => _capcount != 0;

        public string Name { get; }

        public CaptureCollection Captures
        {
            get
            {
                if (_capcoll == null)
                    _capcoll = new CaptureCollection(this);

                return _capcoll;
            }
        }

        public static Group Synchronized(Group inner)
        {
            if (inner == null)
                throw new ArgumentNullException(nameof(inner));


            CaptureCollection capcoll = inner.Captures;
            if (inner.Success)
            {
                capcoll.ForceInitialized();
            }

            return inner;
        }
    }
}
