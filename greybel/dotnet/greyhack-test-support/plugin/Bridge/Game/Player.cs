using System;
using System.Collections.Generic;
using System.Linq;
using GreyHackMessageHook;

namespace GreyHackTestSupport.Plugin.Bridge.Game;

/// <summary>Provides the test player's server, computer, and script user.</summary>
internal sealed class Player
{
    /// <summary>Script user: the fixture user, or root after a fixture performs a root login.</summary>
    public string User { get; set; } = PlayerFixture.TestUser;

    public PlayerServer Server =>
        PlayerServerEx.Current
        ?? throw new InvalidOperationException("No player server is connected");

    /// <summary>
    /// Gets the server-side player computer used by game scripts and message-hook contexts. The client copy
    /// (<c>PlayerClientMethods.pc</c>) contains only displayed state and lacks a new computer's file system. Because
    /// <c>PlayerServer.GetComputer</c> restores server state from the save on each call, modifications must be saved.
    /// </summary>
    public PlayerComputer Computer =>
        Server.GetComputer() ?? throw new InvalidOperationException("The player has no computer");

    /// <summary>Gets the computer at <paramref name="localIp"/> on the player's current network.</summary>
    public Computer GetLanComputer(string localIp)
    {
        Router playerRouter = Computer.GetMainRouter();
        Router router =
            ServerMap.Singleton.GetRouter(new IP(playerRouter.GetPublicIP()), false)
            ?? playerRouter;

        return router.GetComputer(localIp)
            ?? throw new InvalidOperationException("No computer at " + localIp);
    }

    /// <summary>
    /// Gets all instances of the router at <paramref name="localIp"/> on the player's network: the world instance and
    /// the player-computer instances used by scripts.
    /// </summary>
    public List<Router> GetRouterInstances(string localIp)
    {
        Computer player = Server.GetComputer();
        Router serverMain = player.GetMainRouter();
        List<Router> mains = new List<Router>
        {
            ServerMap.Singleton.GetRouter(new IP(serverMain.GetPublicIP()), false),
            serverMain,
            player.GetRouter()?.GetMainRouter(),
        };
        List<Router> routers = mains
            .Where(main => main != null)
            .Select(main => localIp == main.GetLocalIP() ? main : main.GetLanRouter(localIp))
            .Where(router => router != null)
            .Distinct()
            .ToList();

        return routers.Count > 0
            ? routers
            : throw new InvalidOperationException("Router not found: " + localIp);
    }
}
