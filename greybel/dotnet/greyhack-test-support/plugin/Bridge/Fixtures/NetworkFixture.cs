using System;
using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Plugin.Bridge.Setup;
using GreyHackTestSupport.Protocol;
using Newtonsoft.Json;
using ServiceConfig;
using Util;
using ServicioID = ServiceConfig.ServicioID;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>
/// Creates and locates networks and configures their computers, port forwards, libraries, firewalls, and flood
/// counters. It can also disable networks.
/// </summary>
internal sealed class NetworkFixture(Player player, WorldRestore restore, FixtureStore store)
{
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

    /// <summary>Creates a valid rental layout and marks it CTF before the player's computer joins it.</summary>
    public Reply SpawnCtf(SpawnCtfNetworkRpc request)
    {
        Router router = SpawnFresh(ServerMap.TipoRed.RentServer);
        router.SetTipoRed(ServerMap.TipoRed.CTF);
        store.AddNetwork(router.GetPublicIP());

        return new Reply.NetworkCreated(
            new NetworkCreatedReply(router.GetPublicIP(), router.GetLocalIP())
        );
    }

    /// <summary>Creates one fresh network and materializes each named target through the game's LAN generator.</summary>
    public Reply CreateFixture(CreateFixtureNetworkRpc request)
    {
        ServerMap.TipoRed kind = (ServerMap.TipoRed)
            Enum.Parse(typeof(ServerMap.TipoRed), request.Kind);
        Router router = SpawnFresh(kind);
        string publicIp = router.GetPublicIP();
        store.AddNetwork(publicIp);
        NetworkLan network = router.GetNetworkLan();
        if (request.Targets.Any(target => target.Device == "SMARTPHONE"))
        {
            DateTime now = ClockServer.Singleton.GetDateTime();
            DateTime work = now.AddDays((8 - (int)now.DayOfWeek) % 7).Date.AddHours(10);
            GameField.ClockCurrentTime.SetValue(ClockServer.Singleton, work);
        }
        NetworkLan.GlobalLan globalLan = (NetworkLan.GlobalLan)
            GameField.NetworkGlobalLan.GetValue(network);
        NetworkLan.RouterNet child = AddLayout(router, network, globalLan, request.Layout);

        if (request.OldestKernel)
        {
            string content = ReadOldestLibrary("kernel_router.so");
            router
                .GetFileSystem()
                .ForceAddFile(
                    new FileSystem.Archivo(
                        "kernel_router.so",
                        content,
                        "root",
                        FileSystem.Fichero.TypeFile.Library,
                        saveToDB: true,
                        isDefaultContent: true
                    ),
                    "/lib"
                );
        }

        List<(FixtureTarget Request, LanPcInfo Info)> infos =
            new List<(FixtureTarget, LanPcInfo)>();
        foreach (FixtureTarget target in request.Targets)
        {
            NetworkLan.RouterNet parent = target.Location switch
            {
                "main" => network.routerDevice,
                "lan_router" => child
                    ?? throw new InvalidOperationException("A LAN target needs the router layout"),
                _ => throw new InvalidOperationException("Unknown fixture target location"),
            };
            if (
                target.Location != "main"
                && target.Services.Any(service => service.PublicPort != null)
            )
            {
                throw new InvalidOperationException("Only main targets can have public forwards");
            }

            IP localIp = NextAddress(network, parent.localIp);
            Puertos ports = new Puertos();
            Dictionary<ServicioID, LanPcInfo.LibraryInfo> libVersions =
                new Dictionary<ServicioID, LanPcInfo.LibraryInfo>();
            List<Servicio> services = new List<Servicio>();
            foreach (FixtureService fixtureService in target.Services)
            {
                ServicioID serviceId = (ServicioID)
                    Enum.Parse(typeof(ServicioID), fixtureService.Service, true);
                if (fixtureService.PublicPort is ushort publicPort)
                {
                    // A generated network may already forward this external port. Replace that forward as router
                    // configuration would, retaining its generated service without public forwarding.
                    router
                        .GetConfigOS()
                        .puertos.allPorts.RemoveAll(port => port.externalPort == publicPort);
                }
                Servicio service = new Servicio(
                    XmlGlobal.Singleton.tipoServidor.GetServicio(serviceId)
                )
                {
                    puerto = fixtureService.InternalPort,
                };
                parent.PreAddServicio(
                    service,
                    services,
                    libVersions,
                    ports,
                    router,
                    localIp.ToString(),
                    fixtureService.PublicPort != null,
                    isolated: false
                );
                Puerto internalPort = ports.allPorts.Last();
                if (internalPort.internalPort != fixtureService.InternalPort)
                {
                    throw new InvalidOperationException(
                        "The game assigned a different internal port"
                    );
                }
                if (fixtureService.PublicPort is ushort fixedPort)
                {
                    internalPort.externalPort = fixedPort;
                    Puerto forward = router
                        .GetConfigOS()
                        .puertos.allPorts.Last(candidate =>
                            candidate.lanIP == localIp.ToString()
                            && candidate.internalPort == fixtureService.InternalPort
                        );
                    forward.externalPort = fixedPort;
                }
            }

            NetworkLan.TypeDevice device = (NetworkLan.TypeDevice)
                Enum.Parse(typeof(NetworkLan.TypeDevice), target.Device);
            LanPcInfo info = new LanPcInfo(
                localIp.ToString(),
                ports,
                libVersions,
                services,
                router,
                globalLan.random,
                device
            )
            {
                gateway = parent.localIp,
            };
            if (device == NetworkLan.TypeDevice.SMARTPHONE)
            {
                DateTime now = ClockServer.Singleton.GetDateTime();
                string owner = network
                    .GetNpcInfos()
                    .Values.FirstOrDefault(npc =>
                    {
                        Activity activity = npc.schedule.GetCurrentActivity(now.DayOfWeek, now.TimeOfDay);
                        return activity == Activity.Trabajando || activity == Activity.DescansoBreve;
                    })
                    ?.GetSurname()
                    ?? throw new InvalidOperationException("No working NPC can own the smartphone");
                GameField.LanNpcOwner.SetValue(info, owner);
            }
            foreach (
                FixtureService fixtureService in target.Services.Where(service =>
                    service.OldestLibrary
                )
            )
            {
                Servicio service = XmlGlobal.Singleton.tipoServidor.GetServicio(
                    (ServicioID)Enum.Parse(typeof(ServicioID), fixtureService.Service, true)
                );
                string name = service.GetLibName();
                info.UpdateLibVersion(
                    JsonConvert.DeserializeObject<Library>(ReadOldestLibrary(name)),
                    ReadOldestLibrary(name)
                );
            }
            parent.AddPcInfo(info);
            infos.Add((target, info));
        }
        network.UpdateParentDevicesAndPlayers();
        GameMethod.NetworkAllowMainServicesFw.Invoke(network, null);
        GameMethod.NetworkUpdateNpcsLocalIps.Invoke(network, null);
        router.GetRouterConfigOS().BackupPorts();

        List<FixtureTargetReply> replies = new List<FixtureTargetReply>();
        foreach ((FixtureTarget target, LanPcInfo info) in infos)
        {
            if (info.IsHidden(network))
            {
                throw new InvalidOperationException("A fixture target was generated hidden");
            }
            Computer computer =
                router.GetComputer(info.localIp)
                ?? throw new InvalidOperationException(
                    "The game did not materialize a fixture target"
                );
            ApplyLibraryState(computer, target.LibraryState);
            if (
                kind == ServerMap.TipoRed.Comisaria
                && target.Name == "workstation"
                && computer.GetFileSystem().GetArchivo("/lib/libtrafficnet.so") == null
            )
            {
                throw new InvalidOperationException(
                    "The police workstation has no traffic library"
                );
            }
            if (
                kind == ServerMap.TipoRed.HackShop
                && target.Services.Any(service =>
                    service.Service.Equals("repository", StringComparison.OrdinalIgnoreCase)
                )
                && computer.GetFileSystem().GetArchivo("/server/conf/repod.conf") == null
            )
            {
                throw new InvalidOperationException("The HackShop repository has no catalog");
            }
            Computer.User root = computer.GetUser("root");
            if (target.RootPassword.Length > 0)
            {
                root.SetPassword(target.RootPassword);
                computer.GeneratePasswdFile();
            }
            Computer.User user = computer.GetUsers(includeRoot: false).FirstOrDefault();
            List<FixtureServiceReply> services = target
                .Services.Select(service => new FixtureServiceReply(
                    service.Service,
                    service.InternalPort,
                    service.PublicPort
                ))
                .ToList();
            replies.Add(
                new FixtureTargetReply(
                    target.Name,
                    info.localIp,
                    root.GetPassPlano(),
                    user?.nombreUsuario ?? "",
                    user?.GetPassPlano() ?? "",
                    services
                )
            );
            Database.Singleton.SyncFileSystemDB(computer);
        }

        if (kind == ServerMap.TipoRed.Comisaria)
        {
            foreach (NpcInfo npc in network.GetNpcInfos().Values)
            {
                if (npc.GetLicensePlate().Length == 0)
                {
                    GameField.NpcLicensePlate.SetValue(
                        npc,
                        LicensePlateGenerator.GenerateLicensePlate(publicIp, npc.GetSurname())
                    );
                }
            }
        }

        if (child != null)
        {
            Router childRouter = router.GetLanRouter(child.localIp);
            Database.Singleton.SyncFileSystemDB(childRouter);
            Database.Singleton.SyncConfigOS(childRouter);
        }
        Database.Singleton.SyncFileSystemDB(router);
        Database.Singleton.SyncConfigOS(router);
        return new Reply.FixtureNetwork(
            new FixtureNetworkReply(
                publicIp,
                router.GetLocalIP(),
                router.GetTipoRed().ToString(),
                replies
            )
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

    /// <summary>Installs requirement-free generated weaknesses for the legacy reward-only request.</summary>
    public Reply InstallLibrary(InstallLibraryRpc request)
    {
        InstallFixture(
            new InstallFixtureLibraryRpc(
                request.Request,
                request.LocalIp,
                request.Name,
                request
                    .Rewards.Select(reward => new FixtureWeakness(
                        reward,
                        Array.Empty<byte>(),
                        "",
                        ""
                    ))
                    .ToList(),
                false
            )
        );
        return new Reply.Applied();
    }

    /// <summary>Creates a new version through Library.NewVersion and adds exactly the requested generated weaknesses.</summary>
    public Reply InstallFixture(InstallFixtureLibraryRpc request)
    {
        VersionsControl versions = VersionsControl.Singleton;
        Library latest = versions.GetLastLibs()[request.Name];
        Library library = JsonConvert.DeserializeObject<Library>(
            JsonConvert.SerializeObject(latest)
        );
        library.NewVersion();
        Random random = new Random(Guid.NewGuid().GetHashCode());
        List<FixtureWeaknessReply> replies = new List<FixtureWeaknessReply>();

        foreach (FixtureWeakness weaknessSpec in request.Weaknesses)
        {
            VersionsControl.IdLib idLib = library.GetIdLib();
            Library.Vulnerabilidad.HackResult reward = (Library.Vulnerabilidad.HackResult)
                weaknessSpec.Reward;
            List<Library.Vulnerabilidad.Required> requirements = weaknessSpec
                .Requirements.Select(requirement => (Library.Vulnerabilidad.Required)requirement)
                .ToList();
            ValidateWeakness(idLib, reward, requirements, weaknessSpec.User);
            List<Library.Vulnerabilidad.Required> excludedRequirements = Enum.GetValues(
                    typeof(Library.Vulnerabilidad.Required)
                )
                .Cast<Library.Vulnerabilidad.Required>()
                .Except(requirements)
                .ToList();
            string address;
            do
            {
                address = "0x" + random.Next(int.MaxValue).ToString("X");
            } while (library.GetMemoryZones().ContainsKey(address));
            // With reqNotAllowed, Vulnerabilidad.Configure adds every allowed requirement, except that a router kernel
            // draws its reward first and drops all requirements when that draw is SHELL, RANDOM_FOLDER or
            // FIREWALL_DISABLE (the reward is replaced below). Seed 1 draws a reward that retains them; the check below
            // fails loudly if a game update changes that draw.
            Random weaknessRandom =
                idLib == VersionsControl.IdLib.KERNEL_ROUTER && requirements.Count > 0
                    ? new Random(1)
                    : random;
            Library.MemoryZone zone = new Library.MemoryZone(
                weaknessRandom,
                address,
                idLib,
                excludedRequirements,
                out Library.Vulnerabilidad weakness
            );
            if (
                weakness.requiredActions.Count != requirements.Count
                || weakness.requiredActions.Any(requirement => !requirements.Contains(requirement))
            )
            {
                throw new InvalidOperationException(
                    "The game generated different weakness requirements"
                );
            }
            weakness.helperHackResult.hackResult = reward;
            if (weaknessSpec.User.Length > 0)
            {
                weakness.helperHackResult.user = weaknessSpec.User;
            }
            if (weaknessSpec.PathExist.Length > 0)
            {
                weakness.helperHackResult.pathExist = weaknessSpec.PathExist;
            }
            library.GetMemoryZones().Add(address, zone);
            replies.Add(
                new FixtureWeaknessReply(
                    address,
                    weakness.unsecValue,
                    weaknessSpec.Reward,
                    weakness.requiredActions.Select(requirement => (byte)requirement).ToList(),
                    weakness.helperHackResult.user
                )
            );
        }

        Computer computer =
            request.LocalIp.Length == 0
                ? player.Server.GetComputer()
                : player.GetLanComputer(request.LocalIp);
        List<int> staleScripts = computer
            .GetProcesos()
            .Where(process => process.isScript && !process.isProtected)
            .Select(process => process.PID)
            .ToList();
        computer.ClosePrograms(staleScripts);
        string content = JsonConvert.SerializeObject(library);
        computer
            .GetFileSystem()
            .ForceAddFile(
                new FileSystem.Archivo(
                    request.Name,
                    content,
                    "root",
                    FileSystem.Fichero.TypeFile.Library,
                    saveToDB: true,
                    isDefaultContent: true
                ),
                "/lib"
            );
        Database.Singleton.SyncFileSystemDB(computer, onlyFileSystem: true);

        if (request.RegisterExploits)
        {
            GameMethod.VersionsGeneraExploit.Invoke(
                versions,
                new object[] { new KeyValuePair<string, Library>(request.Name, library), false }
            );
            Dictionary<string, Dictionary<string, List<Exploit>>> exploits =
                (Dictionary<string, Dictionary<string, List<Exploit>>>)
                    GameField.VersionsExploits.GetValue(versions);
            Database.Singleton.UpdateExploits(
                versions.GetLastLibs(),
                exploits,
                versions.GetAllLibs()
            );
            store.AddExploit(request.Name, library.GetStrVersion());
        }

        return new Reply.FixtureLibrary(
            new FixtureLibraryReply(request.Name, library.GetStrVersion(), content, replies)
        );
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

    private static void ValidateWeakness(
        VersionsControl.IdLib idLib,
        Library.Vulnerabilidad.HackResult reward,
        List<Library.Vulnerabilidad.Required> requirements,
        string user
    )
    {
        bool router = idLib == VersionsControl.IdLib.KERNEL_ROUTER;
        bool validReward =
            reward
                is Library.Vulnerabilidad.HackResult.SHELL
                    or Library.Vulnerabilidad.HackResult.RANDOM_FOLDER
                    or Library.Vulnerabilidad.HackResult.COMPUTER
            || (router && reward == Library.Vulnerabilidad.HackResult.FIREWALL_DISABLE)
            || (!router && reward == Library.Vulnerabilidad.HackResult.CHANGE_PASS)
            || (
                idLib == VersionsControl.IdLib.LIBSMARTAPPLIANCE
                && reward == Library.Vulnerabilidad.HackResult.OVERRIDE_SETTINGS
            )
            || (
                idLib == VersionsControl.IdLib.LIBTRAFFICNET
                && reward == Library.Vulnerabilidad.HackResult.TRAFFIC_LIGHT_CONTROL
            );
        if (!validReward)
        {
            throw new InvalidOperationException("The requested reward is invalid for this library");
        }
        if (user.Length > 0 && user != "guest" && user != "root" && user != "normal_user")
        {
            throw new InvalidOperationException("Unknown weakness user");
        }
        if (
            requirements.Contains(Library.Vulnerabilidad.Required.ANY_ACTIVE_USER)
            && requirements.Contains(Library.Vulnerabilidad.Required.ROOT_ACTIVE_USER)
        )
        {
            throw new InvalidOperationException("Active-user requirements are mutually exclusive");
        }
        List<Library.Vulnerabilidad.Required> validRequirements = router
            ? new List<Library.Vulnerabilidad.Required>
            {
                Library.Vulnerabilidad.Required.LOCAL_NETWORK,
                Library.Vulnerabilidad.Required.CONN_GATEWAY,
            }
            : Enum.GetValues(typeof(Library.Vulnerabilidad.Required))
                .Cast<Library.Vulnerabilidad.Required>()
                .Where(requirement =>
                    requirement != Library.Vulnerabilidad.Required.CONN_GATEWAY
                    && requirement != Library.Vulnerabilidad.Required.LOCAL_NETWORK
                    && (
                        idLib <= VersionsControl.IdLib.APTCLIENT
                        || requirement != Library.Vulnerabilidad.Required.PORT_FORWARD
                    )
                )
                .ToList();
        if (requirements.Any(requirement => !validRequirements.Contains(requirement)))
        {
            throw new InvalidOperationException("A requirement is invalid for this library");
        }
    }

    private static IP NextAddress(NetworkLan network, string gateway)
    {
        IP address = (IP)GameMethod.NetworkGetNextIp.Invoke(network, new object[] { gateway });
        while (
            HasAddress(network.routerDevice, address.ToString())
            || network.FindDevice(address.ToString()) != null
        )
        {
            address.NextIP();
        }
        return address;
    }

    private static bool HasAddress(NetworkLan.Device device, string address) =>
        device.localIp == address
        || device.GetLanPcInfos().Any(computer => computer.localIp == address)
        || device.GetDevices().Any(child => HasAddress(child, address));

    private static void ApplyLibraryState(Computer computer, string state)
    {
        if (state.Length == 0)
        {
            return;
        }
        FileSystem fs = computer.GetFileSystem();
        FileSystem.Carpeta lib = fs.GetCarpeta("/lib");
        switch (state)
        {
            case "missing":
                lib.RemoveFichero(lib.GetFichero("libssh.so"), true);
                break;
            case "invalid":
                lib.AddFile(new FileSystem.Archivo("libssh.so", "not a library", "root", false, true));
                break;
            case "unexpected":
                fs.GetArchivo("/lib/libssh.so").SetContenido("{\"idLib\":1}", true);
                break;
            case "malformed":
                FileSystem.Archivo badLibrary = fs.GetArchivo("/lib/libssh.so");
                badLibrary.SetContentId(Computer.Md5Sum("not json"), "");
                Database.Singleton.SetContenido(badLibrary, "not json", "");
                break;
            default:
                throw new InvalidOperationException("Unknown library state: " + state);
        }
    }

    private static Router SpawnFresh(ServerMap.TipoRed kind)
    {
        for (int attempt = 0; attempt < 64; attempt++)
        {
            Router router = ServerMap.Singleton.SpawnRouter(
                kind,
                NetworkLan.AccessType.NORMAL,
                null,
                allowMission: false,
                "",
                ServerMap.GenerationProfile.Default,
                preserveTipoRed: true
            );
            string publicIp = router.GetPublicIP();
            if (!IP.IsLanIp(publicIp) && Database.Singleton.GetRouterNode(publicIp) != null)
            {
                return router;
            }
            if (IP.IsLanIp(publicIp))
            {
                Database.Singleton.DeleteNetworks(new List<string> { publicIp });
            }
        }
        throw new InvalidOperationException("The game could not allocate a fresh public address");
    }

    private static NetworkLan.RouterNet AddLayout(
        Router router,
        NetworkLan network,
        NetworkLan.GlobalLan globalLan,
        string layout
    )
    {
        if (layout == "flat")
        {
            return null;
        }
        if (layout != "router" && layout != "router_switch")
        {
            throw new InvalidOperationException("Unknown fixture network layout");
        }

        // RouterNet.ConfigPendingRouter already allocates the child's game-style LAN address.
        NetworkLan.RouterNet child = new NetworkLan.RouterNet(globalLan);
        child.wifiEnabled = true;
        Random wifiRandom = new Random(
            WorldSeed.DeriveInt("wifi-public-v1", router.GetPublicIP(), child.localIp)
        );
        child.bssid = Networking.GeneraMacAddress(wifiRandom);
        child.essid = ServerMap.GeneraWifiName(wifiRandom);
        network.routerDevice.routers.Add(child);

        if (layout == "router_switch")
        {
            NetworkLan.HubNet hub = new NetworkLan.HubNet(
                NetworkLan.Device.PlayerModif.NONE,
                NetworkLan.TypeDevice.SWITCH
            );
            hub.ID = router.GetPublicIP() + ":" + globalLan.random.Next(int.MaxValue);
            hub.localIp = NextAddress(network, network.routerDevice.localIp).ToString();
            network.routerDevice.hubs.Add(hub);
        }
        return child;
    }

    private static string ReadOldestLibrary(string name)
    {
        List<string> versions = VersionsControl.Singleton.GetAllLibs()[name];
        if (versions.Count < 2)
        {
            throw new InvalidOperationException("The world has no older version of " + name);
        }
        return Database.Singleton.GetContenido(versions[0]);
    }
}
