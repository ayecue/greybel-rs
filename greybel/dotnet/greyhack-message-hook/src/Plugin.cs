using BepInEx;
using BepInEx.Configuration;
using HarmonyLib;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackMessageHook
{
    [BepInPlugin(PluginInfo.PLUGIN_GUID, PluginInfo.PLUGIN_NAME, PluginInfo.PLUGIN_VERSION)]
    public class Plugin : BaseUnityPlugin
    {
        public static Harmony Harmony = new Harmony("mod.greyhack_message_hook");
        public static ConfigEntry<int> Port;
        public static ConfigEntry<string> FileExtensions;

        void Awake()
        {
            Logger.LogInfo($"Plugin {PluginInfo.PLUGIN_GUID} is loaded!");

            var absent = RelevantGameMemberTable.GetMissingMembers();
            if (absent.Count > 0)
            {
                Logger.LogError(
                    $"The game has none of these members the plugin uses: {string.Join(", ", absent)}. The game changed: update RelevantGameMemberTable."
                );
            }

            string gameVersion = GameVersion.Current;
            if (gameVersion != Proxy.Converter.GameVersion)
            {
                Logger.LogWarning(
                    $"Network contracts are from Grey Hack {Proxy.Converter.GameVersion}, the game is {gameVersion}: messages the plugin decodes may have changed. Run `make contracts` and rebuild."
                );
            }

            Port = Config.Bind(
                "General",
                "Port",
                7777,
                "Port of the listener speaking duplex (protocols/greyhack)"
            );

            FileExtensions = Config.Bind(
                "General",
                "File Extensions",
                "src,gs,ms",
                "Supported file extensions"
            );

            Harmony.PatchAll();
        }
    }

    [HarmonyPatch(typeof(BiosMenu))]
    public static class BiosMenuPatch
    {
        [HarmonyPostfix]
        [HarmonyPatch("Start")]
        static void StartPostfix(BiosMenu __instance)
        {
            Proxy.Start();
        }

        [HarmonyPrefix]
        [HarmonyPatch("OnExitGame")]
        static bool OnExitGamePrefix(BiosMenu __instance)
        {
            Proxy.Stop();
            return true;
        }
    }

    [HarmonyPatch(typeof(PlayerClient))]
    public static class PlayerClientPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("Disconnect")]
        static bool DisconnectPrefix(PlayerClient __instance)
        {
            Proxy.ResetPlayerClient();
            return true;
        }

        [HarmonyPostfix]
        [HarmonyPatch("ConnectToServer")]
        static void ConnectToServerPostfix(PlayerClient __instance)
        {
            Proxy.SetPlayerClient(__instance);
        }

        [HarmonyPrefix]
        [HarmonyPatch("ReceiveData")]
        static bool ReceiveDataPrefix(PlayerClient __instance)
        {
            PlayerClientEx.ReceiveData(__instance);
            return false;
        }
    }

    [HarmonyPatch(typeof(PlayerServer))]
    public static class PlayerServerPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("ReceiveData")]
        static bool ReceiveDataPrefix(PlayerServer __instance)
        {
            PlayerServerEx.ReceiveData(__instance);
            return false;
        }
    }
}
