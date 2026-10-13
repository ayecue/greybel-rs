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
}
