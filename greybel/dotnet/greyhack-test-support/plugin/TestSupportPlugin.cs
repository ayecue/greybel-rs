using BepInEx;
using GreyHackTestSupport.Plugin.Bridge;
using GreyHackTestSupport.Plugin.Patches;
using HarmonyLib;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackTestSupport.Plugin;

[BepInPlugin(PluginInfo.PLUGIN_GUID, PluginInfo.PLUGIN_NAME, PluginInfo.PLUGIN_VERSION)]
public class TestSupportPlugin : BaseUnityPlugin
{
    private readonly Harmony harmony = new Harmony("mod.greyhack_test_support");
    private TestSupportBridge bridge;

    void Awake()
    {
        MusicPlayerPatch.Mute();
        ServerPortPatch.Port = Config
            .Bind("General", "ServerPort", 21200, "Port of the game's single-player server")
            .Value;
        harmony.PatchAll();
        bridge = new TestSupportBridge(
            Config
                .Bind("General", "TestPort", 0, "Port of the test-only setup/query protocol")
                .Value
        );
    }

    void OnDestroy()
    {
        bridge?.Dispose();
    }
}
