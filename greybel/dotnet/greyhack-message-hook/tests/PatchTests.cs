using GreyHackMessageHook.Tests.Support;
using HarmonyLib;
using static GreyHackMessageHook.Tests.Support.PatchSetup;

namespace GreyHackMessageHook.Tests;

public class PatchTests
{
    [Theory]
    [MemberData(nameof(PatchClasses), MemberType = typeof(PatchSetup))]
    public void ShouldPatchTheGamesMethods(string patchClass)
    {
        var harmony = new Harmony("tests." + patchClass);

        try
        {
            var methods = harmony.CreateClassProcessor(PluginAssembly.FindType(patchClass)).Patch();

            Assert.NotEmpty(methods);
        }
        finally
        {
            harmony.UnpatchSelf();
        }
    }
}
