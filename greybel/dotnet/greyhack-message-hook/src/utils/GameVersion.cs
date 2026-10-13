using HarmonyLib;
using Util;

namespace GreyHackMessageHook
{
    public static class GameVersion
    {
        public static string Current =>
            (string)RelevantGameMemberTable.GameVersionField?.GetRawConstantValue();
    }
}
