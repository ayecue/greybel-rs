using HarmonyLib;

namespace GreyHackTestSupport.Plugin.Patches;

[HarmonyPatch(typeof(BootUp))]
public static class BootUpPatch
{
    public const string Ready = "GreyHackTestSupport - desktop ready";

    [HarmonyPostfix]
    [HarmonyPatch("ResumeBoot")]
    static void Postfix_ResumeBoot()
    {
        UnityEngine.Debug.Log(Ready);
    }
}
