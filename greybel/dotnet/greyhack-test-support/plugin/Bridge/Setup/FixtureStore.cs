using System.Collections.Generic;
using System.IO;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using Newtonsoft.Json;

namespace GreyHackTestSupport.Plugin.Bridge.Setup;

/// <summary>Persists disposable fixture state so the next player setup can remove it, including after a restart.</summary>
internal sealed class FixtureStore
{
    private static string NetworksPath => GetPluginPath("fixture-networks.json");

    private static string ExploitsPath => GetPluginPath("fixture-exploits.json");

    private readonly List<string> networks = ReadJson<List<string>>(NetworksPath);

    private readonly List<FixtureExploit> exploits = ReadJson<List<FixtureExploit>>(ExploitsPath);

    public void AddNetwork(string publicIp)
    {
        if (!networks.Contains(publicIp))
        {
            networks.Add(publicIp);
            WriteJson(NetworksPath, networks);
        }
    }

    public void AddExploit(string name, string version)
    {
        if (!exploits.Any(exploit => exploit.Name == name && exploit.Version == version))
        {
            exploits.Add(new FixtureExploit(name, version));
            WriteJson(ExploitsPath, exploits);
        }
    }

    /// <summary>Deletes only state created by fixtures without changing real network and library releases.</summary>
    public void Reset()
    {
        if (networks.Count > 0)
        {
            Database.Singleton.DeleteNetworks(networks);
            networks.Clear();
            WriteJson(NetworksPath, networks);
        }

        if (exploits.Count == 0)
        {
            return;
        }

        VersionsControl versions = VersionsControl.Singleton;
        Dictionary<string, Dictionary<string, List<Exploit>>> gameExploits =
            (Dictionary<string, Dictionary<string, List<Exploit>>>)
                GameField.VersionsExploits.GetValue(versions);
        foreach (FixtureExploit fixture in exploits)
        {
            if (
                gameExploits.TryGetValue(fixture.Name, out var byVersion)
                && byVersion.Remove(fixture.Version)
                && byVersion.Count == 0
            )
            {
                gameExploits.Remove(fixture.Name);
            }
        }
        Database.Singleton.UpdateExploits(
            versions.GetLastLibs(),
            gameExploits,
            versions.GetAllLibs()
        );
        exploits.Clear();
        WriteJson(ExploitsPath, exploits);
    }

    private static string GetPluginPath(string name) =>
        Path.Combine(Path.GetDirectoryName(typeof(FixtureStore).Assembly.Location) ?? ".", name);

    private static T ReadJson<T>(string path)
        where T : class, new() =>
        File.Exists(path)
            ? JsonConvert.DeserializeObject<T>(File.ReadAllText(path)) ?? new T()
            : new T();

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

    private sealed class FixtureExploit(string name, string version)
    {
        public string Name { get; } = name;

        public string Version { get; } = version;
    }
}
