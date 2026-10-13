using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.State;
using GreyHackTestSupport.Protocol;
using MissionConfig;
using Newtonsoft.Json.Linq;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Draws procedural missions as the game does and reads the mission it builds and the networks it generates.</summary>
internal sealed class MissionFixture
{
    /// <summary>
    /// Draws the group of mission <paramref name="request"/>.Index through <c>IPGroupManager.GetRandomGroupForMission</c>,
    /// builds the mission with <c>MissionGen.GetProceduralMission</c> from the seed of the group, and generates the
    /// networks of the group with <c>ServerMap.GetRouter</c> as the first access to them does.
    /// </summary>
    public Reply ReadProceduralMission(ProceduralMissionRpc request)
    {
        TypeMission type = request.Police ? TypeMission.POLICE_PROCEDURAL : TypeMission.PROCEDURAL;
        ProceduralMissionSource source = MissionGen.GetProceduralSource(type);
        List<string> group = IPGroupManager.GetRandomGroupForMission(
            type.ToString(),
            (int)request.Index
        );
        System.Random random = new System.Random(MissionGen.Singleton.GetMissionSeed(group));
        List<Mission> parts = MissionGen.Singleton.GetProceduralMission(
            random,
            group.Count,
            source
        );
        List<MissionNetworkSnapshot> networks = [];

        // A clean baseline: the game copies retain their worlds between runs, so the group may contain computers an
        // earlier run saved. The game removes a network the same way for its rumor network
        // (ZeroDaySystem.IsAvailableRumor); ServerMap.GetRouter then generates it again.
        Database.Singleton.DeleteNetworks(group);

        for (int index = 0; index < parts.Count && index < group.Count; index++)
        {
            Router router = ServerMap.Singleton.GetRouter(new IP(group[index]));
            List<MissionComputerSnapshot> computers = [];

            // NetworkLan.GetComputers only returns computers the game has saved (LanPcInfo.saved), and a new network
            // has none. A player reaches a computer through Router.GetComputer, which generates an unsaved one with
            // its mission files (Router.GeneraLanPc, PostGenMission); the computers are read the same way and not
            // saved. One router instance per network: an unsaved router is generated again on every GetRouter.
            // The main router counts too: a mission can place its items on it. Every computer of the LAN is read,
            // also those behind LAN routers: Router.GetComputersIPs and NetworkLan.GetComputersIPs only list the
            // computers whose gateway is the main router, while the mission picks its computers from the entire
            // network (NetworkLan.GetLanPcInfos walks every device).
            List<Computer> network = [router];

            foreach (LanPcInfo info in router.GetNetworkLan().GetLanPcInfos())
            {
                Computer computer = router.GetComputer(info.localIp);

                if (computer != null)
                {
                    network.Add(computer);
                }
            }

            foreach (Computer computer in network)
            {
                List<FileSnapshot> files = [];

                NetworkSnapshotReader.ReadFolder(computer.GetFileSystem().GetCarpetaRaiz(), files);
                computers.Add(new MissionComputerSnapshot(computer.GetLocalIP(), files));
            }

            networks.Add(new MissionNetworkSnapshot(group[index], computers));
        }

        return new Reply.ProceduralMission(
            new ProceduralMissionReply(group, ReadMission(parts).ToString(0), networks)
        );
    }

    /// <summary>
    /// The texts of the name lists <c>WordGenerator</c> loads in its static constructor. Their order decides every
    /// seeded draw from them (<c>WordGenerator.GetNombre</c>, <c>MarkovNameGenerator.NextName</c>).
    /// </summary>
    public Reply ReadWordLists()
    {
        string[] names =
        [
            "male-names",
            "female-names",
            "Surnamesmod",
            "usernames",
            "CorpNames",
            "Passwords",
            "lib_variables",
            "exploitNames",
        ];
        List<WordListSnapshot> lists = [];

        foreach (string name in names)
        {
            UnityEngine.TextAsset asset = (UnityEngine.TextAsset)
                UnityEngine.Resources.Load("TextFiles/Names/" + name);

            lists.Add(new WordListSnapshot(name, asset.text));
        }

        return new Reply.WordLists(new WordListsReply(lists));
    }

    /// <summary>The mission as JSON: its common fields, its NPCs, and each part's computers with their items.</summary>
    private static JObject ReadMission(List<Mission> parts)
    {
        Mission first = parts[0];

        return new JObject
        {
            ["starterTextId"] = first.starterTextID,
            ["closureNetworkIndex"] = first.closureNetworkIndex,
            ["closureLocalIndex"] = first.closureLocalIndex,
            ["proceduralGlobalNpcIndex"] = first.proceduralGlobalNpcIndex,
            ["proceduralLevel"] = first.proceduralLevel,
            ["npcs"] = new JArray(first.preGenNpcs.Select(ReadNpc)),
            ["parts"] = new JArray(
                parts.Select(part => new JArray(part.computers.Select(ReadComputer)))
            ),
        };
    }

    private static JObject ReadNpc(PreGenNpc npc) =>
        new JObject
        {
            ["name"] = npc.name,
            ["surname"] = npc.surname,
            ["isForcedPass"] = npc.isForcedPass,
            ["forcedPass"] = npc.forcedPass,
        };

    private static JObject ReadComputer(ComputerMission computer) =>
        new JObject
        {
            ["computerIndexNet"] = computer.computerIndexNet,
            ["cloneNpcPcIdx"] = computer.cloneNpcPcIdx,
            ["archetype"] = computer.proceduralArchetype.HasValue
                ? (int)computer.proceduralArchetype.Value
                : null,
            ["items"] = new JArray(computer.GetMission().Select(ReadItem)),
        };

    private static JObject ReadItem(MissionItem item)
    {
        JObject json = new JObject
        {
            ["type"] = (int)item.typeItem,
            ["id"] = item.ID,
            ["isEvidence"] = item.isEvidence,
            ["tags"] = new JObject(
                item.runtimeTags.OrderBy(tag => tag.Key, System.StringComparer.Ordinal)
                    .Select(tag => new JProperty(tag.Key, tag.Value))
            ),
        };

        switch (item)
        {
            case ChatLogItem chat:
                json["forceNpcReceive"] = chat.forceNpcReceive;
                json["indexNpc"] = chat.indexNpc;
                json["randomPath"] = chat.randomPath;
                json["path"] = chat.path;
                break;
            case MailConversItem mail:
                json["forceNpcReceive"] = mail.forceNpcReceive;
                json["indexNpc"] = mail.indexNpc;
                json["useNetworkNpcSender"] = mail.useNetworkNpcSender;
                json["titleId"] = mail.titleID;
                break;
            case SystemLogItem log:
                json["action"] = (int)log.action;
                json["port"] = log.port;
                json["toComputerIdx"] = log.toComputerIdx;
                json["isReceivingConn"] = log.isReceivingConn;
                break;
            case FileItem file:
                json["rowShuffleSeed"] = file.rowShuffleSeed;
                json["fileName"] = file.fileName;
                json["randomPath"] = file.randomPath;
                json["isPdf"] = file.isPdf;
                json["isBinary"] = file.isBinary;
                json["isScript"] = file.isScript;
                json["allowImport"] = file.allowImport;
                json["isImage"] = file.isImage;
                json["path"] = file.path;
                json["symlinkTarget"] = file.symlinkTarget;
                json["owner"] = file.owner;
                json["permissions"] = file.permissions;
                json["directoryPermissions"] = file.directoryPermissions;
                json["encryptWithOwnerPassword"] = file.encryptWithOwnerPassword;
                json["encryptionPassword"] = file.encryptionPassword;
                json["rawContent"] = file.rawContent;
                json["companions"] = new JArray(
                    file.companionFiles.Select(companion => new JObject
                    {
                        ["fileName"] = companion.fileName,
                        ["content"] = companion.content,
                        ["textId"] = companion.textId,
                        ["permissions"] = companion.permissions,
                        ["encryptionPassword"] = companion.encryptionPassword,
                    })
                );
                break;
            case ServiceItem service:
                json["service"] = (int)service.servicio;
                json["allowDuplicates"] = service.allowDuplicates;
                break;
        }

        return json;
    }
}
