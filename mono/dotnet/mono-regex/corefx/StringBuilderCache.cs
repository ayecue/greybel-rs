



namespace System.Text
{
    internal static class StringBuilderCache
    {



        private const int MaxBuilderSize = 360;
        private const int DefaultCapacity = 16;





        [ThreadStatic]
        private static StringBuilder t_cachedInstance;

        public static StringBuilder Acquire(int capacity = DefaultCapacity)
        {
            if (capacity <= MaxBuilderSize)
            {
                StringBuilder sb = t_cachedInstance;
                if (sb != null)
                {


                    if (capacity <= sb.Capacity)
                    {
                        t_cachedInstance = null;
                        sb.Clear();
                        return sb;
                    }
                }
            }
            return new StringBuilder(capacity);
        }

        public static void Release(StringBuilder sb)
        {
            if (sb.Capacity <= MaxBuilderSize)
            {
                t_cachedInstance = sb;
            }
        }

        public static string GetStringAndRelease(StringBuilder sb)
        {
            string result = sb.ToString();
            Release(sb);
            return result;
        }
    }
}
