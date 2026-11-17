using System;
using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Exercises ZeroDaySystem offers, mission generation, and library zero-day insertion.</summary>
internal sealed class ZeroDayFixture(Player player, ClockFixture clock, MissionFixture missions)
{
    public Reply Rumor(ZeroDayRumorRpc request)
    {
        EnsureRumorTime();
        ZeroDaySystem system = ZeroDaySystem.Singleton;
        (string user, string password) = system.GetNpcPassword(Guid.NewGuid().ToString());
        if (player.Computer.GetUserBank() == null)
        {
            Router bankRouter = ServerMap.Singleton.SpawnRouter(
                ServerMap.TipoRed.Bancos,
                NetworkLan.AccessType.NORMAL,
                null,
                false
            );
            player.Server.bankHelper.RegisterBankServerRpc(
                player.Computer.GetPlayerConfigOS().GetMailAccount().userName,
                "matrix1",
                bankRouter.GetPublicIP(),
                -1
            );
        }
        string bank = player.Computer.GetUserBank().userName;
        string posted = system.AddOfferCreds(Guid.NewGuid().ToString(), bank, 1000, user, password);
        if (posted.Length > 0)
        {
            return new Reply.ZeroDayRumor(new ZeroDayRumorReply("POSTED:" + posted, "", "", false));
        }
        if (!system.GetBestOfferCreds(out ZeroDaySystem.DebugToolsPlayerOffer best))
        {
            return new Reply.ZeroDayRumor(new ZeroDayRumorReply("NO OFFER", "", "", false));
        }
        string bestText = best == null ? "" : best.userName + ":" + best.price;
        string first = system.BuyOfferCreds(
            best.playerID,
            best.price,
            player.Server.GetPlayerID(),
            _ => "",
            out _
        );
        string second = system.AddOfferCreds(
            Guid.NewGuid().ToString(),
            bank,
            best.price,
            user,
            password
        );

        clock.SetTime(ClockServer.Singleton.GetDateTime().AddDays(120));
        bool expired = !system.GetBestOfferCreds(out _);

        return new Reply.ZeroDayRumor(new ZeroDayRumorReply(bestText, first, second, expired));
    }

    public Reply State(ZeroDayStateRpc request)
    {
        if (request.Fresh)
        {
            DateTime closed = new DateTime(ClockServer.Singleton.GetDateTime().Year, 1, 15);
            clock.SetTime(closed);
            ZeroDaySystem.Singleton.IsAvailableRumor(request.PlayerId);
        }
        if (request.Open)
        {
            EnsureRumorTime();
            ZeroDaySystem.Singleton.IsAvailableRumor(request.PlayerId, true);
        }

        string firstPublicIp = "";
        string firstLanIp = "";
        List<ZeroDayChatSnapshot> chats = new List<ZeroDayChatSnapshot>();
        if (request.FirstRumor)
        {
            Router first = ZeroDaySystem.Singleton.GenerateNetworkFirstRumor(out firstLanIp);
            firstPublicIp = first.GetPublicIP();
            chats = ReadRumorChats(first);
        }
        if (request.PlayerId.Length > 0)
        {
            ZeroDaySystem.Singleton.GetNpcPassword(request.PlayerId);
        }
        if (request.BindUser.Length > 0 && request.BindTo.Length > 0)
        {
            ZeroDaySystem.Singleton.AddCredentialsPlayerID(request.BindUser, request.BindTo);
        }

        return new Reply.ZeroDayState(
            ReadState(request.PlayerId, firstPublicIp, firstLanIp, chats)
        );
    }

    private static ZeroDayStateReply ReadState(
        string playerId,
        string firstPublicIp,
        string firstLanIp,
        List<ZeroDayChatSnapshot> chats
    )
    {
        ZeroDaySystem system = Database.Singleton.RestoreZeroDaySystem();
        JObject state = JObject.FromObject(system);
        List<ZeroDayNpcCredentialSnapshot> credentials = ((JObject)state["npcPasswords"]!)
            .Properties()
            .Select(entry =>
            {
                string owner = entry.Value["playerID"]?.Value<string>() ?? "";
                return new ZeroDayNpcCredentialSnapshot(
                    entry.Name,
                    entry.Value["password"]!.Value<string>()!,
                    owner.Length == 0 ? null : owner
                );
            })
            .ToList();
        List<ZeroDayRequestedCredentialSnapshot> requested = (
            (JObject)state["playerReqestedCreds"]!
        )
            .Properties()
            .Select(entry => new ZeroDayRequestedCredentialSnapshot(
                entry.Name,
                TupleText(entry.Value, "Item1", 0),
                TupleText(entry.Value, "Item2", 1)
            ))
            .ToList();
        List<ZeroDayOfferSnapshot> offers = ((JObject)state["debugToolsOffers"]!)
            .Properties()
            .Select(entry => new ZeroDayOfferSnapshot(
                entry.Value["playerID"]!.Value<string>()!,
                entry.Value["bankID"]!.Value<string>()!,
                entry.Value["price"]!.Value<int>(),
                entry.Value["userName"]!.Value<string>()!,
                entry.Value["password"]!.Value<string>()!,
                entry.Value["purchased"]!.Value<bool>()
            ))
            .ToList();
        List<ZeroDayPlayerSnapshot> players = new List<ZeroDayPlayerSnapshot>();
        if (playerId.Length > 0)
        {
            ZeroDayPlayerInfo info = Database.Singleton.GetZeroDayRequestPlayer(playerId);
            JObject saved = JObject.FromObject(info);
            List<ZeroDayNetworkBindingSnapshot> bindings = (
                saved["debugToolsCreds"] ?? new JArray()
            )
                .Select(binding => new ZeroDayNetworkBindingSnapshot(
                    binding["user"]!.Value<string>()!,
                    binding["password"]!.Value<string>()!,
                    binding["ipAddress"]!.Value<string>()!
                ))
                .ToList();
            players.Add(
                new ZeroDayPlayerSnapshot(
                    playerId,
                    RequestedAt(info),
                    (uint)info.GetDebugToolsLoginAttempts(),
                    (uint)info.GetSoloAttempts(),
                    info.IsMailSoloReceived(),
                    info.IsFailedUnitTesting(),
                    bindings
                )
            );
        }

        return new ZeroDayStateReply(
            credentials,
            requested,
            offers,
            players,
            state["ipAddressRumor"]!.Value<string>()!,
            firstPublicIp,
            firstLanIp,
            chats
        );
    }

    private static string TupleText(JToken token, string name, int index)
    {
        return token.Type == JTokenType.Array
            ? token[index]!.Value<string>()!
            : token[name]!.Value<string>()!;
    }

    private static ulong? RequestedAt(ZeroDayPlayerInfo info)
    {
        DateTime time = info.GetTimeRequest();
        if (time == default)
        {
            return null;
        }

        DateTime epoch = new DateTime(1970, 1, 1, 0, 0, 0, time.Kind);
        return (ulong)Math.Max(0, (long)(time - epoch).TotalSeconds);
    }

    private static List<ZeroDayChatSnapshot> ReadRumorChats(Router router)
    {
        List<ZeroDayChatSnapshot> result = new List<ZeroDayChatSnapshot>();
        foreach (LanPcInfo info in router.GetNetworkLan().GetLanPcInfos())
        {
            Computer computer = router.GetComputer(info.localIp);
            if (computer == null)
            {
                continue;
            }
            foreach (
                FileSystem.Archivo file in FileSystem.GetArchivos(
                    computer.GetFileSystem().GetCarpetaRaiz()
                )
            )
            {
                string content = file.GetContenido() ?? "";
                if (content.Contains("CONV_RUMOR_"))
                {
                    result.Add(
                        new ZeroDayChatSnapshot(
                            info.localIp,
                            file.GetNameOwner(),
                            file.GetRuta(),
                            content
                        )
                    );
                }
            }
        }
        return result;
    }

    public Reply CreateZeroDayChain(CreateZeroDayChainRpc request)
    {
        Reply.Mission reply = (Reply.Mission)missions.ReadMission(new MissionRpc(0, "ZERODAY", 0));
        InstallWeakAptClient(reply.Value.Group[0]);

        return reply;
    }

    public Reply CreateProceduralMission(CreateProceduralMissionRpc request)
    {
        return missions.ReadMission(new MissionRpc(0, "PROCEDURAL", 0));
    }

    public Reply Unlock(UnlockDebugLibraryRpc request)
    {
        Computer computer = ServerMap
            .Singleton.GetRouter(new IP(request.PublicIp))
            .GetComputer(request.LanIp);
        FileSystem.Archivo file = computer.GetFileSystem().GetArchivo(request.Path);
        if (file == null || !file.IsTypeFile(FileSystem.Fichero.TypeFile.Library))
        {
            UnityEngine.Debug.Log(
                $"ZERO unlock failed: router={request.PublicIp}, lan={request.LanIp}, computer={(computer == null ? "null" : computer.GetLocalIP())}, file={(file == null ? "null" : file.GetNombre())}"
            );
            return new Reply.ZeroDayUnlocked(false);
        }

        Library library = JsonConvert.DeserializeObject<Library>(file.GetContenido());
        Random random = new Random(Guid.NewGuid().GetHashCode());
        library.GenerateZeroDayVulnerability(random, out Library.MemoryZone zone);
        if (!library.GetMemoryZones().ContainsKey(zone.address))
        {
            library.GetMemoryZones().Add(zone.address, zone);
        }
        file.SetContenido(JsonConvert.SerializeObject(library), saveToDB: true);
        Database.Singleton.SyncFileSystemDB(computer, onlyFileSystem: true);

        return new Reply.ZeroDayUnlocked(true);
    }

    private void InstallWeakAptClient(string publicIp)
    {
        Router router = ServerMap.Singleton.GetRouter(new IP(publicIp));
        foreach (LanPcInfo info in router.GetNetworkLan().GetLanPcInfos())
        {
            Computer computer = router.GetComputer(info.localIp);
            if (computer == null)
            {
                continue;
            }

            Library latest = VersionsControl.Singleton.GetLastLibs()["aptclient.so"];
            Library library = JsonConvert.DeserializeObject<Library>(
                JsonConvert.SerializeObject(latest)
            );
            library.NewVersion();
            Library.MemoryZone zone = new Library.MemoryZone(
                new Random(1),
                "0xA11CE",
                library.GetIdLib(),
                Enum.GetValues(typeof(Library.Vulnerabilidad.Required))
                    .Cast<Library.Vulnerabilidad.Required>()
                    .ToList(),
                out Library.Vulnerabilidad weakness
            );
            weakness.unsecValue = "rvalues";
            weakness.requiredActions.Clear();
            weakness.helperHackResult.hackResult = Library.Vulnerabilidad.HackResult.SHELL;
            weakness.helperHackResult.user = "guest";
            library.GetMemoryZones().Clear();
            library.GetMemoryZones().Add(zone.address, zone);
            computer
                .GetFileSystem()
                .ForceAddFile(
                    new FileSystem.Archivo(
                        "aptclient.so",
                        JsonConvert.SerializeObject(library),
                        "root",
                        FileSystem.Fichero.TypeFile.Library,
                        saveToDB: true,
                        isDefaultContent: true
                    ),
                    "/lib"
                );
            Database.Singleton.SyncFileSystemDB(computer, onlyFileSystem: true);
            return;
        }
    }

    private void EnsureRumorTime()
    {
        if (ZeroDaySystem.IsRumorTime())
        {
            return;
        }

        DateTime next = ClockServer.Singleton.GetDateTime();
        next = new DateTime(next.Year, next.Month, 1).AddMonths(1);
        while (next.Month % 2 != 0)
        {
            next = next.AddMonths(1);
        }
        clock.SetTime(next);
    }
}
