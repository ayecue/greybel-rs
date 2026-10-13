using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Controls smart appliances on the player's network.</summary>
internal sealed class ApplianceFixture(Player player)
{
    /// <summary>Relocks override settings for the smart appliance at <c>request.LocalIp</c>.</summary>
    public Reply LockAppliance(LockApplianceRpc request)
    {
        Computer appliance = player.GetLanComputer(request.LocalIp);

        GameField.ApplianceOverrideSettings.SetValue(appliance.GetConfigOS(), false);
        Database.Singleton.SyncConfigOS(appliance);
        return new Reply.Applied();
    }
}
