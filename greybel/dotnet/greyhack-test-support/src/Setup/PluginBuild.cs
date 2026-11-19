namespace GreyHackTestSupport.Setup;

/// <summary>A BepInEx plugin by its GUID and its assembly for each BepInEx major.</summary>
internal sealed record PluginBuild(string Guid, string StableAssemblyName, string BetaAssemblyName)
{
    public static readonly PluginBuild MessageHook = new(
        "GreyHackMessageHook",
        "GreyHackMessageHook5",
        "GreyHackMessageHook"
    );

    public string GetAssemblyName(BepInExMajor major) =>
        major == BepInExMajor.Stable ? StableAssemblyName : BetaAssemblyName;
}
