using HarmonyLib;

namespace GreyHackTestSupport.Plugin.Patches;

[HarmonyPatch(typeof(PlayerHelperServer), "CrearPlayerPcServerRpc")]
public static class PlayerFixturePatch
{
    [HarmonyPrefix]
    private static void Prefix(
        ref string nombreUser,
        ref string nombreMaquina,
        ref string password,
        ref bool tutoOk,
        ref bool enableGift
    )
    {
        nombreUser = PlayerFixture.TestUser;
        nombreMaquina = PlayerFixture.MachineName;
        password = PlayerFixture.TestUser;
        tutoOk = false;
        enableGift = true;
    }
}
