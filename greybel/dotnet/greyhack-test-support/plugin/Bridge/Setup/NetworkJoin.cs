using System;
using GreyHackTestSupport.Plugin.Bridge.Connection;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using GameProtocol = GreyHackMessageHook.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Setup;

/// <summary>Joins a test network and waits for the game's confirmation message.</summary>
internal sealed class NetworkJoin(MainThread main, Player player)
{
    /// <summary>Target network for <see cref="ConnectNetwork"/> while join confirmation is pending.</summary>
    private PendingConnection pendingConnection;

    /// <summary>
    /// Moves near the router, stabilizes hardware, and joins the Wi-Fi network at <c>request.PublicIp</c> through the
    /// player path (<c>IwconfigUIServerRpc</c>). Replies after the game confirms the join result.
    /// </summary>
    public void ConnectNetwork(Requester requester, ConnectNetworkRpc request)
    {
        main.Post(
            requester,
            () =>
            {
                try
                {
                    PlayerServer server = player.Server;
                    Router router =
                        ServerMap.Singleton.GetRouter(new IP(request.PublicIp), true)
                        ?? throw new InvalidOperationException(
                            "Unable to create router: " + request.PublicIp
                        );
                    var position = server.playerHelper.GetMapHelper().posNodoPlayer;
                    ServerMap.RouterNode node = Database.Singleton.GetRouterNode(request.PublicIp);
                    if (node != null)
                    {
                        node.routerPos = position;
                        Database.Singleton.SyncServerMap(node);
                    }
                    ServerMap.RouterNode origin = Database.Singleton.GetRouterNode(
                        server.GetComputer().GetConfigOS().GetOrigIpPublica()
                    );
                    if (origin != null)
                    {
                        origin.routerPos = position;
                        Database.Singleton.SyncServerMap(origin);
                    }
                    HardwareRepair.RestoreNetwork(router);
                    pendingConnection?.Requester.Fail(
                        new InvalidOperationException("Another network was joined meanwhile")
                    );
                    pendingConnection = new PendingConnection(
                        requester,
                        router.GetPublicIP(),
                        router.GetLocalIP()
                    );
                    server.networkHelper.IwconfigUIServerRpc(
                        router.GetBssid(),
                        router.GetEssid(),
                        router.GetPassword(),
                        request.PublicIp,
                        -1
                    );
                }
                catch
                {
                    pendingConnection = null;
                    throw;
                }
            }
        );
    }

    /// <summary>Returns the game's join result for the network targeted by <see cref="ConnectNetwork"/>.</summary>
    public void Handle(Duplex.Message message)
    {
        switch (message)
        {
            case GameProtocol.IwconfigClientRpc _ when pendingConnection != null:
                PendingConnection connection = pendingConnection;
                pendingConnection = null;
                connection.Requester.Reply(
                    new Reply.NetworkConnection(
                        new NetworkConnectionReply(
                            true,
                            connection.PublicIp,
                            connection.LocalIp,
                            ""
                        )
                    )
                );
                break;
            case GameProtocol.CloseConnectionClientRpc closeRpc when pendingConnection != null:
                Requester requester = pendingConnection.Requester;
                pendingConnection = null;
                requester.Reply(
                    new Reply.NetworkConnection(
                        new NetworkConnectionReply(false, "", "", closeRpc.Message)
                    )
                );
                break;
        }
    }

    private sealed class PendingConnection(Requester requester, string publicIp, string localIp)
    {
        public Requester Requester { get; } = requester;

        public string PublicIp { get; } = publicIp;

        public string LocalIp { get; } = localIp;
    }
}
