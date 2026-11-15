using System.Reflection;

namespace GreyHackTestSupport.Plugin.Bridge.Game;

/// <summary>Defines each private game method called by the bridge in one place.</summary>
internal static class GameMethod
{
    /// <summary>Allocates the next local address below a gateway.</summary>
    public static readonly MethodInfo NetworkGetNextIp = typeof(NetworkLan).GetMethod(
        "GetNextIP",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Allows the generated path firewalls to reach every main-router service forward.</summary>
    public static readonly MethodInfo NetworkAllowMainServicesFw = typeof(NetworkLan).GetMethod(
        "AllowMainServicesFw",
        BindingFlags.Instance | BindingFlags.NonPublic,
        null,
        System.Type.EmptyTypes,
        null
    );

    /// <summary>Assigns each generated NPC the local address of its registered workstation.</summary>
    public static readonly MethodInfo NetworkUpdateNpcsLocalIps = typeof(NetworkLan).GetMethod(
        "UpdateNpcsLocalIps",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Generates the exploit shelf of a library version.</summary>
    public static readonly MethodInfo VersionsGeneraExploit = typeof(VersionsControl).GetMethod(
        "GeneraExploit",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

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
