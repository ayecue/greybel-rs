using System;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Controls smart appliances on the player's network.</summary>
internal sealed class ApplianceFixture(Player player, ClockFixture clock)
{
    /// <summary>Relocks override settings for the smart appliance at <c>request.LocalIp</c>.</summary>
    public Reply LockAppliance(LockApplianceRpc request)
    {
        Computer appliance = player.GetLanComputer(request.LocalIp);

        GameField.ApplianceOverrideSettings.SetValue(appliance.GetConfigOS(), false);
        Database.Singleton.SyncConfigOS(appliance);
        return new Reply.Applied();
    }

    /// <summary>Breaks an appliance or puts its next valid override inside the accident window.</summary>
    public Reply Configure(ConfigureApplianceRpc request)
    {
        SmartAppliance appliance = player.GetLanComputer(request.LocalIp) as SmartAppliance;
        if (appliance == null)
        {
            throw new InvalidOperationException(
                "The appliance fixture target is not a smart appliance"
            );
        }

        switch (request.Fixture)
        {
            case "broken":
                ((SmartAppConfigOS)appliance.GetConfigOS()).SetBroken(
                    ClockServer.Singleton.GetDateTime().AddMinutes(5)
                );
                break;
            case "timed_accident":
                NpcInfo npc = NpcFixture.EnsureNpc(
                    appliance.GetMainRouter().GetNetworkLan(),
                    appliance.GetMainRouter()
                );
                DateTime walk =
                    ClockServer.Singleton.GetDateTime().Date
                    + npc.schedule.GetTimeWalkToAppliance();
                clock.SetTime(walk.AddMinutes(-1));
                clock.Stop();
                break;
            default:
                throw new InvalidOperationException(
                    "Unknown appliance fixture: " + request.Fixture
                );
        }

        Database.Singleton.SyncConfigOS(appliance);
        return new Reply.Applied();
    }
}
