using System;
using HarmonyLib;
using NetworkMessages;

namespace GreyHackTestSupport.Plugin.Patches;

/// <summary>
/// Bypasses the new-player install screen. When boot displays it (<c>InstallOS.ShowUserConfig</c>), the client
/// immediately completes <c>InstallOS.FinishInstall</c> with fixture names (<see cref="PlayerFixture"/>). Readiness is
/// reported after the client receives the computer (<c>PlayerClientMethods.CrearPlayerPcClientRpc</c>) and finishes
/// loading the desktop. Before startup scripts run (<c>BootUp.AnimLoadDesktop</c>,
/// <c>PlayerHelperServer.AutorunServerRpc</c>), the desktop reconnects saved Wi-Fi
/// (<c>PlayerComputer.AutoConnectWifi</c>); a new computer has none, so an earlier test network would otherwise be
/// disconnected.
/// </summary>
public static class NewPlayerPatch
{
    /// <summary>Occurs after new-player computer installation and desktop loading complete.</summary>
    public static event Action NewComputer;

    /// <summary>True after the client receives a new computer but before its desktop finishes loading.</summary>
    private static volatile bool loading;

    [HarmonyPatch(typeof(InstallOS), nameof(InstallOS.ShowUserConfig))]
    private static class ShowUserConfigPatch
    {
        [HarmonyPrefix]
        private static bool Prefix()
        {
            UnityEngine.Debug.Log("GreyHackTestSupport - installing the new player's computer");
            MessageServer install = new MessageServer(
                IdServer.CrearPlayerPcServerRpc,
                PlayerFixture.TestUser,
                PlayerFixture.MachineName,
                PlayerFixture.TestUser,
                false,
                true
            );
            PlayerClient.Singleton.SendData(install);

            return false;
        }
    }

    [HarmonyPatch(typeof(PlayerClientMethods), "CrearPlayerPcClientRpc")]
    private static class CrearPlayerPcClientRpcPatch
    {
        [HarmonyPostfix]
        private static void Postfix()
        {
            UnityEngine.Debug.Log("GreyHackTestSupport - the new player's computer is installed");
            loading = true;
        }
    }

    [HarmonyPatch(typeof(PlayerHelperServer), nameof(PlayerHelperServer.AutorunServerRpc))]
    private static class AutorunServerRpcPatch
    {
        [HarmonyPostfix]
        private static void Postfix()
        {
            if (!loading)
            {
                return;
            }

            loading = false;
            UnityEngine.Debug.Log("GreyHackTestSupport - the new player's desktop is loaded");
            NewComputer?.Invoke();
        }
    }
}
