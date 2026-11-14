using System.Reflection;

namespace GreyHackTestSupport.Plugin.Bridge.Game;

/// <summary>Defines each private game method called by the bridge in one place.</summary>
internal static class GameMethod
{
    /// <summary>Reports the newest library as a network's update, as replying to the update-libraries mail does.</summary>
    public static readonly MethodInfo PersonaUpdateReportedNetworks = typeof(Persona).GetMethod(
        "UpdateReportedNetworks",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Counts every visible weakness of a library as used up, as the world's first versions are made.</summary>
    public static readonly MethodInfo VersionsMarkVisibleVulnerabilities =
        typeof(VersionsControl).GetMethod(
            "MarkVisibleVulnerabilitiesAsUsed",
            BindingFlags.Instance | BindingFlags.NonPublic
        );

    /// <summary>Releases the next version of a library when its weaknesses were used up.</summary>
    public static readonly MethodInfo VersionsCheckUpdatePrograms =
        typeof(VersionsControl).GetMethod(
            "CheckUpdatePrograms",
            BindingFlags.Instance | BindingFlags.NonPublic
        );
}
