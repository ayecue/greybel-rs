using System;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>
/// Controls world time, including stopping, setting, and advancing it while applying NPC alarm effects. Each player
/// setup restarts the clock at the normal game rate.
/// </summary>
internal sealed class ClockFixture(NpcFixture npcs) : IFixture
{
    /// <summary>Ratio of game world time to real time.</summary>
    private const int GameRate = 14;

    public void Reset()
    {
        Clock.multTiempo = GameRate;
    }

    /// <summary>Assigns <paramref name="date"/> to the world clock.</summary>
    public void SetTime(DateTime date)
    {
        GameField.ClockCurrentTime.SetValue(ClockServer.Singleton, date);
    }

    /// <summary>Freezes world time so time-dependent state remains stable during test comparisons.</summary>
    public Reply Stop()
    {
        Clock.multTiempo = 0;
        return new Reply.Applied();
    }

    public Reply Advance(AdvanceClockRpc request)
    {
        DateTime later = ClockServer.Singleton.GetDateTime().AddSeconds(request.Seconds);
        SetTime(later);
        npcs.UpdateLateness(later);
        return new Reply.Applied();
    }
}
