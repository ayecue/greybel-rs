using GreyHackSetupWizard.Plugins;

namespace GreyHackSetupWizard.Setup;

public sealed class PluginSource
{
    private PluginSource(string? assemblyPath, PluginRelease? release)
    {
        AssemblyPath = assemblyPath;
        Release = release;
    }

    public string? AssemblyPath { get; }

    public PluginRelease? Release { get; }

    public static PluginSource FromAssembly(string assemblyPath) => new(assemblyPath, null);

    public static PluginSource FromRelease(PluginRelease release) => new(null, release);
}
