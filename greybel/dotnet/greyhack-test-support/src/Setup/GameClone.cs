using GreyHackTestSupport.Platform;

namespace GreyHackTestSupport.Setup;

/// <summary>
/// Work-folder copy of the installed game using APFS cloning on macOS or reflinks elsewhere to share disk blocks.
/// Plugins, configuration, and the single-player world remain independent.
/// </summary>
internal static class GameClone
{
    /// <summary>Creates <paramref name="clone"/> from <paramref name="source"/> if the clone does not exist.</summary>
    public static void Ensure(string source, string clone)
    {
        if (Directory.Exists(clone))
        {
            return;
        }
        string[] options = OperatingSystem.IsMacOS() ? ["-cR"] : ["-a", "--reflink=auto"];
        ExternalProcess copy = ExternalProcess.Run("cp", [.. options, source, clone]);
        if (!copy.Succeeded)
        {
            throw new TestSupportException($"cloning {source} into {clone} failed: {copy.Output}");
        }
    }
}
