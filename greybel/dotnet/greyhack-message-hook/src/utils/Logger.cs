using UnityEngine;

namespace GreyHackMessageHook
{
    static class Logger
    {
        public static void Log(object message)
        {
#if DEBUG
            Debug.Log(message);
#endif
        }

        public static void LogError(object message)
        {
#if DEBUG
            Debug.LogError(message);
#endif
        }
    }
}
