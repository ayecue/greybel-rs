using HarmonyLib;

namespace GreyHackMessageHook.Tests.Support;

internal static class PatchSetup
{
    public static TheoryData<string> PatchClasses() =>
        new(PluginAssembly.PatchClasses().Select(type => type.FullName!));
}
