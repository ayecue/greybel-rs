



using System.Threading;

namespace MonoCorefx.Text.RegularExpressions
{
    public partial class Regex
    {


        private static readonly TimeSpan s_maximumMatchTimeout = TimeSpan.FromMilliseconds(int.MaxValue - 1);


        private const string DefaultMatchTimeout_ConfigKeyName = "REGEX_DEFAULT_MATCH_TIMEOUT";



        internal static readonly TimeSpan s_defaultMatchTimeout;









        public static readonly TimeSpan InfiniteMatchTimeout = Timeout.InfiniteTimeSpan;


        protected internal TimeSpan internalMatchTimeout;

        static Regex()
        {
            s_defaultMatchTimeout = InitDefaultMatchTimeout();
        }

        public TimeSpan MatchTimeout => internalMatchTimeout;


        protected internal static void ValidateMatchTimeout(TimeSpan matchTimeout)
        {
            if (InfiniteMatchTimeout == matchTimeout)
                return;


            if (TimeSpan.Zero < matchTimeout && matchTimeout <= s_maximumMatchTimeout)
                return;

            throw new ArgumentOutOfRangeException(nameof(matchTimeout));
        }

        private static TimeSpan InitDefaultMatchTimeout()
        {

            AppDomain ad = AppDomain.CurrentDomain;
            object defaultMatchTimeoutObj = ad.GetData(DefaultMatchTimeout_ConfigKeyName);


            if (defaultMatchTimeoutObj == null)
            {
                return InfiniteMatchTimeout;
            }

            if (defaultMatchTimeoutObj is TimeSpan defaultMatchTimeOut)
            {

                try
                {
                    ValidateMatchTimeout(defaultMatchTimeOut);
                }
                catch (ArgumentOutOfRangeException)
                {
                    throw new ArgumentOutOfRangeException(SR.Format(SR.IllegalDefaultRegexMatchTimeoutInAppDomain, DefaultMatchTimeout_ConfigKeyName, defaultMatchTimeOut));
                }

                return defaultMatchTimeOut;
            }

            throw new InvalidCastException(SR.Format(SR.IllegalDefaultRegexMatchTimeoutInAppDomain, DefaultMatchTimeout_ConfigKeyName, defaultMatchTimeoutObj));
        }
    }
}
