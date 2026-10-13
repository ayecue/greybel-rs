using System.Collections.Generic;
using System.IO;
using GreyHackTestSupport.Plugin.Bridge.Game;
using Newtonsoft.Json;

namespace GreyHackTestSupport.Plugin.Bridge.Setup;

/// <summary>
/// Tracks player-network modifications for restoration during the next player setup if a test did not restore them.
/// Because the game persists these changes, restoration data is also stored beside the plugin to survive game exit.
/// </summary>
internal sealed class WorldRestore(Player player, RouterKernel kernels)
{
    /// <summary>Original router kernels indexed by LAN address.</summary>
    private readonly Dictionary<string, string> replacedKernels = ReadJson<
        Dictionary<string, string>
    >(ReplacedKernelsPath);

    /// <summary>
    /// Original firewall-rule actions for modified routers and switches, indexed by LAN address.
    /// </summary>
    private readonly Dictionary<string, List<HwFirewallRule.Action>> allowedFirewalls = ReadJson<
        Dictionary<string, List<HwFirewallRule.Action>>
    >(AllowedFirewallsPath);

    private static string ReplacedKernelsPath => GetPathBesidePlugin("replaced-kernels.json");

    private static string AllowedFirewallsPath => GetPathBesidePlugin("allowed-firewalls.json");

    /// <summary>Saves the current kernel at <paramref name="localIp"/> if no original is already saved.</summary>
    public void RecordKernel(string localIp)
    {
        if (!replacedKernels.ContainsKey(localIp))
        {
            replacedKernels[localIp] = kernels.Read(localIp);
            WriteJson(ReplacedKernelsPath, replacedKernels);
        }
    }

    /// <summary>Discards the saved kernel at <paramref name="localIp"/> after restoration.</summary>
    public void RemoveRestoredKernel(string localIp, string content)
    {
        if (replacedKernels.TryGetValue(localIp, out string original) && original == content)
        {
            replacedKernels.Remove(localIp);
            WriteJson(ReplacedKernelsPath, replacedKernels);
        }
    }

    /// <summary>Saves firewall-rule actions for <paramref name="localIp"/> if they are not already saved.</summary>
    public void RecordFirewall(string localIp, List<HwFirewallRule.Action> actions)
    {
        if (!allowedFirewalls.ContainsKey(localIp))
        {
            allowedFirewalls[localIp] = actions;
            WriteJson(AllowedFirewallsPath, allowedFirewalls);
        }
    }

    /// <summary>Restores all saved kernels and firewall-rule actions.</summary>
    public void Restore()
    {
        foreach (KeyValuePair<string, string> entry in replacedKernels)
        {
            kernels.Write(entry.Key, entry.Value);
        }
        replacedKernels.Clear();
        WriteJson(ReplacedKernelsPath, replacedKernels);

        if (allowedFirewalls.Count == 0)
        {
            return;
        }

        Router main = player.Computer.GetMainRouter();

        foreach (KeyValuePair<string, List<HwFirewallRule.Action>> entry in allowedFirewalls)
        {
            List<HwFirewallRule> rules = main.GetNetworkLan()
                .FindDevice(entry.Key)
                ?.GetDevice()
                ?.firewallRules;

            for (
                int index = 0;
                rules != null && index < rules.Count && index < entry.Value.Count;
                index++
            )
            {
                rules[index].action = entry.Value[index];
            }
        }

        Database.Singleton.SyncConfigOS(main);
        allowedFirewalls.Clear();
        WriteJson(AllowedFirewallsPath, allowedFirewalls);
    }

    private static string GetPathBesidePlugin(string name) =>
        Path.Combine(Path.GetDirectoryName(typeof(WorldRestore).Assembly.Location) ?? ".", name);

    private static T ReadJson<T>(string path)
        where T : class, new()
    {
        return File.Exists(path)
            ? JsonConvert.DeserializeObject<T>(File.ReadAllText(path)) ?? new T()
            : new T();
    }

    private static void WriteJson<T>(string path, ICollection<T> entries)
    {
        if (entries.Count == 0)
        {
            File.Delete(path);
        }
        else
        {
            File.WriteAllText(path, JsonConvert.SerializeObject(entries));
        }
    }
}
