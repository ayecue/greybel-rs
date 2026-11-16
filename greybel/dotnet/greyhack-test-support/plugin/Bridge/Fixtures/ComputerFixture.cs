using System;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using HarmonyLib;
using Newtonsoft.Json;
using ServiceConfig;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Creates files and deterministic boundary states on the player's computer or a LAN computer.</summary>
internal sealed class ComputerFixture(Player player)
{
    /// <summary>Creates a text file owned by the first user of the player's computer.</summary>
    public Reply WriteFile(WriteFileRpc request)
    {
        string parent = request.Path.Substring(0, request.Path.LastIndexOf('/'));
        if (parent.Length == 0)
        {
            parent = "/";
        }
        string name = request.Path.Substring(request.Path.LastIndexOf('/') + 1);
        PlayerComputer computer = player.Computer;
        string owner = computer.GetUsers().FirstOrDefault()?.nombreUsuario ?? "root";
        computer
            .GetFileSystem()
            .ForceAddFile(
                new FileSystem.Archivo(name, request.Content, owner, false, true),
                parent
            );
        Database.Singleton.SyncFileSystemDB(computer, onlyFileSystem: true);
        return new Reply.FileWritten(new FileWrittenReply(request.Path));
    }

    /// <summary>Builds a requested state with the game's own account, filesystem, and hardware methods.</summary>
    public Reply Configure(ConfigureComputerRpc request)
    {
        Computer computer =
            request.LocalIp.Length == 0
                ? player.Computer
                : player.GetLanComputer(request.LocalIp);
        switch (request.Fixture)
        {
            case "permissions":
                PermissionTree(computer);
                break;
            case "users":
                while (computer.GetNumUsers() < 16)
                {
                    computer.CreateUser("fixture" + computer.GetNumUsers(), "pass1");
                }
                break;
            case "full_folder":
                FillFolder(computer, 250);
                break;
            case "full_filesystem":
                FillFilesystem(computer);
                break;
            case "full_disk":
                computer.GetHardware().hardDisk.totalSize = 1;
                Database.Singleton.SyncHardwareDB(computer.GetID(), computer.GetHardware());
                break;
            case "disabled_wifi":
                computer.GetConfigOS().SetActiveNetCard(ConfigOS.ActiveNetCard.NONE);
                break;
            case "ethernet_home":
                EnsureEthernet(computer);
                ((PlayerComputer)computer).GetPlayerConfigOS().rentalHomeIP = computer.GetPublicIP();
                break;
            case "missing_isp":
                EnsureEthernet(computer);
                ((PlayerComputer)computer).GetPlayerConfigOS().rentalHomeIP = "203.0.113.1";
                break;
            case "deep_tree":
                string owner = computer.GetUsers().First().nombreUsuario;
                computer.GetFileSystem().GetLastFolder("/" + string.Join("/", Enumerable.Repeat("d", 16)), owner);
                break;
            case "low_ram":
                Hardware lowRam = computer.GetHardware();
                lowRam.rams.Clear();
                lowRam.rams.Add(new Hardware.RAM("fixture", 1, 1, new Random(1)));
                Database.Singleton.SyncHardwareDB(computer.GetID(), lowRam);
                break;
            case "terminal_limit":
                while (computer.GetProcesos().Count(process => process.isTerminal) < 15)
                {
                    computer.AddProcess("Terminal", "root", Computer.GetRamUsage("Terminal"), false, true);
                }
                break;
            case "rebooting":
                computer.Reboot(false);
                break;
            case "closed_public_service":
                MainForward(computer).isClosed = true;
                break;
            case "missing_machine":
                MainForward(computer).lanIP = "192.0.2.1";
                break;
            case "missing_ssh_service":
                while (computer.RemoveServicio(ServicioID.ssh)) { }
                computer.GetConfigOS().puertos.allPorts.RemoveAll(port => port.ID == ServicioID.ssh);
                if (computer.ExisteServicio(ServicioID.ssh))
                {
                    throw new InvalidOperationException("The SSH service fixture still has a registered service");
                }
                break;
            case "unreachable":
                computer.GetMainRouter().GetNetworkLan().routerDevice.localIp = "0.0.0.0";
                break;
            case "firewall_deny":
                foreach (HwFirewallRule rule in computer.GetMainRouter().GetNetworkLan().routerDevice.firewallRules)
                {
                    rule.action = HwFirewallRule.Action.DENY;
                }
                break;
            case "chat_program":
                AddNativeProgram(computer, "Chat.exe");
                break;
            case "gui_program":
                AddNativeProgram(computer, "Notepad.exe");
                break;
            case "root_only":
                foreach (Computer.User user in computer.GetUsers(false).ToList())
                {
                    computer.DeleteUser(user.nombreUsuario, true);
                }
                break;
            case "startup_error":
                FileSystem.Carpeta boot = computer.GetFileSystem().GetCarpeta("/boot");
                boot.RemoveFichero(boot.GetFichero("System.map"), true);
                break;
            case "missing_kernel":
                FileSystem.Carpeta kernelFolder = computer.GetFileSystem().GetCarpeta("/lib");
                kernelFolder.RemoveFichero(kernelFolder.GetFichero("kernel_router.so"), true);
                break;
            case "invalid_kernel":
                FileSystem.Carpeta invalidKernelFolder = computer.GetFileSystem().GetCarpeta("/lib");
                invalidKernelFolder.AddFile(new FileSystem.Archivo("kernel_router.so", "not a kernel", "root", false, true));
                break;
            case "symlink_kernel":
                computer.GetFileSystem().GetArchivo("/lib/kernel_router.so").SetSymlink("/lib/kernel_router.so");
                break;
            case "closed_ports":
                foreach (Router router in player.GetRouterInstances(computer.GetMainRouter().GetLocalIP()))
                {
                    foreach (Puerto port in router.GetConfigOS().puertos.allPorts)
                    {
                        port.isClosed = true;
                    }
                    Database.Singleton.SyncConfigOS(router);
                }
                break;
            case "home_readonly":
                computer.GetFileSystem().GetCarpeta("/home/test").SetPermisos("dr-xr-xr-x");
                break;
            case "crypto_files":
                FileSystem.Carpeta cryptoHome = computer.GetFileSystem().GetLastFolder("/home/test", "test");
                cryptoHome.AddFile(new FileSystem.Archivo("plain.txt", "plain", "test", false, true));
                cryptoHome.AddFile(new FileSystem.Archivo("malformed.cap", "not json", "root", true, true));
                FileSystem.Carpeta noAccess = computer.GetFileSystem().GetLastFolder("/home/test/noaccess", "root");
                noAccess.SetPermisos("d---------");
                FileSystem.Archivo lockedCapture = new FileSystem.Archivo("locked.cap", "not json", "root", true, true);
                lockedCapture.SetPermisos("----------");
                noAccess.AddFile(lockedCapture);
                FileSystem.Archivo encryptedCapture = new FileSystem.Archivo("encrypted.cap", "not json", "root", true, true);
                encryptedCapture.Encrypt("hunter2");
                cryptoHome.AddFile(encryptedCapture);
                string missingCapture = JsonConvert.SerializeObject(new
                {
                    power = 1,
                    numAcks = 300000,
                    bssid = "00:00:00:00:00:00",
                    essid = "missing",
                    origPublicIp = computer.GetPublicIP()
                });
                cryptoHome.AddFile(new FileSystem.Archivo("missing_router.cap", missingCapture, "root", true, true));
                break;
            case "distant_router":
                ServerMap.RouterNode distantNode = Database.Singleton.GetRouterNode(computer.GetPublicIP());
                distantNode.routerPos = new CustomVector.Vector2(
                    distantNode.routerPos.x + 1f,
                    distantNode.routerPos.y
                );
                Database.Singleton.SyncServerMap(distantNode);
                break;
            case "unsupported_monitor":
                foreach (Hardware.NetworkDevice card in computer.GetHardware().networkDevices.Where(card => card.isWifi))
                {
                    card.monitorSupport = false;
                }
                Database.Singleton.SyncHardwareDB(computer.GetID(), computer.GetHardware());
                break;
            case "invalid_smtp_target":
                MainForward(computer).internalPort++;
                break;
            case "missing_smtp_service":
                while (computer.RemoveServicio(ServicioID.smtp)) { }
                computer.GetConfigOS().puertos.allPorts.RemoveAll(port => port.ID == ServicioID.smtp);
                break;
            case "malformed_metaxploit":
                FileSystem.Archivo badLibrary = computer.GetFileSystem().GetArchivo("/lib/metaxploit.so");
                badLibrary.SetContentId(Computer.Md5Sum("not json"), "");
                Database.Singleton.SetContenido(badLibrary, "not json", "");
                break;
            case "ssh_service":
                computer.AddServicio(
                    XmlGlobal.Singleton.tipoServidor.GetServicio(ServicioID.ssh),
                    11022
                );
                foreach (Router router in player.GetRouterInstances(computer.GetMainRouter().GetLocalIP()))
                {
                    router.GetConfigOS().puertos.AddPuerto(
                        new Puerto(ServicioID.ssh, 11022, 11022, false, true, computer.GetLocalIP()),
                        computer.GetLocalIP()
                    );
                    Database.Singleton.SyncConfigOS(router);
                }
                break;
            case "apt_cache_missing":
                RemoveEntry(computer, "/etc/apt/aptcache.bin");
                break;
            case "apt_cache_binary":
                computer.GetFileSystem().GetArchivo("/etc/apt/aptcache.bin").SetBinario(true);
                break;
            case "apt_cache_malformed":
                computer.GetFileSystem().GetArchivo("/etc/apt/aptcache.bin").SetContenido("not json", true);
                break;
            case "apt_sources_missing":
                RemoveEntry(computer, "/etc/apt/sources.txt");
                break;
            case "apt_sources_binary":
                computer.GetFileSystem().GetArchivo("/etc/apt/sources.txt").SetBinario(true);
                break;
            case "apt_sources_malformed":
                computer.GetFileSystem().GetArchivo("/etc/apt/sources.txt").SetContenido("not json", true);
                break;
            case "apt_sources_unknown":
                computer.GetFileSystem().ForceAddFile(
                    new FileSystem.Archivo(
                        "sources.txt",
                        "{\"official_server\":false,\"sourceList\":{\"ghost.org\":0}}",
                        "root",
                        false,
                        true
                    ),
                    "/etc/apt"
                );
                break;
            case "repository_missing_db":
                RemoveEntry(computer, "/server/conf/repod.conf");
                break;
            case "repository_binary_db":
                computer.GetFileSystem().GetArchivo("/server/conf/repod.conf").SetBinario(true);
                break;
            case "repository_malformed_db":
                computer.GetFileSystem().GetArchivo("/server/conf/repod.conf").SetContenido("not json", true);
                break;
            case "repository_missing_folder":
                RemoveEntry(computer, "/Public/htdocs/downloads");
                break;
            case "repository_missing_package":
                foreach (string package in new[] { "metaxploit.so", "scanlib" })
                {
                    if (computer.GetFileSystem().GetFichero("/Public/htdocs/downloads/" + package) != null)
                    {
                        RemoveEntry(computer, "/Public/htdocs/downloads/" + package);
                    }
                }
                break;
            case "repository_service_down":
                while (computer.RemoveServicio(ServicioID.repository)) { }
                computer.GetConfigOS().puertos.allPorts.RemoveAll(port => port.ID == ServicioID.repository);
                break;
            case "missing_traffic_workstation":
                NetworkLan trafficNetwork = computer.GetMainRouter().GetNetworkLan();
                NpcInfo trafficNpc =
                    trafficNetwork.GetNpcInfoTrafficCrecentials()
                    ?? NpcFixture.EnsureNpc(trafficNetwork, computer.GetMainRouter());
                Traverse.Create(trafficNetwork).Field("trafficCredentials").SetValue(trafficNpc.GetSurname());
                LanPcInfo workstation = trafficNetwork.GetLanPcInfo(trafficNpc.GetLocalIP())
                    ?? throw new InvalidOperationException("The traffic fixture has no NPC workstation");
                workstation.SetHidden(true);
                break;
            case "router_ssh":
                Router targetRouter = player.Computer.GetMainRouter();
                targetRouter.AddServicio(
                    XmlGlobal.Singleton.tipoServidor.GetServicio(ServicioID.ssh),
                    22
                );
                foreach (Router router in player.GetRouterInstances(targetRouter.GetLocalIP()))
                {
                    router.GetConfigOS().puertos.AddPuerto(
                        new Puerto(ServicioID.ssh, 22, 22, false, true, targetRouter.GetLocalIP()),
                        targetRouter.GetLocalIP()
                    );
                    Database.Singleton.SyncConfigOS(router);
                }
                break;
            default:
                throw new InvalidOperationException("Unknown computer fixture: " + request.Fixture);
        }
        Database.Singleton.SyncFileSystemDB(computer);
        Database.Singleton.SyncConfigOS(computer);
        return new Reply.Applied();
    }

    /// <summary>Installs a wired card through the game's hardware inventory when the generated computer lacks one.</summary>
    private static void EnsureEthernet(Computer computer)
    {
        Hardware hardware = computer.GetHardware();
        if (!hardware.networkDevices.Any(device => !device.isWifi))
        {
            hardware.AddItemHardware(
                new Hardware.NetworkDevice(new Random(1), false),
                Hardware.TypeHardware.PCI
            );
            Database.Singleton.SyncHardwareDB(computer.GetID(), hardware);
        }
    }

    /// <summary>Creates an unprivileged account and readable, denied, protected, and mixed-permission trees.</summary>
    private static void PermissionTree(Computer computer)
    {
        const string user = "limited";
        const string root = "/home/limited/matrix";
        if (computer.GetUser(user) == null)
        {
            computer.CreateUser(user, "pass1");
        }
        FileSystem fs = computer.GetFileSystem();
        fs.GetLastFolder(root, user);
        fs.GetLastFolder(root + "/open", user);
        fs.GetLastFolder(root + "/mixed", user);
        fs.GetLastFolder(root + "/destination", user);
        AddFolder(fs, root + "/locked", "root", "drwx------");
        AddFolder(fs, root + "/destination_locked", "root", "drwxr-xr-x");
        AddFolder(fs, root + "/mixed/locked", "root", "drwx------");
        AddFile(fs, root + "/open", "readable.txt", "content", user, "-rw-r--r--");
        AddFile(fs, root + "/open", "writable.txt", "content", user, "-rw-r--r--");
        AddFile(fs, root, "locked.txt", "secret", "root", "-r--------");
        AddFile(fs, root + "/mixed", "open.txt", "open", user, "-rw-r--r--");
        AddFile(fs, root + "/mixed", "locked.txt", "secret", "root", "-r--------");
        AddFile(fs, root + "/mixed/locked", "child.txt", "secret", "root", "-r--------");
        AddFile(fs, root, "protected.txt", "protected", "root", "-rw-r--r--", true);
        AddFolder(fs, root + "/replace", "root", "drwxr-xr-x");
        AddFolder(fs, root + "/replace/mixed", "root", "drwx------");
        AddFile(fs, root + "/replace/mixed", "child.txt", "secret", "root", "-r--------");
    }

    /// <summary>Fills a folder past the create-file limit; copy's lower boundary is covered too.</summary>
    private static void FillFolder(Computer computer, int count)
    {
        string owner = computer.GetUsers().First().nombreUsuario;
        string path = "/home/" + owner + "/full_folder";
        FileSystem fs = computer.GetFileSystem();
        fs.GetLastFolder(path, owner);
        FileSystem.Carpeta folder = fs.GetCarpeta(path);
        for (int index = folder.GetNumFicheros(); index <= count; index++)
        {
            AddFile(fs, path, "f" + index + ".txt", "", owner, "-rw-r--r--");
        }
    }

    /// <summary>Fills the filesystem beyond 3125 entries without generating a large string or disk payload.</summary>
    private static void FillFilesystem(Computer computer)
    {
        string owner = computer.GetUsers().First().nombreUsuario;
        string path = "/home/" + owner + "/full_filesystem";
        FileSystem fs = computer.GetFileSystem();
        fs.GetLastFolder(path, owner);
        int index = 0;
        while (fs.GetCarpetaRaiz().GetTotalFicheros() <= 3125)
        {
            AddFile(fs, path, "f" + index + ".txt", "", owner, "-rw-r--r--");
            index++;
        }
    }

    private static Puerto MainForward(Computer computer)
    {
        return computer
            .GetMainRouter()
            .GetConfigOS()
            .puertos.allPorts.First(port => port.lanIP == computer.GetLocalIP());
    }

    private static void RemoveEntry(Computer computer, string path)
    {
        FileSystem.Fichero entry = computer.GetFileSystem().GetFichero(path);
        if (entry == null)
        {
            throw new InvalidOperationException("The fixture path does not exist: " + path);
        }
        entry.GetParent().RemoveFichero(entry, true);
    }

    private static void AddNativeProgram(Computer computer, string name)
    {
        FileSystem fs = computer.GetFileSystem();
        if (fs.GetArchivo("/usr/bin/" + name) == null)
        {
            fs.ForceAddFile(new FileSystem.Archivo(name, "", "root", false, true), "/usr/bin");
        }
    }

    private static void AddFolder(FileSystem fs, string path, string owner, string permissions)
    {
        string parent = path.Substring(0, path.LastIndexOf('/'));
        string name = path.Substring(path.LastIndexOf('/') + 1);
        if (fs.GetCarpeta(path) == null)
        {
            fs.ForceAddFolder(new FileSystem.Carpeta(name, owner), parent);
        }
        fs.GetCarpeta(path).SetPermisos(permissions);
    }

    private static void AddFile(
        FileSystem fs,
        string parent,
        string name,
        string content,
        string owner,
        string permissions,
        bool isProtected = false
    )
    {
        FileSystem.Archivo file = new FileSystem.Archivo(name, content, owner, false, true);
        file.SetPermisos(permissions);
        file.SetProtected(isProtected);
        fs.ForceAddFile(file, parent);
    }
}
