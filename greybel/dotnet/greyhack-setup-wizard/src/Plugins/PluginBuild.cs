using GreyHackSetupWizard.BepInEx;

namespace GreyHackSetupWizard.Plugins;

public sealed record PluginBuild(string Guid, string StableAssemblyName, string BetaAssemblyName)
{
    public static readonly PluginBuild MessageHook = new(
        "GreyHackMessageHook",
        "GreyHackMessageHook5",
        "GreyHackMessageHook"
    );

    public string GetAssemblyName(BepInExMajor major) =>
        major == BepInExMajor.Stable ? StableAssemblyName : BetaAssemblyName;

    public IEnumerable<string> Assemblies =>
        new[] { StableAssemblyName, BetaAssemblyName }.Distinct();
}
