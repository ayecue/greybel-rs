using HarmonyLib;
using UnityEngine;

namespace GreyHackTestSupport.Plugin.Patches;

[HarmonyPatch(typeof(MusicPlayer))]
public static class MusicPlayerPatch
{
    public static void Mute()
    {
        if (!Application.isBatchMode)
        {
            return;
        }
        AudioListener.pause = true;
        AudioListener.volume = 0f;
    }

    [HarmonyPrefix]
    [HarmonyPatch("PlayTrack")]
    static bool Prefix_PlayTrack() => !Application.isBatchMode;
}
