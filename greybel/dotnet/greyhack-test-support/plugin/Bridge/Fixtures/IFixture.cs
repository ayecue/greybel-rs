namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Tracks fixture state created for a test so the next player setup can reset it.</summary>
internal interface IFixture
{
    void Reset();
}
