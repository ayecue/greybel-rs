using System.Reflection;
using HarmonyLib;

namespace GreyHackMessageHook.Tests.Support;

internal static class PluginAssembly
{
    private static readonly Assembly Assembly = typeof(Proxy).Assembly;

    public static IEnumerable<Type> PatchClasses() =>
        Assembly
            .GetTypes()
            .Where(type =>
                type.GetCustomAttributes(typeof(HarmonyPatch), inherit: false).Length > 0
            );

    public static Type FindType(string fullName) =>
        Assembly.GetType(fullName)
        ?? throw new ArgumentException($"the plugin has no type {fullName}");
}
