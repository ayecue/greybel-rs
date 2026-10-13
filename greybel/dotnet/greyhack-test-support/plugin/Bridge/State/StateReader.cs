using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Fixtures;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.State;

/// <summary>Provides test queries for the player's computer, files, and network.</summary>
internal sealed class StateReader(Player player, AccountFixture accounts, TrafficFixture traffic)
{
    public Reply ReadPlayer()
    {
        Computer computer = player.Computer;
        return new Reply.PlayerState(
            new PlayerStateReply(
                computer.GetID(),
                computer.GetPublicIP(),
                computer.GetLocalIP(),
                computer.GetUsers().Select(user => user.nombreUsuario).ToList(),
                (uint)computer.GetHardware().GetTotalMemoryRam(),
                (uint)computer.GetProcesos().Count
            )
        );
    }

    public Reply ReadFile(QueryFileRpc request)
    {
        FileSystem.Archivo file = player.Computer.GetFileSystem().GetArchivo(request.Path);
        return new Reply.FileState(
            new FileStateReply(
                request.Path,
                file != null,
                file?.GetContenido() ?? "",
                file?.IsBinario() ?? false
            )
        );
    }

    /// <summary>Gets current game network state, including fixture changes for this test.</summary>
    public Reply ReadNetwork()
    {
        return new Reply.NetworkState(
            new NetworkStateReply(
                NetworkSnapshotReader.Read(
                    player.Server.GetComputer(),
                    accounts.DebugUser,
                    accounts.DebugPassword,
                    accounts.CtfEventName,
                    traffic.AccidentAt
                )
            )
        );
    }
}
