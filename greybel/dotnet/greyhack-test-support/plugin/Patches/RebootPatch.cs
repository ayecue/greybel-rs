using System;
using HarmonyLib;

namespace GreyHackTestSupport.Plugin.Patches;

/// <summary>
/// Reports when the game's client has started the player's computer again after a restart. The desktop loading ends
/// a normal start (<c>BootUp.AnimLoadDesktop</c> sends <c>PlayerHelperServer.AutorunServerRpc</c> after it connected
/// the saved Wi-Fi network, <c>BootUp.MostrarDesktopIcons</c>); a restart in safe mode ends when the client has shown
/// the safe-mode terminal, which is the last step of <c>BootUp.AnimScreen</c> (<c>Ventana.SetPID</c>). Before that the
/// boot animation is still running, and another restart would run a second one at the same time.
/// </summary>
public static class RebootPatch
{
    /// <summary>Occurs after the desktop loaded.</summary>
    public static event Action DesktopReady;

    /// <summary>Occurs after the client showed the safe-mode terminal.</summary>
    public static event Action SafeModeReady;

    [HarmonyPatch(typeof(PlayerHelperServer), nameof(PlayerHelperServer.AutorunServerRpc))]
    private static class AutorunServerRpcPatch
    {
        [HarmonyPostfix]
        private static void Postfix()
        {
            DesktopReady?.Invoke();
        }
    }

    [HarmonyPatch(typeof(Ventana), nameof(Ventana.SetPID))]
    private static class SetPidPatch
    {
        [HarmonyPostfix]
        private static void Postfix(Ventana __instance)
        {
            if (__instance is Terminal)
            {
                SafeModeReady?.Invoke();
            }
        }
    }
}
