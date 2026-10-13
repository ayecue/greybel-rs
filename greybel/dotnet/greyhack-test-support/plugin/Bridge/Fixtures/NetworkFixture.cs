using System;
using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Plugin.Bridge.Setup;
using GreyHackTestSupport.Protocol;
using Newtonsoft.Json;
using ServiceConfig;
using ServicioID = ServiceConfig.ServicioID;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>
/// Creates and locates networks and configures their computers, port forwards, libraries, firewalls, and flood
/// counters. It can also disable networks.
/// </summary>
internal sealed class NetworkFixture(Player player, WorldRestore restore)
{
    /// <summary>
    /// Seeds the address sequence used by <see cref="Find"/> so tests locate the same network on each run.
    /// </summary>
    private const int NetworkSearchSeed = 7;

    public Reply Create(CreateNetworkRpc request)
    {
        Router router =
            ServerMap.Singleton.GetRouter(new IP(request.PublicIp), true)
            ?? throw new InvalidOperationException("Unable to create network: " + request.PublicIp);
        return new Reply.NetworkCreated(
            new NetworkCreatedReply(router.GetPublicIP(), router.GetLocalIP())
        );
    }

    public Reply Spawn(SpawnNetworkRpc request)
    {
        ServerMap.TipoRed kind = (ServerMap.TipoRed)
            Enum.Parse(typeof(ServerMap.TipoRed), request.Kind);
        Router router = ServerMap.Singleton.SpawnRouter(kind);

        return new Reply.NetworkCreated(
            new NetworkCreatedReply(router.GetPublicIP(), router.GetLocalIP())
        );
    }

    /// <summary>Finds the first network with the device in the <see cref="NetworkSearchSeed"/> address sequence.</summary>
    public Reply Find(FindNetworkRpc request)
    {
        ServerMap.TipoRed kind = (ServerMap.TipoRed)
            Enum.Parse(typeof(ServerMap.TipoRed), request.Kind);
        Random random = new Random(NetworkSearchSeed);

        for (int attempt = 0; attempt < 64; attempt++)
        {
            Router router = ServerMap.Singleton.GetRouter(
                new IP(IpGenerator.GenerateRandomIpForType(kind, random)),
                true
            );
            if (
                router != null
                && (
                    request.Device.Length == 0
                    || HasDevice(router.GetNetworkLan().routerDevice, request.Device)
                )
            )
            {
                return new Reply.NetworkCreated(
                    new NetworkCreatedReply(router.GetPublicIP(), router.GetLocalIP())
                );
            }
        }

        throw new InvalidOperationException(
            "No " + request.Kind + " network has a " + request.Device
        );
    }

    /// <summary>Finds an unvisited network of the requested kind whose router forwards a port to the service.</summary>
    public Reply FindNew(FindNewNetworkRpc request)
    {
        ServerMap.TipoRed kind = (ServerMap.TipoRed)
            Enum.Parse(typeof(ServerMap.TipoRed), request.Kind);
        ServicioID service = (ServicioID)Enum.Parse(typeof(ServicioID), request.Service, true);
        Random random = new Random(Guid.NewGuid().GetHashCode());

        for (int attempt = 0; attempt < 64; attempt++)
        {
            string address = IpGenerator.GenerateRandomIpForType(kind, random);
            if (Database.Singleton.GetRouterNode(address) != null)
            {
                continue;
            }
            Router router = ServerMap.Singleton.GetRouter(new IP(address), true);
            if (
                router != null
                && router
                    .GetConfigOS()
                    .puertos.allPorts.Any(port =>
                        !string.IsNullOrEmpty(port.lanIP)
                        && router.GetPortVersion(port).Split(' ')[0] == service.ToString()
                    )
            )
            {
                return new Reply.NetworkCreated(
                    new NetworkCreatedReply(router.GetPublicIP(), router.GetLocalIP())
                );
            }
        }

        throw new InvalidOperationException(
            "No new " + request.Kind + " network forwards " + request.Service
        );
    }

    public Reply Disable(DisableNetworkRpc request)
    {
        Router router =
            ServerMap.Singleton.GetRouter(new IP(request.PublicIp), false)
            ?? throw new InvalidOperationException("No network at " + request.PublicIp);

        router.GetNetworkLan().disabledNetwork = request.Disabled;
        Database.Singleton.SyncConfigOS(router);
        return new Reply.Applied();
    }

    /// <summary>Adds a main-router port forward to the service on the computer at <c>request.LocalIp</c>.</summary>
    public Reply ForwardService(ForwardServiceRpc request)
    {
        Computer target = player.GetLanComputer(request.LocalIp);
        Router main = player.Server.GetComputer().GetMainRouter();
        Puerto port = new Puerto(
            (ServicioID)Enum.Parse(typeof(ServicioID), request.Service, true),
            request.InternalPort,
            request.PublicPort,
            false,
            true,
            target.GetLocalIP()
        );

        foreach (Router router in player.GetRouterInstances(main.GetLocalIP()))
        {
            router.GetConfigOS().puertos.AddPuerto(port, target.GetLocalIP());
            Database.Singleton.SyncConfigOS(router);
        }
        return new Reply.Applied();
    }

    /// <summary>
    /// Stops running scripts on the player's computer or the computer at <c>request.LocalIp</c>, then installs the
    /// game-generated library version with the requested reward and fewest requirements.
    /// </summary>
    public Reply InstallLibrary(InstallLibraryRpc request)
    {
        Library.Vulnerabilidad.HackResult reward = (Library.Vulnerabilidad.HackResult)
            request.Reward;
        string selection = null;
        int fewest = int.MaxValue;

        foreach (string hash in VersionsControl.Singleton.GetAllLibs()[request.Name])
        {
            string content = Database.Singleton.GetContenido(hash);
            Library library = string.IsNullOrEmpty(content)
                ? null
                : JsonConvert.DeserializeObject<Library>(content);
            if (library == null)
            {
                continue;
            }
            foreach (
                Library.Vulnerabilidad weakness in library
                    .GetMemoryZones()
                    .Values.SelectMany(zone => zone.vulnerabs)
            )
            {
                if (
                    weakness.GetHackResult().hackResult == reward
                    && weakness.requiredActions.Count < fewest
                )
                {
                    selection = content;
                    fewest = weakness.requiredActions.Count;
                }
            }
        }

        if (selection == null)
        {
            throw new InvalidOperationException(
                "No generated " + request.Name + " has that reward"
            );
        }

        Computer computer =
            request.LocalIp.Length == 0 ? player.Computer : player.GetLanComputer(request.LocalIp);
        List<int> staleScripts = computer
            .GetProcesos()
            .Where(process => process.isScript && !process.isProtected)
            .Select(process => process.PID)
            .ToList();
        computer.ClosePrograms(staleScripts);
        computer
            .GetFileSystem()
            .ForceAddFile(
                new FileSystem.Archivo(
                    request.Name,
                    selection,
                    "root",
                    FileSystem.Fichero.TypeFile.Library,
                    saveToDB: true,
                    isDefaultContent: true
                ),
                "/lib"
            );
        Database.Singleton.SyncFileSystemDB(computer, onlyFileSystem: true);
        return new Reply.Applied();
    }

    /// <summary>Changes all firewall rules at <c>request.LocalIp</c> to allow; the next setup restores them.</summary>
    public Reply AllowFirewall(AllowFirewallRpc request)
    {
        Router main = player.Computer.GetMainRouter();
        NetworkLan.Device device =
            main.GetNetworkLan().FindDevice(request.LocalIp)?.GetDevice()
            ?? throw new InvalidOperationException("No device at " + request.LocalIp);

        restore.RecordFirewall(
            request.LocalIp,
            device.firewallRules.Select(rule => rule.action).ToList()
        );

        foreach (HwFirewallRule rule in device.firewallRules)
        {
            rule.action = HwFirewallRule.Action.ALLOW;
        }

        Database.Singleton.SyncConfigOS(main);
        return new Reply.Applied();
    }

    /// <summary>
    /// Sets the flood counter for the computer at <c>request.LocalIp</c> to one connection below the reboot threshold
    /// that <c>NetSessionIntrinsics.flood_connection</c> passes to <c>FloodSystem</c>.
    /// </summary>
    public Reply PrimeFlood(PrimeFloodRpc request)
    {
        PlayerServer server = player.Server;
        Computer computer = player.GetLanComputer(request.LocalIp);
        Router router = computer.GetMainRouter();
        NpcInfo admin = router.GetNetworkLan().GetAdminNpcInfo();
        Database.Singleton.SyncConfigOS(router);
        if (StocksCompany.IsListedOnStockMarket(router.GetTipoRed()))
        {
            // `AppLauncher.ListenProcessMessages` publishes DDoS effects on company stock and guild state.
            StocksCompany.SyncMarketData(router);
        }
        int speed = (int)router.GetNetworkLan().netSpeed;
        bool superAdmin =
            router.GetTipoRed() == ServerMap.TipoRed.RentServer
                ? router.GetNetworkLan().IsSuperAdminRental()
                : admin != null && admin.IsSuperAdmin(checkDeadOrFired: true);
        int maximum = speed * 3;
        if (!superAdmin)
        {
            maximum = (maximum * 3 + 3) / 4;
        }

        FloodSystem.Singleton.ClearComputerFloods(computer.GetID());
        for (int origin = 1; origin < maximum; origin++)
        {
            FloodSystem.Singleton.AddFlood(
                server.playerID,
                computer.GetID(),
                "fixture-" + origin,
                maximum
            );
        }
        return new Reply.Applied();
    }

    private static bool HasDevice(NetworkLan.Device device, string kind)
    {
        return device.GetLanPcInfos().Any(computer => computer.typeDevice.ToString() == kind)
            || device.GetDevices().Any(child => HasDevice(child, kind));
    }
}
