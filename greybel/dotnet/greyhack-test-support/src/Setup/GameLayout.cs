namespace GreyHackTestSupport.Setup;

/// <summary>
/// Paths to a game folder, its executable, and its BepInEx plugins and configuration, as reported by greyhack-setup.
/// </summary>
internal sealed record GameLayout(string Folder, string Executable)
{
    public string PluginsFolder => Path.Combine(Folder, "BepInEx", "plugins");

    public string ConfigFolder => Path.Combine(Folder, "BepInEx", "config");
}
