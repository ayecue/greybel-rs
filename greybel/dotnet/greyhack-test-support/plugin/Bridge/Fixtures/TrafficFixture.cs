using System;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Controls police-network traffic cameras and the traffic accident fixture.</summary>
internal sealed class TrafficFixture(Player player, ClockFixture clock) : IFixture
{
    /// <summary>Accident timestamp set by <see cref="RunAccident"/> and returned by network queries.</summary>
    public string AccidentAt { get; private set; }

    public void Reset()
    {
        AccidentAt = null;
    }

    /// <summary>
    /// Freezes a weekday one minute after a driver starts commuting to work, clears the camera cooldown, unlocks the
    /// network traffic override, and returns the driver's plate.
    /// </summary>
    public Reply Create()
    {
        Router router = player.Computer.GetMainRouter();
        NpcInfo driver = router
            .GetNetworkLan()
            .GetNpcInfos()
            .Values.First(npc =>
                !string.IsNullOrEmpty(npc.GetLicensePlate()) && !IsFixtureWorkstation(router, npc)
            );
        DateTime date = ClockServer.Singleton.GetDateTime().Date;
        if (date.DayOfWeek == DayOfWeek.Saturday)
        {
            date = date.AddDays(2);
        }
        else if (date.DayOfWeek == DayOfWeek.Sunday)
        {
            date = date.AddDays(1);
        }
        date = date.Add(driver.schedule.GetActivityTimeStart(Activity.ViajeTrabajo)).AddMinutes(1);
        clock.SetTime(date);
        clock.Stop();
        EndCooldown();
        router.UnlockOverrideSetting();
        return new Reply.TrafficReady(new TrafficReadyReply(driver.GetLicensePlate()));
    }

    /// <summary>
    /// Starts the client accident through <c>PlayerHelperServer.EnableCarAccidentServerRpc</c>. The operation requires
    /// prior use of the police network override.
    /// </summary>
    public Reply RunAccident(TrafficAccidentRpc request)
    {
        PlayerServer server = player.Server;
        AccidentAt = ClockServer.Singleton.GetFechaCompleta();
        server.playerHelper.SetNpcInfoCar(request.User, request.PublicIp, request.PoliceIp);
        server.playerHelper.EnableCarAccidentServerRpc();
        if (!player.Computer.GetMainRouter().GetConfigOS().IsOverrideUsed())
        {
            throw new InvalidOperationException(
                "The game didn't use the override for the accident"
            );
        }
        return new Reply.Applied();
    }

    /// <summary>Clears the camera cooldown and advances the clock two days beyond the accident cooldown.</summary>
    public Reply ResetCooldown()
    {
        EndCooldown();
        clock.SetTime(ClockServer.Singleton.GetDateTime().AddDays(2));
        return new Reply.Applied();
    }

    private static bool IsFixtureWorkstation(Router router, NpcInfo npc)
    {
        string localIp = npc.GetLocalIP();
        Computer computer = string.IsNullOrEmpty(localIp) ? null : router.GetComputer(localIp);
        return computer != null && computer.GetConfigOS().puertos.ExistePuertoExterno(10022);
    }

    private void EndCooldown()
    {
        PlayerServer server = player.Server;
        DateTime past = DateTime.Now.AddDays(-1);
        Database.Singleton.SyncTLCooldown(server.GetPlayerID(), past);
        GameField.TrafficCooldown.SetValue(server.playerHelper, past);
    }
}
