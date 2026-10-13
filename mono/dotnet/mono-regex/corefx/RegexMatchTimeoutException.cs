



using System.Runtime.Serialization;

namespace MonoCorefx.Text.RegularExpressions
{
    [Serializable]
#if !MONO
    [System.Runtime.CompilerServices.TypeForwardedFrom("System, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089")]
#endif
    public class RegexMatchTimeoutException : TimeoutException, ISerializable
    {
        public RegexMatchTimeoutException(string regexInput, string regexPattern, TimeSpan matchTimeout)
            : base(SR.RegexMatchTimeoutException_Occurred)
        {
            Input = regexInput;
            Pattern = regexPattern;
            MatchTimeout = matchTimeout;
        }

        public RegexMatchTimeoutException() { }

        public RegexMatchTimeoutException(string message) : base(message) { }

        public RegexMatchTimeoutException(string message, Exception inner) : base(message, inner) { }

        protected RegexMatchTimeoutException(SerializationInfo info, StreamingContext context)
            : base(info, context)
        {
            Input = info.GetString("regexInput");
            Pattern = info.GetString("regexPattern");
            MatchTimeout = new TimeSpan(info.GetInt64("timeoutTicks"));
        }

        void ISerializable.GetObjectData(SerializationInfo info, StreamingContext context)
        {
            base.GetObjectData(info, context);
            info.AddValue("regexInput", Input);
            info.AddValue("regexPattern", Pattern);
            info.AddValue("timeoutTicks", MatchTimeout.Ticks);
        }

        public string Input { get; } = string.Empty;

        public string Pattern { get; } = string.Empty;

        public TimeSpan MatchTimeout { get; } = TimeSpan.FromTicks(-1);
    }
}
